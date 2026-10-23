<?php

declare(strict_types=1);

namespace App\Metadata;

use Symfony\Component\DependencyInjection\Attribute\Autowire;
use Symfony\Contracts\HttpClient\HttpClientInterface;
use Symfony\Contracts\HttpClient\ResponseInterface;

/**
 * The GameUPC API (gameupc.com/gameupc-oas.yaml) with the server's key, in the `x-api-key` header, never in an address.
 * Addresses are fixed: the only things that change them are a barcode that is digits with a right check digit and, for
 * a choice, a whole number. Whatever GameUPC sends is read with a size cap and checked field by field; its covers are
 * BoardGameGeek's and pass the same check as theirs.
 */
final readonly class GameUpcHttpSource implements GameUpcSource
{
    private const string API = 'https://api.gameupc.com/v1';
    private const float TIMEOUT_SECONDS = 6.0;
    private const int MAX_BODY_BYTES = 256 * 1024;
    /** How many of its guesses are looked at to find eight that are well formed. */
    private const int GAMES_LOOKED_AT = 40;

    public function __construct(
        private HttpClientInterface $httpClient,
        #[Autowire(env: 'GAMEUPC_API_KEY')]
        private string $apiKey,
    ) {
    }

    public function isConfigured(): bool
    {
        return '' !== trim($this->apiKey);
    }

    public function lookup(Barcode $code): GameUpcAnswer
    {
        $response = $this->start('GET', \sprintf('%s/upc/%s', self::API, $code->value), null);
        $decoded = json_decode($this->read($response), true, 16);
        if (!\is_array($decoded)) {
            throw MetadataProviderException::unusable();
        }

        $games = [];
        $listed = $decoded['bgg_info'] ?? [];
        foreach (\array_slice(\is_array($listed) ? array_values($listed) : [], 0, self::GAMES_LOOKED_AT) as $row) {
            $game = \is_array($row) ? $this->game($row) : null;
            if ($game instanceof GameUpcGame) {
                $games[] = $game;
            }
        }

        // The documentation calls it `bgg_info_status` in its examples and `bgg_status` in its schema.
        $status = $decoded['bgg_info_status'] ?? $decoded['bgg_status'] ?? null;
        if ('verified' === $status && [] !== $games) {
            return GameUpcAnswer::verified($games[0]);
        }

        return GameUpcAnswer::choose($games);
    }

    public function choose(Barcode $code, int $gameId, string $userId): void
    {
        $response = $this->start('POST', \sprintf('%s/upc/%s/bgg_id/%d', self::API, $code->value, $gameId), ['user_id' => $userId]);
        // What it answers is the barcode's record again: nothing is taken from it.
        $this->read($response);
    }

    /** @param array<string, string>|null $json */
    private function start(string $method, string $url, ?array $json): ResponseInterface
    {
        $options = [
            'headers' => ['x-api-key' => trim($this->apiKey), 'Accept' => 'application/json'],
            'timeout' => self::TIMEOUT_SECONDS,
            'max_duration' => self::TIMEOUT_SECONDS,
            'max_redirects' => 0,
        ];
        if (null !== $json) {
            $options['json'] = $json;
        }

        try {
            return $this->httpClient->request($method, $url, $options);
        } catch (\Throwable) {
            // On purpose without `previous`: what the http client throws can carry the key's header.
            throw MetadataProviderException::failed();
        }
    }

    /** @throws MetadataProviderException */
    private function read(ResponseInterface $response): string
    {
        try {
            $status = $response->getStatusCode();
            // 429 is its "Overloaded; try again later", which includes its hundred new barcodes a day.
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

    /** @param array<array-key, mixed> $row */
    private function game(array $row): ?GameUpcGame
    {
        $id = $row['id'] ?? null;
        if (!\is_int($id) || $id < 1 || $id > GameUpcGame::MAX_ID) {
            return null;
        }
        $title = TextCleaner::whole($row['name'] ?? null, BggHttpSource::MAX_TEXT);
        if (null === $title) {
            return null;
        }

        return new GameUpcGame($id, $title, BggHttpSource::coverAddress($row['image_url'] ?? null) ?? BggHttpSource::coverAddress($row['thumbnail_url'] ?? null));
    }
}
