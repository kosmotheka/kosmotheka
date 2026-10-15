<?php

declare(strict_types=1);

namespace App\Metadata;

use App\Enum\MediaKind;
use Symfony\Component\DependencyInjection\Attribute\Autowire;
use Symfony\Contracts\HttpClient\HttpClientInterface;
use Symfony\Contracts\HttpClient\ResponseInterface;

/**
 * The TMDB API v3 with the server's token (D62). Addresses are fixed; the only things that change them are the kind (a
 * closed list), the language (a closed list) and, for a detail, a number TMDB itself returned. The text a person
 * searches for travels as one encoded parameter, and the token in a header, never in an address. Whatever TMDB sends
 * is read with a size cap and cut and type-checked field by field.
 */
final readonly class TmdbHttpSource implements TmdbSource
{
    private const string API = 'https://api.themoviedb.org/3';
    private const string IMAGES = 'https://image.tmdb.org/t/p/w500';
    private const float TIMEOUT_SECONDS = 5.0;
    private const int MAX_BODY_BYTES = 512 * 1024;
    public const int MAX_HITS = 8;
    /** How many results are looked at to find those eight that are well formed. */
    private const int RESULTS_LOOKED_AT = 40;
    private const int MAX_TEXT = 300;
    private const int MAX_NAME = 200;
    private const int MAX_PEOPLE = 5;
    private const int MAX_ID = 1_000_000_000;

    public function __construct(
        private HttpClientInterface $httpClient,
        #[Autowire(env: 'TMDB_API_TOKEN')]
        private string $apiToken,
    ) {
    }

    public function isConfigured(): bool
    {
        return '' !== trim($this->apiToken);
    }

    public function search(MediaKind $kind, string $language, string $query): array
    {
        $response = $this->start(
            \sprintf('%s/search/%s', self::API, $kind->tmdbPath()),
            ['query' => $query, 'include_adult' => 'false', 'language' => $this->tag($language), 'page' => 1],
        );
        $decoded = json_decode($this->read($response), true, 16);
        $results = \is_array($decoded) ? ($decoded['results'] ?? null) : null;
        if (!\is_array($results)) {
            throw MetadataProviderException::failed();
        }

        $hits = [];
        foreach (\array_slice($results, 0, self::RESULTS_LOOKED_AT) as $result) {
            $hit = \is_array($result) ? $this->hit($kind, $result) : null;
            if ($hit instanceof TmdbHit) {
                $hits[] = $hit;
            }
            if (\count($hits) >= self::MAX_HITS) {
                break;
            }
        }

        return $hits;
    }

    public function details(MediaKind $kind, string $language, array $ids, \Closure $mayCall, \Closure $onQuota): array
    {
        // All started before any is read, so the time spent is that of the slowest and not the sum.
        $started = [];
        foreach ($ids as $id) {
            if (!$mayCall()) {
                break;
            }
            try {
                $query = ['language' => $this->tag($language)];
                if (MediaKind::Film === $kind) {
                    $query['append_to_response'] = 'credits';
                }
                $started[$id] = $this->begin(\sprintf('%s/%s/%d', self::API, $kind->tmdbPath(), $id), $query);
            } catch (\Throwable) {
                // On purpose without `previous`: what the http client throws can carry the token.
            }
        }

        $details = [];
        foreach ($started as $id => $response) {
            try {
                $details[$id] = $this->detail($kind, $this->read($response));
            } catch (MetadataProviderException $exception) {
                if ($exception->quota) {
                    $onQuota();
                }
                if ($exception->answeredUnusable) {
                    $details[$id] = null;
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
            'query' => $query,
            'headers' => ['Authorization' => 'Bearer '.trim($this->apiToken), 'Accept' => 'application/json'],
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

    /** @param array<array-key, mixed> $result */
    private function hit(MediaKind $kind, array $result): ?TmdbHit
    {
        $id = $result['id'] ?? null;
        if (!\is_int($id) || $id < 1 || $id > self::MAX_ID) {
            return null;
        }
        [$title, $original, $date] = MediaKind::Film === $kind
            ? ['title', 'original_title', 'release_date']
            : ['name', 'original_name', 'first_air_date'];
        $title = TextCleaner::clean($result[$title] ?? null, self::MAX_TEXT);
        $originalTitle = TextCleaner::clean($result[$original] ?? null, self::MAX_TEXT);
        if (null === $title && null === $originalTitle) {
            return null;
        }

        $language = $result['original_language'] ?? null;
        $poster = $result['poster_path'] ?? null;
        $released = $result[$date] ?? null;

        return new TmdbHit(
            $id,
            $title,
            $originalTitle,
            \is_string($released) && 1 === preg_match('/^(\d{4})/', $released, $year) && (int) $year[1] >= 1800 && (int) $year[1] <= 2200 ? (int) $year[1] : null,
            \is_string($language) && 1 === preg_match('/^[a-z]{2,3}$/D', $language) ? $language : null,
            \is_string($poster) && 1 === preg_match('/^\/[A-Za-z0-9]{1,100}\.(jpg|png)$/D', $poster) ? self::IMAGES.$poster : null,
        );
    }

    private function detail(MediaKind $kind, string $body): TmdbDetail
    {
        $decoded = json_decode($body, true, 16);
        if (!\is_array($decoded)) {
            throw MetadataProviderException::unusable();
        }

        if (MediaKind::TvSeries === $kind) {
            $seasons = $decoded['number_of_seasons'] ?? null;

            return new TmdbDetail(
                creators: $this->names($decoded['created_by'] ?? null, null),
                seasons: \is_int($seasons) && $seasons >= 1 && $seasons <= 500 ? $seasons : null,
            );
        }

        $runtime = $decoded['runtime'] ?? null;
        $credits = $decoded['credits'] ?? null;

        return new TmdbDetail(
            directors: $this->names(\is_array($credits) ? ($credits['crew'] ?? null) : null, 'Director'),
            durationMinutes: \is_int($runtime) && $runtime >= 1 && $runtime <= 1000 ? $runtime : null,
        );
    }

    /**
     * The names of people in a list TMDB sent, of those with this job if one is given.
     *
     * @return list<string>
     */
    private function names(mixed $people, ?string $job): array
    {
        $names = [];
        // The whole list is looked at: a big film names hundreds of people before its director, and the answer was
        // already read under a size cap.
        foreach (\is_array($people) ? $people : [] as $person) {
            if (!\is_array($person) || (null !== $job && ($person['job'] ?? null) !== $job)) {
                continue;
            }
            $name = TextCleaner::clean($person['name'] ?? null, self::MAX_NAME);
            if (null !== $name && !\in_array($name, $names, true)) {
                $names[] = $name;
            }
            if (\count($names) >= self::MAX_PEOPLE) {
                break;
            }
        }

        return $names;
    }

    private function tag(string $language): string
    {
        return 'es' === $language ? 'es-ES' : 'en-US';
    }

    /** A poster address as this source makes them, and nothing else: what is read back from the cache passes it (SEG-057). */
    public static function posterAddress(mixed $url): ?string
    {
        return \is_string($url) && 1 === preg_match('#^'.preg_quote(self::IMAGES, '#').'/[A-Za-z0-9]{1,100}\.(jpg|png)$#D', $url) ? $url : null;
    }
}
