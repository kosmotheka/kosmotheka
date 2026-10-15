<?php

declare(strict_types=1);

namespace App\Metadata;

use App\Entity\MetadataCacheEntry;
use App\Repository\MetadataCacheRepository;
use App\Security\AccountException;
use App\Security\RequestLimiter;
use Doctrine\DBAL\Exception\UniqueConstraintViolationException;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;

/**
 * Looks an ISBN up for an account (7.8, D83): the cache first, then the provider, within what the server allows itself.
 *
 * It never learns who asked (the caller spends the per-account limit before), so no row and no log can say it. A
 * failure is never stored. The pause after a quota refusal and the new tries are `ProviderGate`'s; the day's cap on
 * real calls is the limiter's.
 */
final readonly class BookLookup
{
    public function __construct(
        private MetadataCacheRepository $cache,
        private EntityManagerInterface $entityManager,
        private RequestLimiter $limiter,
        private ClockInterface $clock,
        private ProviderGate $gate,
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

        $this->gate->requireOpen($provider->name());
        $book = $this->gate->call(
            $provider->name(),
            $this->limiter->consumeGoogleBooksCall(...),
            static fn (): ?BookMetadata => $provider->lookup($isbn),
        );
        $this->remember($entry, $provider, $isbn, $book, $now);

        return $book;
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
            coverUrl: GoogleBooksHttpProvider::cover($p['cover_url'] ?? null),
            externalId: $this->string($p['external_id'] ?? null),
        );
    }

    private function string(mixed $value): ?string
    {
        return \is_string($value) ? $value : null;
    }
}
