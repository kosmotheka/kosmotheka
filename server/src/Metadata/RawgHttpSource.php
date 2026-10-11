<?php

declare(strict_types=1);

namespace App\Metadata;

use Symfony\Component\DependencyInjection\Attribute\Autowire;
use Symfony\Contracts\HttpClient\HttpClientInterface;
use Symfony\Contracts\HttpClient\ResponseInterface;

/**
 * The RAWG API with the server's key (D62). **RAWG only takes the key as the `key` parameter of the address** (it has
 * no header for it), so here the key is in every outgoing address, and everything is arranged so that it goes nowhere
 * else: the client has no logger ({@see UnloggedClientFactory}), every failure is thrown as fixed text without `previous`
 * (what the http client throws carries the address), and nothing of RAWG's own answer is passed on. The text a person
 * searches for travels as one encoded parameter; the detail is asked by a number RAWG itself returned. Whatever RAWG
 * sends is read with a size cap and cut and type-checked field by field.
 */
final readonly class RawgHttpSource implements RawgSource
{
    private const string API = 'https://api.rawg.io/api';
    private const float TIMEOUT_SECONDS = 5.0;
    private const int MAX_BODY_BYTES = 512 * 1024;
    public const int MAX_HITS = 8;
    /** How many results are looked at to find those eight that are well formed. */
    private const int RESULTS_LOOKED_AT = 40;
    private const int MAX_TEXT = 300;
    private const int MAX_NAME = 200;
    private const int MAX_PLATFORM = 100;
    private const int MAX_PLATFORMS = 10;
    private const int MAX_PEOPLE = 5;
    private const int MAX_ID = 1_000_000_000;
    private const int MAX_URL = 300;
    /** The only host RAWG's images are on; anything else it names is dropped. */
    private const string COVER_HOST = 'media.rawg.io';

    public function __construct(
        private HttpClientInterface $httpClient,
        #[Autowire(env: 'RAWG_API_KEY')]
        private string $apiKey,
    ) {
    }

    public function isConfigured(): bool
    {
        return '' !== trim($this->apiKey);
    }

    public function search(string $query): array
    {
        $response = $this->start(self::API.'/games', ['search' => $query, 'page_size' => self::MAX_HITS]);
        $decoded = json_decode($this->read($response), true, 16);
        $results = \is_array($decoded) ? ($decoded['results'] ?? null) : null;
        if (!\is_array($results)) {
            throw MetadataProviderException::failed();
        }

        $hits = [];
        foreach (\array_slice($results, 0, self::RESULTS_LOOKED_AT) as $result) {
            $hit = \is_array($result) ? $this->hit($result) : null;
            if ($hit instanceof RawgHit) {
                $hits[] = $hit;
            }
            if (\count($hits) >= self::MAX_HITS) {
                break;
            }
        }

        return $hits;
    }

    public function details(array $ids, \Closure $mayCall, \Closure $onQuota): array
    {
        // All started before any is read, so the time spent is that of the slowest and not the sum.
        $started = [];
        foreach ($ids as $id) {
            if (!$mayCall()) {
                break;
            }
            try {
                $started[$id] = $this->begin(\sprintf('%s/games/%d', self::API, $id), []);
            } catch (\Throwable) {
                // On purpose without `previous`: what the http client throws carries the address, and the key.
            }
        }

        $details = [];
        foreach ($started as $id => $response) {
            try {
                $details[$id] = $this->detail($this->read($response));
            } catch (MetadataProviderException $exception) {
                if ($exception->quota) {
                    $onQuota();
                }
            } catch (\Throwable) {
                // A detail that cannot be had is a candidate without it.
            }
        }

        return $details;
    }

    /** @param array<string, scalar> $query */
    private function start(string $url, array $query): ResponseInterface
    {
        try {
            return $this->begin($url, $query);
        } catch (\Throwable) {
            throw MetadataProviderException::failed();
        }
    }

    /** @param array<string, scalar> $query */
    private function begin(string $url, array $query): ResponseInterface
    {
        return $this->httpClient->request('GET', $url, [
            // The key last, and only here: RAWG has no other way to take it.
            'query' => $query + ['key' => trim($this->apiKey)],
            'headers' => ['Accept' => 'application/json'],
            'timeout' => self::TIMEOUT_SECONDS,
            'max_duration' => self::TIMEOUT_SECONDS,
            'max_redirects' => 0,
        ]);
    }

    /** @throws MetadataProviderException */
    private function read(ResponseInterface $response): string
    {
        try {
            $status = $response->getStatusCode();
            if (429 === $status || 401 === $status || 403 === $status) {
                $response->cancel();

                throw MetadataProviderException::quota();
            }
            if ($status >= 500) {
                $response->cancel();

                throw MetadataProviderException::brokeItself();
            }
            if (200 !== $status) {
                $response->cancel();

                throw MetadataProviderException::failed();
            }

            $body = '';
            foreach ($this->httpClient->stream($response) as $chunk) {
                $body .= $chunk->getContent();
                if (\strlen($body) > self::MAX_BODY_BYTES) {
                    $response->cancel();

                    throw MetadataProviderException::failed();
                }
            }

            return $body;
        } catch (MetadataProviderException $exception) {
            throw $exception;
        } catch (\Throwable) {
            throw MetadataProviderException::failed();
        }
    }

    /** @param array<array-key, mixed> $result */
    private function hit(array $result): ?RawgHit
    {
        $id = $result['id'] ?? null;
        if (!\is_int($id) || $id < 1 || $id > self::MAX_ID) {
            return null;
        }
        $title = $this->clean($result['name'] ?? null, self::MAX_TEXT);
        if (null === $title) {
            return null;
        }

        $released = $result['released'] ?? null;

        return new RawgHit(
            $id,
            $title,
            \is_string($released) && 1 === preg_match('/^(\d{4})/', $released, $year) && (int) $year[1] >= 1950 && (int) $year[1] <= 2200 ? (int) $year[1] : null,
            $this->cover($result['background_image'] ?? null),
            $this->platforms($result['platforms'] ?? null),
        );
    }

    /** @return list<string> */
    private function platforms(mixed $platforms): array
    {
        $names = [];
        foreach (\is_array($platforms) ? \array_slice($platforms, 0, 200) : [] as $entry) {
            $platform = \is_array($entry) ? ($entry['platform'] ?? null) : null;
            $name = \is_array($platform) ? $this->clean($platform['name'] ?? null, self::MAX_PLATFORM) : null;
            if (null !== $name && !\in_array($name, $names, true)) {
                $names[] = $name;
            }
            if (\count($names) >= self::MAX_PLATFORMS) {
                break;
            }
        }

        return $names;
    }

    private function detail(string $body): RawgDetail
    {
        $decoded = json_decode($body, true, 16);
        if (!\is_array($decoded)) {
            throw MetadataProviderException::failed();
        }

        return new RawgDetail($this->names($decoded['developers'] ?? null), $this->names($decoded['publishers'] ?? null));
    }

    /**
     * The names in a list of companies RAWG sent.
     *
     * @return list<string>
     */
    private function names(mixed $companies): array
    {
        $names = [];
        foreach (\is_array($companies) ? \array_slice($companies, 0, 200) : [] as $company) {
            $name = \is_array($company) ? $this->clean($company['name'] ?? null, self::MAX_NAME) : null;
            if (null !== $name && !\in_array($name, $names, true)) {
                $names[] = $name;
            }
            if (\count($names) >= self::MAX_PEOPLE) {
                break;
            }
        }

        return $names;
    }

    /**
     * Text from RAWG, cleaned: and gone if it holds our key, which no text of a game ever should (a RAWG that echoed
     * the address of a call back in a field must not make the key travel on to the app and the cache).
     */
    private function clean(mixed $value, int $max): ?string
    {
        $text = TextCleaner::clean($value, $max);
        $key = trim($this->apiKey);

        return null !== $text && '' !== $key && str_contains($text, $key) ? null : $text;
    }

    /** `https://media.rawg.io/media/…` ending in an image extension, with nothing in it that is not a plain name. */
    private function cover(mixed $value): ?string
    {
        if (!\is_string($value) || \strlen($value) > self::MAX_URL
            || 1 !== preg_match('#^https://'.preg_quote(self::COVER_HOST, '#').'/media/(?:[A-Za-z0-9_-]+/)*[A-Za-z0-9_-]+\.(?:jpg|jpeg|png|webp)$#D', $value)
        ) {
            return null;
        }

        return $value;
    }
}
