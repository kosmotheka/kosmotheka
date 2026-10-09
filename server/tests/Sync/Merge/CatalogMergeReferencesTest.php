<?php

declare(strict_types=1);

namespace App\Tests\Sync\Merge;

use App\Entity\Catalog\Edition;
use App\Entity\Catalog\FieldVersion;
use App\Enum\CatalogEntity;
use App\Enum\ChangeOutcome;
use App\Sync\Merge\CatalogMerge;
use App\Tests\Support\CatalogRows;
use App\Tests\Support\Ops;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;

/**
 * The checks before writing apply only to the fields that win by HLC, as in the Rust client
 * (sync.rs, apply_change): a field that loses is not written, so it cannot break a constraint.
 * This is an order of delivery fixed by hand, not a conformance case: delivered the other way round,
 * Rust skips the whole change.
 */
final class CatalogMergeReferencesTest extends KernelTestCase
{
    private EntityManagerInterface $em;
    private CatalogMerge $merge;

    protected function setUp(): void
    {
        self::bootKernel();
        $this->em = self::getContainer()->get(EntityManagerInterface::class);
        $this->merge = self::getContainer()->get(CatalogMerge::class);
    }

    public function testALosingReferenceToARowThatNeverExistedDoesNotSkipTheRestOfTheChange(): void
    {
        $library = new CatalogRows($this->em)->cloudLibrary();
        $this->em->flush();
        $hlc = static fn (int $time): string => Ops::hlc($time);

        $this->merge->apply($library, Ops::operation($hlc(1), [
            Ops::create(CatalogEntity::Work, 'w1', ['kind' => 'book', 'title' => 'Dune']),
            Ops::create(CatalogEntity::Publisher, 'p1', ['name' => 'Acme']),
            Ops::create(CatalogEntity::Edition, 'e1', ['work_id' => 'w1', 'title' => 'Dune', 'publisher_id' => 'p1']),
        ]));
        // A, the later one: no publisher.
        $this->merge->apply($library, Ops::operation($hlc(3), [Ops::update(CatalogEntity::Edition, 'e1', ['publisher_id' => null])]));
        // B, the earlier one: a publisher that never existed, and a subtitle.
        $outcome = $this->merge->apply($library, Ops::operation($hlc(2), [
            Ops::update(CatalogEntity::Edition, 'e1', ['publisher_id' => 'never-existed', 'subtitle' => 'x']),
        ]));
        $this->merge->flush();

        self::assertSame([ChangeOutcome::Written], $outcome);
        self::assertTrue($this->em->isOpen());
        $edition = $this->em->getRepository(Edition::class)->findOneBy(['id' => 'e1']) ?? self::fail('The edition is gone.');
        self::assertSame('x', $edition->getSubtitle());
        self::assertNull($edition->getPublisher());
        $versions = [];
        foreach ($this->em->getRepository(FieldVersion::class)->findBy(['entity' => 'edition', 'entityId' => 'e1']) as $version) {
            $versions[$version->getField()] = $version->getHlc();
        }
        self::assertSame($hlc(2), $versions['subtitle'] ?? null);
        self::assertSame($hlc(3), $versions['publisher_id'] ?? null);
    }
}
