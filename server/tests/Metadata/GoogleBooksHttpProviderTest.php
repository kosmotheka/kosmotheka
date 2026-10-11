<?php

declare(strict_types=1);

namespace App\Tests\Metadata;

use App\Metadata\GoogleBooksHttpProvider;
use App\Metadata\Isbn;
use App\Metadata\MetadataProviderException;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;
use Symfony\Component\HttpClient\Exception\TransportException;
use Symfony\Component\HttpClient\MockHttpClient;
use Symfony\Component\HttpClient\Response\MockResponse;

/** What Google may send, and what the server lets through (7.8). */
final class GoogleBooksHttpProviderTest extends TestCase
{
    private const string KEY = 'SENTINEL-key-0001';

    /** @param array<string, mixed> $info */
    private function volumeInfoJson(array $info, string $id = 'v1'): string
    {
        return json_encode(['items' => [['id' => $id, 'volumeInfo' => $info]]], \JSON_THROW_ON_ERROR);
    }

    private function provider(MockResponse|\Closure $answer, string $key = self::KEY): GoogleBooksHttpProvider
    {
        if ($answer instanceof MockResponse) {
            $answer = static fn (): MockResponse => $answer;
        }

        return new GoogleBooksHttpProvider(new MockHttpClient($answer), $key);
    }

    private function isbn(): Isbn
    {
        return Isbn::parse('9788408176039');
    }

    public function testItIsConfiguredOnlyWithAKey(): void
    {
        self::assertTrue($this->provider(new MockResponse('{}'))->isConfigured());
        self::assertFalse($this->provider(new MockResponse('{}'), '')->isConfigured());
        self::assertFalse($this->provider(new MockResponse('{}'), '   ')->isConfigured());
    }

    /** @return iterable<string, array{string, ?string}> */
    public static function covers(): iterable
    {
        yield 'https on books.google.com' => ['https://books.google.com/books/content?id=1&zoom=1', 'https://books.google.com/books/content?id=1&zoom=1'];
        yield 'http is raised to https' => ['http://books.google.com/books/content?id=1&edge=curl', 'https://books.google.com/books/content?id=1'];
        yield 'another site' => ['https://evil.example/cover.jpg', null];
        yield 'a look-alike host' => ['https://books.google.com.evil.example/c.jpg', null];
        yield 'a user in the address' => ['https://books.google.com@evil.example/c.jpg', null];
        yield 'a port' => ['https://books.google.com:8443/c.jpg', null];
        yield 'ftp' => ['ftp://books.google.com/c.jpg', null];
        yield 'javascript' => ['javascript:alert(1)', null];
        yield 'data' => ['data:image/png;base64,AAAA', null];
        yield 'not a url' => ['cover', null];
        yield 'very long' => ['https://books.google.com/'.str_repeat('a', 5000), null];
        yield 'a line break' => ["https://books.google.com/a\nb", null];
    }

    #[DataProvider('covers')]
    public function testOnlyACoverFromGoogleBooksSurvives(string $given, ?string $expected): void
    {
        $provider = $this->provider(new MockResponse($this->volumeInfoJson(['title' => 'T', 'imageLinks' => ['thumbnail' => $given]])));

        self::assertSame($expected, $provider->lookup($this->isbn())?->coverUrl);
    }

    public function testAVolumeWithNeitherTitleNorAuthorsIsNotAFind(): void
    {
        $provider = $this->provider(new MockResponse($this->volumeInfoJson(['publisher' => 'Planeta', 'title' => ['not', 'a string'], 'authors' => 'nobody'])));

        self::assertNull($provider->lookup($this->isbn()));
    }

    public function testFieldsAreCutAndTheirTypesChecked(): void
    {
        $found = $this->provider(new MockResponse($this->volumeInfoJson([
            'title' => str_repeat('é', 5000),
            'authors' => [...array_fill(0, 500, 'A'), ['x'], 3],
            'publishedDate' => '1999',
            'language' => 'es',
        ], str_repeat('i', 500))))->lookup($this->isbn());

        self::assertNotNull($found);
        self::assertSame(300, mb_strlen((string) $found->title));
        self::assertCount(20, $found->authors);
        self::assertSame(1999, $found->publishedYear);
        self::assertSame('es', $found->language);
        self::assertNull($found->externalId);
    }

    public function testTheKeyGoesInAHeaderAndNeverInTheAddress(): void
    {
        $seenUrl = '';
        $seenHeaders = null;
        $provider = $this->provider(static function (string $method, string $url, array $options) use (&$seenUrl, &$seenHeaders): MockResponse {
            $seenUrl = $url;
            $normalized = $options['normalized_headers'] ?? [];
            $seenHeaders = \is_array($normalized) ? ($normalized['x-goog-api-key'] ?? null) : null;

            return new MockResponse('{}');
        });

        $provider->lookup($this->isbn());

        self::assertStringNotContainsString(self::KEY, $seenUrl);
        self::assertSame(['X-Goog-Api-Key: '.self::KEY], $seenHeaders);
    }

    /** @return iterable<string, array{MockResponse|\Closure, bool}> */
    public static function failures(): iterable
    {
        yield '429' => [new MockResponse('{}', ['http_code' => 429]), true];
        yield '403' => [new MockResponse('{"k":"'.self::KEY.'"}', ['http_code' => 403]), true];
        yield '500' => [new MockResponse('boom '.self::KEY, ['http_code' => 500]), false];
        yield '404' => [new MockResponse('{}', ['http_code' => 404]), false];
        yield 'not json' => [new MockResponse('<html>'), false];
        yield 'json that is not an object' => [new MockResponse('"text"'), false];
        yield 'too big' => [new MockResponse('{"items":[],"pad":"'.str_repeat('x', 2_000_000).'"}'), false];
        yield 'a timeout carrying the key' => [static fn (): never => throw new TransportException('timeout for https://x/?key='.self::KEY), false];
    }

    #[DataProvider('failures')]
    public function testFailuresSayOnlyWhetherItWasTheQuotaAndNeverCarryTheKey(MockResponse|\Closure $answer, bool $quota): void
    {
        try {
            $this->provider($answer)->lookup($this->isbn());
            self::fail('It should have failed.');
        } catch (MetadataProviderException $exception) {
            self::assertSame($quota, $exception->quota);
            self::assertStringNotContainsString(self::KEY, $exception->getMessage());
            self::assertNotInstanceOf(\Throwable::class, $exception->getPrevious());
        }
    }

    public function testNoItemsMeansNotFound(): void
    {
        self::assertNull($this->provider(new MockResponse('{"kind":"books#volumes","totalItems":0}'))->lookup($this->isbn()));
    }
}
