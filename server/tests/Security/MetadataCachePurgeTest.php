<?php

declare(strict_types=1);

namespace App\Tests\Security;

use App\Entity\MetadataCacheEntry;
use App\Security\AccountPurger;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;
use Symfony\Component\Clock\Test\ClockSensitiveTrait;

/** The purge that already runs every hour also empties the metadata cache of what is past its age (7.8). */
final class MetadataCachePurgeTest extends KernelTestCase
{
    use ClockSensitiveTrait;

    public function testPurgeDeletesWhatIsPastItsAgeAndKeepsTheRest(): void
    {
        self::mockTime('2026-06-30 12:00:00');
        self::bootKernel();
        $entityManager = self::getContainer()->get(EntityManagerInterface::class);
        $entries = [
            'found-old' => [true, '2026-05-30 11:00:00'],      // 31 days: gone
            'found-fresh' => [true, '2026-06-01 12:00:00'],    // 29 days: kept
            'missing-old' => [false, '2026-06-29 11:00:00'],   // 25 hours: gone
            'missing-fresh' => [false, '2026-06-29 13:00:00'], // 23 hours: kept
        ];
        // A title search is kept a day whether or not it found anything; its detail, thirty days.
        $entityManager->persist(new MetadataCacheEntry(MetadataCacheEntry::TMDB_SEARCH, str_repeat('a', 64), true, [], new \DateTimeImmutable('2026-06-29 11:00:00')));
        $entityManager->persist(new MetadataCacheEntry(MetadataCacheEntry::TMDB_SEARCH, str_repeat('b', 64), true, [], new \DateTimeImmutable('2026-06-29 13:00:00')));
        // A video game search is kept a week.
        $entityManager->persist(new MetadataCacheEntry(MetadataCacheEntry::RAWG_SEARCH, str_repeat('c', 64), true, [], new \DateTimeImmutable('2026-06-22 11:00:00')));
        $entityManager->persist(new MetadataCacheEntry(MetadataCacheEntry::RAWG_SEARCH, str_repeat('d', 64), true, [], new \DateTimeImmutable('2026-06-24 13:00:00')));
        $entityManager->persist(new MetadataCacheEntry('rawg_game', '3498', true, [], new \DateTimeImmutable('2026-06-01 12:00:00')));
        $entityManager->persist(new MetadataCacheEntry('tmdb_movie_en', '603', true, [], new \DateTimeImmutable('2026-06-01 12:00:00')));
        $entityManager->persist(new MetadataCacheEntry('tmdb_movie_en', '604', true, [], new \DateTimeImmutable('2026-05-30 11:00:00')));
        foreach (array_keys($entries) as $n => $name) {
            [$found, $at] = $entries[$name];
            $entityManager->persist(new MetadataCacheEntry('google_books', '97800000000'.\sprintf('%02d', $n), $found, [], new \DateTimeImmutable($at)));
        }
        $entityManager->flush();

        $result = self::getContainer()->get(AccountPurger::class)->purge();

        self::assertSame(5, $result['metadata_cache']);
        $entityManager->clear();
        $left = array_map(
            static fn (MetadataCacheEntry $entry): string => $entry->getCode(),
            $entityManager->getRepository(MetadataCacheEntry::class)->findBy([], ['code' => 'ASC']),
        );
        self::assertSame(['3498', '603', '9780000000001', '9780000000003', str_repeat('b', 64), str_repeat('d', 64)], $left);
    }
}
