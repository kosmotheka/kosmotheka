<?php

declare(strict_types=1);

namespace App\Metadata;

use App\Entity\MetadataCacheEntry;
use App\Repository\MetadataCacheRepository;
use App\Security\AccountException;
use App\Security\RequestLimiter;
use Doctrine\DBAL\Exception\UniqueConstraintViolationException;
use Doctrine\ORM\EntityManagerInterface;
use Psr\Cache\CacheItemPoolInterface;
use Symfony\Component\Clock\ClockInterface;
use Symfony\Component\DependencyInjection\Attribute\Target;

/**
 * Looks an ISBN up for an account (7.8, D83): the cache first, then the provider, within what the server allows itself.
 *
 * It never learns who asked (the caller spends the per-account limit before), so no row and no log can say it. A
 * failure is never stored. A provider that answers it is broken is asked up to three times. When a provider says its quota is gone (or refuses the key), nobody asks it again for
 * {@see self::PAUSE_SECONDS}; that pause, and the day's cap on real calls, are what stop a bad day from hammering it.
 */
final readonly class BookLookup
{
    /** How long the server leaves a provider alone after it said no on quota. */
    private const int PAUSE_SECONDS = 1800;
    /** The most real calls one lookup may cost, when the provider answers that it is broken. */
    private const int ATTEMPTS = 3;
    private const float SECONDS_BETWEEN_ATTEMPTS = 0.2;

    public function __construct(
        private MetadataCacheRepository $cache,
        private EntityManagerInterface $entityManager,
        private RequestLimiter $limiter,
        private ClockInterface $clock,
        #[Target('cache.metadata')]
        private CacheItemPoolInterface $pauses,
    ) {
    }

    /**
     * @return ?BookMetadata null when the book is not known
     *
     * @throws AccountException with AccountErrorCode::MetadataQuotaExhausted or MetadataFailed
     */
    public function lookup(BookProvider $provider, Isbn $isbn): ?BookMetadata
    {
        $now = $this->clock->now();
        $entry = $this->cache->findOneByProviderAndCode($provider->name(), $isbn->value);
        if ($entry instanceof MetadataCacheEntry && $entry->isFreshAt($now)) {
            return $this->fromEntry($entry);
        }

        $this->requireNotPaused($provider);
        $book = $this->ask($provider, $isbn);
        $this->remember($entry, $provider, $isbn, $book, $now);

        return $book;
    }

    /**
     * Asks the provider, again when it answers that it is broken: Google Books does so often and answers the same
     * question a moment later. Every attempt is a real call and spends the server's day; a slow provider is not asked
     * again, so nobody waits three times as long.
     *
     * @throws AccountException with AccountErrorCode::MetadataQuotaExhausted or MetadataFailed
     */
    private function ask(BookProvider $provider, Isbn $isbn): ?BookMetadata
    {
        for ($attempt = 1;; ++$attempt) {
            $this->limiter->consumeGoogleBooksCall();

            try {
                return $provider->lookup($isbn);
            } catch (MetadataProviderException $exception) {
                if ($exception->quota) {
                    $this->pause($provider);

                    throw AccountException::metadataQuotaExhausted(self::PAUSE_SECONDS);
                }
                if (!$exception->brokeItself || $attempt >= self::ATTEMPTS) {
                    throw AccountException::metadataFailed();
                }
            }

            $this->clock->sleep(self::SECONDS_BETWEEN_ATTEMPTS);
        }
    }

    private function remember(?MetadataCacheEntry $entry, BookProvider $provider, Isbn $isbn, ?BookMetadata $book, \DateTimeImmutable $now): void
    {
        $found = $book instanceof BookMetadata;
        $payload = $found ? $book->toArray() : BookMetadata::notFoundPayload();
        // The flag is a column of its own; the payload keeps only the book.
        unset($payload['found']);

        if ($entry instanceof MetadataCacheEntry) {
            $entry->renew($found, $payload, $now);
        } else {
            $this->entityManager->persist(new MetadataCacheEntry($provider->name(), $isbn->value, $found, $payload, $now));
        }

        try {
            $this->entityManager->flush();
        } catch (UniqueConstraintViolationException) {
            // Another request stored the same book a moment ago: its row is as good as ours.
        }
    }

    private function fromEntry(MetadataCacheEntry $entry): ?BookMetadata
    {
        if (!$entry->isFound()) {
            return null;
        }
        $p = $entry->getPayload();
        $authors = [];
        foreach (\is_array($p['authors'] ?? null) ? $p['authors'] : [] as $author) {
            if (\is_string($author)) {
                $authors[] = $author;
            }
        }

        return new BookMetadata(
            title: $this->string($p['title'] ?? null),
            subtitle: $this->string($p['subtitle'] ?? null),
            authors: $authors,
            publisher: $this->string($p['publisher'] ?? null),
            publishedYear: \is_int($p['published_year'] ?? null) ? $p['published_year'] : null,
            language: $this->string($p['language'] ?? null),
            coverUrl: $this->string($p['cover_url'] ?? null),
            externalId: $this->string($p['external_id'] ?? null),
        );
    }

    private function string(mixed $value): ?string
    {
        return \is_string($value) ? $value : null;
    }

    private function requireNotPaused(BookProvider $provider): void
    {
        $item = $this->pauses->getItem($this->pauseKey($provider));
        if ($item->isHit()) {
            $until = $item->get();
            $left = \is_int($until) ? $until - $this->clock->now()->getTimestamp() : self::PAUSE_SECONDS;

            throw AccountException::metadataQuotaExhausted($left);
        }
    }

    private function pause(BookProvider $provider): void
    {
        $item = $this->pauses->getItem($this->pauseKey($provider));
        $item->set($this->clock->now()->getTimestamp() + self::PAUSE_SECONDS)->expiresAfter(self::PAUSE_SECONDS);
        $this->pauses->save($item);
    }

    private function pauseKey(BookProvider $provider): string
    {
        return 'pause_'.$provider->name();
    }
}
