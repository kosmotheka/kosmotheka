<?php

declare(strict_types=1);

namespace App\Metadata;

use App\Enum\BggScope;
use Symfony\Component\DependencyInjection\Attribute\Autowire;
use Symfony\Contracts\HttpClient\HttpClientInterface;
use Symfony\Contracts\HttpClient\ResponseInterface;

/**
 * The XML API 2 of BoardGameGeek and RPGGeek with the server's token (D62). Addresses are fixed; the only things that
 * change them are the scope (a closed list) and, for the detail, numbers BoardGameGeek itself returned. The text a
 * person searches for travels as one encoded parameter, and the token in a header, never in an address.
 *
 * The answer is XML from outside, so before it is parsed anything that could make the parser do more than read is
 * refused (a document type, an entity, another encoding), and what is read is size-capped and checked field by field.
 * Its terms forbid changing its data: a value is taken whole through {@see TextCleaner::whole()} or dropped.
 */
final readonly class BggHttpSource implements BggSource
{
    private const float TIMEOUT_SECONDS = 6.0;
    /** A detail of eight games can name hundreds of publishers each. */
    private const int MAX_BODY_BYTES = 2 * 1024 * 1024;
    public const int MAX_HITS = 8;
    /** How many results are looked at to choose the best eight: the real search for a common word gave 286. */
    private const int RESULTS_LOOKED_AT = 400;
    public const int MAX_TEXT = 300;
    public const int MAX_PEOPLE = 5;
    public const int MAX_MINUTES = 9999;
    /** How many `link` lines of one game are looked at: more than a real one has. */
    private const int LINKS_LOOKED_AT = 3000;
    private const int MAX_ID = 1_000_000_000;
    private const string COVER_PATTERN = '#^https://cf\.geekdo-images\.com/[A-Za-z0-9_\-./=:()]{1,300}\.(jpg|jpeg|png|webp|gif)$#D';

    public function __construct(
        private HttpClientInterface $httpClient,
        #[Autowire(env: 'BGG_API_TOKEN')]
        private string $apiToken,
    ) {
    }

    public function isConfigured(): bool
    {
        return '' !== trim($this->apiToken);
    }

    public function search(BggScope $scope, string $query): array
    {
        $response = $this->start($scope->api().'/search', ['query' => $query, 'type' => implode(',', $scope->itemTypes())]);
        $xml = $this->parse($this->read($response));

        // BoardGameGeek answers with everything that holds the words, in no order of relevance (286 results for
        // "catan", the game itself fifth), so the eight are chosen here: what is called exactly that, what starts with
        // it, what holds it; a game before an expansion in each group, then the shorter title, then its own order.
        $ranked = [];
        $seen = 0;
        foreach ($xml->item as $item) {
            if (++$seen > self::RESULTS_LOOKED_AT) {
                break;
            }
            $hit = $this->hit($scope, $item);
            if (!$hit instanceof BggHit || isset($ranked[$hit->id])) {
                continue;
            }
            $title = TextCleaner::normaliseQuery($hit->title);
            $score = match (true) {
                $title === $query => 0,
                str_starts_with($title, $query) => 1,
                str_contains($title, $query) => 2,
                default => 3,
            };
            $ranked[$hit->id] = [$score, 'boardgameexpansion' === (string) $item['type'] ? 1 : 0, mb_strlen($title), $seen, $hit];
        }
        usort($ranked, static fn (array $a, array $b): int => \array_slice($a, 0, 4) <=> \array_slice($b, 0, 4));

        return array_map(static fn (array $row): BggHit => $row[4], \array_slice($ranked, 0, self::MAX_HITS));
    }

    public function details(BggScope $scope, array $ids, \Closure $mayCall, \Closure $onQuota): array
    {
        if ([] === $ids || !$mayCall()) {
            return [];
        }

        try {
            $response = $this->begin($scope->api().'/thing', ['id' => implode(',', $ids), 'type' => implode(',', $scope->itemTypes())]);
            $xml = $this->parse($this->read($response));
        } catch (MetadataProviderException $exception) {
            if ($exception->quota) {
                $onQuota();
            }

            // Whatever it was, those details are not had now; only "it answered it has none" is worth remembering.
            return $exception->answeredUnusable ? array_fill_keys($ids, null) : [];
        } catch (\Throwable) {
            // On purpose without `previous`: what the http client throws can carry the token.
            return [];
        }

        $details = array_fill_keys($ids, null);
        foreach ($xml->item as $item) {
            $id = $this->number($item['id'], self::MAX_ID);
            if (null !== $id && \array_key_exists($id, $details) && \in_array((string) $item['type'], $scope->itemTypes(), true)) {
                $details[$id] = $this->detail($scope, $item);
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
            'query' => $query,
            'headers' => ['Authorization' => 'Bearer '.trim($this->apiToken), 'Accept' => 'application/xml'],
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
            // 202 is "queued, ask again in a moment": the same as a provider that broke, which is tried again.
            if (202 === $status || $status >= 500) {
                $response->cancel();

                throw MetadataProviderException::brokeItself();
            }
            if (200 !== $status) {
                $response->cancel();

                throw MetadataProviderException::unusable();
            }

            $body = '';
            foreach ($this->httpClient->stream($response) as $chunk) {
                $body .= $chunk->getContent();
                if (\strlen($body) > self::MAX_BODY_BYTES) {
                    $response->cancel();

                    throw MetadataProviderException::unusable();
                }
            }

            return $body;
        } catch (MetadataProviderException $exception) {
            throw $exception;
        } catch (\Throwable) {
            throw MetadataProviderException::failed();
        }
    }

    /**
     * Reads the XML only if it is plain: no document type or entity (where external entities and expansion bombs
     * live), no NUL byte, nothing but UTF-8 (a document in another encoding could hide a document type from the
     * check). The parser is also told never to touch the network.
     *
     * @throws MetadataProviderException
     */
    private function parse(string $body): \SimpleXMLElement
    {
        if (str_contains($body, "\0") || !mb_check_encoding($body, 'UTF-8') || 1 === preg_match('/<!\s*(DOCTYPE|ENTITY)/i', $body)
            || 1 === preg_match('/^\s*<\?xml[^>]*encoding\s*=\s*["\'](?!utf-8["\'])/i', $body)) {
            throw MetadataProviderException::unusable();
        }

        $previous = libxml_use_internal_errors(true);
        try {
            $xml = simplexml_load_string($body, \SimpleXMLElement::class, \LIBXML_NONET | \LIBXML_NOCDATA);
        } finally {
            libxml_clear_errors();
            libxml_use_internal_errors($previous);
        }
        if (!$xml instanceof \SimpleXMLElement || 'items' !== $xml->getName()) {
            throw MetadataProviderException::unusable();
        }

        return $xml;
    }

    private function hit(BggScope $scope, \SimpleXMLElement $item): ?BggHit
    {
        $id = $this->number($item['id'], self::MAX_ID);
        $title = TextCleaner::whole((string) $item->name['value'], self::MAX_TEXT);
        if (null === $id || null === $title || !\in_array((string) $item['type'], $scope->itemTypes(), true)) {
            return null;
        }

        return new BggHit($id, $title, $this->year($item->yearpublished['value'] ?? null));
    }

    private function detail(BggScope $scope, \SimpleXMLElement $item): BggDetail
    {
        $title = null;
        foreach ($item->name as $name) {
            if ('primary' === (string) $name['type']) {
                $title = TextCleaner::whole((string) $name['value'], self::MAX_TEXT);
                break;
            }
        }

        $designers = [];
        $artists = [];
        $publisher = null;
        $expandsId = null;
        $expandsTitle = null;
        $looked = 0;
        foreach ($item->link as $link) {
            if (++$looked > self::LINKS_LOOKED_AT) {
                break;
            }
            $type = (string) $link['type'];
            $value = TextCleaner::whole((string) $link['value'], self::MAX_TEXT);
            if (null === $value) {
                continue;
            }
            if ($type === $scope->designerLink() && \count($designers) < self::MAX_PEOPLE && !\in_array($value, $designers, true)) {
                $designers[] = $value;
            } elseif ($type === $scope->artistLink() && \count($artists) < self::MAX_PEOPLE && !\in_array($value, $artists, true)) {
                $artists[] = $value;
            } elseif ($type === $scope->publisherLink() && null === $publisher) {
                $publisher = $value;
            } elseif (BggScope::Boardgame === $scope && 'boardgameexpansion' === $type && 'true' === (string) $link['inbound'] && null === $expandsId) {
                // Inbound: this game is the expansion, and the link names the game it expands.
                $id = $this->number($link['id'], self::MAX_ID);
                if (null !== $id) {
                    $expandsId = $id;
                    $expandsTitle = $value;
                }
            }
        }

        $isGame = BggScope::Boardgame === $scope;

        return new BggDetail(
            $title,
            self::coverAddress((string) $item->image),
            $publisher,
            $designers,
            $artists,
            $isGame ? $this->number($item->minplayers['value'] ?? null, 99) : null,
            $isGame ? $this->number($item->maxplayers['value'] ?? null, 99) : null,
            $isGame ? $this->number($item->playingtime['value'] ?? null, self::MAX_MINUTES) : null,
            $isGame ? $this->number($item->minage['value'] ?? null, 99) : null,
            $expandsId,
            $expandsTitle,
        );
    }

    /** A whole number from an attribute, 1 to `$max`: BoardGameGeek writes 0 for "unknown", which is nothing here. */
    private function number(mixed $value, int $max): ?int
    {
        $text = $value instanceof \SimpleXMLElement || \is_string($value) ? trim((string) $value) : '';
        if (1 !== preg_match('/^\d{1,9}$/D', $text)) {
            return null;
        }
        $number = (int) $text;

        return $number >= 1 && $number <= $max ? $number : null;
    }

    private function year(mixed $value): ?int
    {
        return $this->number($value, 2200);
    }

    /** A cover address as BoardGameGeek's image host makes them, and nothing else: what is read back from the cache passes it too (SEG-057). */
    public static function coverAddress(mixed $url): ?string
    {
        $url = \is_string($url) ? trim($url) : '';

        // A `//` after the scheme would be another address written inside the path.
        return !str_contains($url, '..') && !str_contains(substr($url, 8), '//') && 1 === preg_match(self::COVER_PATTERN, $url) ? $url : null;
    }
}
