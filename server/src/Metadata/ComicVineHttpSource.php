<?php

declare(strict_types=1);

namespace App\Metadata;

use App\Enum\ComicVineResource;
use Symfony\Component\DependencyInjection\Attribute\Autowire;
use Symfony\Contracts\HttpClient\HttpClientInterface;
use Symfony\Contracts\HttpClient\ResponseInterface;

/**
 * The Comic Vine API with the server's key (D62). **Comic Vine only takes the key as the `api_key` parameter of the
 * address**, so here the key is in every outgoing address, and everything is arranged so that it goes nowhere else,
 * as for RAWG: the client has no logger ({@see UnloggedClientFactory}), every failure is thrown as fixed text without
 * `previous`, nothing of its own answer is passed on, and a text that holds the key is dropped. The text a person
 * searches for travels as one encoded parameter; a detail is asked by a number Comic Vine itself returned. Whatever it
 * sends is read with a size cap and cut and type-checked field by field.
 *
 * What was read of its documentation (comicvine.gamespot.com/api/documentation): `search` takes `api_key`, `format`,
 * `query`, `resources` (`volume`, `issue`…), `limit` (at most 10 for the search) and `field_list`, and answers
 * `status_code` (1 is fine, 100 an invalid key), `error`, `results`, each result with a `resource_type`; `volume` has
 * `name`, `start_year`, `publisher`, `image`, `person_credits`; `issue` has `name`, `issue_number`, `cover_date`,
 * `store_date`, `volume`, `image`, `person_credits`. What the documentation does not say, and is assumed (from how the
 * API is known to answer; there is no key to see it with): that the detail of a series is `volume/4050-<id>/` and of an
 * issue `issue/4000-<id>/`, that `image` holds `small_url` and `medium_url` under `comicvine.gamespot.com/a/uploads/`,
 * that `person_credits` are `{name, role}`, and that it refuses a request that overruns its limit with `420` or `429`.
 */
final readonly class ComicVineHttpSource implements ComicVineSource
{
    private const string API = 'https://comicvine.gamespot.com/api';
    private const string USER_AGENT = 'Kosmotheka (+https://kosmotheka.app)';
    private const float TIMEOUT_SECONDS = 5.0;
    private const int MAX_BODY_BYTES = 512 * 1024;
    public const int MAX_HITS = 8;
    /** How many results are looked at to find those eight that are well formed. */
    private const int RESULTS_LOOKED_AT = 40;
    private const int MAX_TEXT = 300;
    private const int MAX_NAME = 200;
    private const int MAX_NUMBER = 20;
    private const int MAX_PEOPLE = 5;
    private const int MAX_ID = 1_000_000_000;
    private const int MAX_URL = 300;
    private const string SEARCH_FIELDS = 'id,name,resource_type,start_year,publisher,image,issue_number,cover_date,store_date,volume';
    /** Comic Vine tells a resource from another in a detail address by a number before the id. */
    private const array DETAIL_PREFIX = ['volume' => '4050', 'issue' => '4000'];
    /** What people did, as Comic Vine writes the roles of a credit, grouped by what the app can use. */
    private const array WRITER_ROLES = ['writer'];
    private const array ARTIST_ROLES = ['penciler', 'penciller', 'inker', 'artist', 'painter', 'illustrator'];

    public function __construct(
        private HttpClientInterface $httpClient,
        #[Autowire(env: 'COMIC_VINE_API_KEY')]
        private string $apiKey,
    ) {
    }

    public function isConfigured(): bool
    {
        return '' !== trim($this->apiKey);
    }

    public function search(string $query): array
    {
        $response = $this->start(self::API.'/search/', ['query' => $query, 'resources' => 'volume,issue', 'limit' => self::MAX_HITS, 'field_list' => self::SEARCH_FIELDS]);
        $results = $this->decode($this->read($response))['results'] ?? null;
        if (!\is_array($results)) {
            throw MetadataProviderException::failed();
        }

        // The series first, then the issues, each in the order Comic Vine gave them.
        $volumes = [];
        $issues = [];
        foreach (\array_slice($results, 0, self::RESULTS_LOOKED_AT) as $result) {
            $hit = \is_array($result) ? $this->hit($result) : null;
            if (!$hit instanceof ComicVineHit) {
                continue;
            }
            if (ComicVineResource::Volume === $hit->resource) {
                $volumes[] = $hit;
            } else {
                $issues[] = $hit;
            }
        }

        return \array_slice([...$volumes, ...$issues], 0, self::MAX_HITS);
    }

    public function details(array $hits, \Closure $mayCall, \Closure $onQuota): array
    {
        // All started before any is read, so the time spent is that of the slowest and not the sum.
        $started = [];
        foreach ($hits as $hit) {
            if (!$mayCall($hit->resource)) {
                continue;
            }
            try {
                $address = \sprintf('%s/%s/%s-%d/', self::API, $hit->resource->value, ComicVineResource::Volume === $hit->resource ? self::DETAIL_PREFIX['volume'] : self::DETAIL_PREFIX['issue'], $hit->id);
                $started[$hit->resource->value.':'.$hit->id] = $this->begin($address, ['field_list' => 'person_credits']);
            } catch (\Throwable) {
                // On purpose without `previous`: what the http client throws carries the address, and the key.
            }
        }

        $details = [];
        foreach ($started as $key => $response) {
            try {
                $details[$key] = $this->detail($this->read($response));
            } catch (MetadataProviderException $exception) {
                if ($exception->quota) {
                    $onQuota();
                }
                if ($exception->answeredUnusable) {
                    $details[$key] = null;
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
            // The key last, and only here: Comic Vine has no other way to take it.
            'query' => $query + ['format' => 'json', 'api_key' => trim($this->apiKey)],
            'headers' => ['Accept' => 'application/json', 'User-Agent' => self::USER_AGENT],
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
            if (\in_array($status, [401, 403, 420, 429], true)) {
                $response->cancel();

                throw MetadataProviderException::quota();
            }
            if ($status >= 500) {
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
     * The answer as an array, if its own status code says it went well: 100 is a key it does not accept, which is a
     * quota as far as anybody can do anything (and it comes even with a success status).
     *
     * @return array<array-key, mixed>
     *
     * @throws MetadataProviderException
     */
    private function decode(string $body): array
    {
        $decoded = json_decode($body, true, 16);
        if (!\is_array($decoded)) {
            throw MetadataProviderException::unusable();
        }
        $code = $decoded['status_code'] ?? null;
        if (100 === $code) {
            throw MetadataProviderException::quota();
        }
        if (1 !== $code) {
            throw MetadataProviderException::unusable();
        }

        return $decoded;
    }

    /** @param array<array-key, mixed> $result */
    private function hit(array $result): ?ComicVineHit
    {
        $type = \is_string($result['resource_type'] ?? null) ? ComicVineResource::tryFrom($result['resource_type']) : null;
        $id = $result['id'] ?? null;
        if (!\in_array($type, [ComicVineResource::Volume, ComicVineResource::Issue], true) || !\is_int($id) || $id < 1 || $id > self::MAX_ID) {
            return null;
        }

        if (ComicVineResource::Volume === $type) {
            $publisher = $result['publisher'] ?? null;

            return new ComicVineHit($type, $id, $this->clean($result['name'] ?? null, self::MAX_TEXT), null, null, $this->year($result['start_year'] ?? null), \is_array($publisher) ? $this->clean($publisher['name'] ?? null, self::MAX_NAME) : null, $this->cover($result['image'] ?? null));
        }

        $volume = $result['volume'] ?? null;

        return new ComicVineHit(
            $type,
            $id,
            $this->clean($result['name'] ?? null, self::MAX_TEXT),
            \is_array($volume) ? $this->clean($volume['name'] ?? null, self::MAX_TEXT) : null,
            $this->clean($result['issue_number'] ?? null, self::MAX_NUMBER),
            $this->year($result['cover_date'] ?? null) ?? $this->year($result['store_date'] ?? null),
            null,
            $this->cover($result['image'] ?? null),
        );
    }

    /** A year from `1940` or from the front of `1940-04-01`. */
    private function year(mixed $value): ?int
    {
        if (!\is_string($value) || 1 !== preg_match('/^(\d{4})/', $value, $match)) {
            return null;
        }
        $year = (int) $match[1];

        return $year >= 1800 && $year <= 2200 ? $year : null;
    }

    private function detail(string $body): ComicVineDetail
    {
        $credits = $this->decode($body)['results'] ?? null;
        $people = \is_array($credits) ? ($credits['person_credits'] ?? null) : null;

        $writers = [];
        $artists = [];
        foreach (\is_array($people) ? \array_slice($people, 0, 400) : [] as $person) {
            $name = \is_array($person) ? $this->clean($person['name'] ?? null, self::MAX_NAME) : null;
            $roles = \is_array($person) ? $this->roles($person['role'] ?? null) : [];
            if (null === $name) {
                continue;
            }
            if ([] !== array_intersect($roles, self::WRITER_ROLES) && !\in_array($name, $writers, true) && \count($writers) < self::MAX_PEOPLE) {
                $writers[] = $name;
            }
            if ([] !== array_intersect($roles, self::ARTIST_ROLES) && !\in_array($name, $artists, true) && \count($artists) < self::MAX_PEOPLE) {
                $artists[] = $name;
            }
        }

        return new ComicVineDetail($writers, $artists);
    }

    /**
     * What a credit says they did: `writer, penciler` as `['writer', 'penciler']`, lower case and without blanks.
     *
     * @return list<string>
     */
    private function roles(mixed $role): array
    {
        if (!\is_string($role) || \strlen($role) > 200) {
            return [];
        }

        return array_values(array_filter(array_map(static fn (string $part): string => strtolower(trim($part)), preg_split('/[,;&\/]/', $role) ?: [])));
    }

    /**
     * Text from Comic Vine, cleaned: and gone if it holds our key, which no text of a comic ever should (a Comic Vine
     * that echoed the address of a call back in a field must not make the key travel on to the app and the cache).
     */
    private function clean(mixed $value, int $max): ?string
    {
        $text = TextCleaner::clean($value, $max);
        $key = trim($this->apiKey);

        return null !== $text && '' !== $key && str_contains($text, $key) ? null : $text;
    }

    /**
     * The medium image (or else the small one; never the original, which can be huge) of `image`, if it is an address
     * of its image site: `https://comicvine.gamespot.com/a/uploads/<names>/<file>` ending in an image extension, with
     * nothing in it that is not a plain name.
     */
    private function cover(mixed $image): ?string
    {
        if (!\is_array($image)) {
            return null;
        }
        foreach (['medium_url', 'small_url'] as $size) {
            $url = self::coverAddress($image[$size] ?? null);
            if (null !== $url) {
                return $url;
            }
        }

        return null;
    }

    /** One cover address as Comic Vine serves them, and nothing else: also what is read back from the cache (SEG-057). */
    public static function coverAddress(mixed $url): ?string
    {
        return \is_string($url) && \strlen($url) <= self::MAX_URL
            && 1 === preg_match('#^https://comicvine\.gamespot\.com/a/uploads/(?:[A-Za-z0-9_-]+/)+[A-Za-z0-9_-]+\.(?:jpg|jpeg|png|gif|webp)$#D', $url)
            ? $url : null;
    }
}
