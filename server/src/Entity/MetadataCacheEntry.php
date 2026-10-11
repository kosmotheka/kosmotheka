<?php

declare(strict_types=1);

namespace App\Entity;

use App\Repository\MetadataCacheRepository;
use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;
use Symfony\Component\Uid\Uuid;

/**
 * What a provider said about one book (7.8, D83): a row per provider and ISBN-13, already in the fixed shape. It
 * deliberately says nothing about who asked, and a failure of the provider is never kept here. A found book stays 30
 * days, a missing one a day, a title search a day; the two periods live here and nowhere else (the purge reads them too).
 */
#[ORM\Entity(repositoryClass: MetadataCacheRepository::class)]
#[ORM\Table(name: 'metadata_cache')]
#[ORM\UniqueConstraint(name: 'uniq_metadata_cache_provider_code', columns: ['provider', 'code'])]
#[ORM\Index(name: 'idx_metadata_cache_fetched', columns: ['fetched_at'])]
class MetadataCacheEntry
{
    public const string FOUND_TTL = '30 days';
    public const string MISSING_TTL = '1 day';
    /** A title search is kept a day, found or not: the provider of those rows is this one. */
    public const string TMDB_SEARCH = 'tmdb_search';
    public const string SEARCH_TTL = '1 day';

    #[ORM\Id]
    #[ORM\Column(type: 'uuid')]
    private readonly Uuid $id;

    public function __construct(
        #[ORM\Column(length: 32)]
        private readonly string $provider,
        #[ORM\Column(length: 64)]
        private readonly string $code,
        #[ORM\Column]
        private bool $found,
        /** @var array<string, mixed> the book without `found`, as `BookMetadata::toArray` gives it */
        #[ORM\Column(type: Types::JSON)]
        private array $payload,
        #[ORM\Column]
        private \DateTimeImmutable $fetchedAt,
    ) {
        $this->id = Uuid::v7();
    }

    public function getProvider(): string
    {
        return $this->provider;
    }

    public function getCode(): string
    {
        return $this->code;
    }

    public function isFound(): bool
    {
        return $this->found;
    }

    /** @return array<string, mixed> */
    public function getPayload(): array
    {
        return $this->payload;
    }

    public function getFetchedAt(): \DateTimeImmutable
    {
        return $this->fetchedAt;
    }

    public function isFreshAt(\DateTimeImmutable $now): bool
    {
        return $this->fetchedAt > $now->modify('-'.$this->ttl());
    }

    private function ttl(): string
    {
        if (self::TMDB_SEARCH === $this->provider) {
            return self::SEARCH_TTL;
        }

        return $this->found ? self::FOUND_TTL : self::MISSING_TTL;
    }

    /** @param array<string, mixed> $payload */
    public function renew(bool $found, array $payload, \DateTimeImmutable $at): void
    {
        $this->found = $found;
        $this->payload = $payload;
        $this->fetchedAt = $at;
    }
}
