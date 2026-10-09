<?php

declare(strict_types=1);

namespace App\Tests\Sync\Merge;

use App\Entity\Catalog\RowAuthor;
use App\Enum\CatalogEntity;
use App\Enum\ChangeOutcome;
use App\Sync\Merge\CatalogMerge;
use App\Tests\Support\CatalogRows;
use App\Tests\Support\Ops;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;

/**
 * Who created each row (D45): the merge notes the device of the first create that wrote it.
 */
final class CatalogMergeAuthorsTest extends KernelTestCase
{
    private const string FIRST = '01997c7e-0000-7000-8000-0000000000a1';
    private const string SECOND = '01997c7e-0000-7000-8000-0000000000a2';

    private EntityManagerInterface $em;
    private CatalogMerge $merge;

    protected function setUp(): void
    {
        self::bootKernel();
        $this->em = self::getContainer()->get(EntityManagerInterface::class);
        $this->merge = self::getContainer()->get(CatalogMerge::class);
    }

    public function testTheFirstCreateCountsAndALaterCreateOfTheSameRowIsAnEdit(): void
    {
        $library = new CatalogRows($this->em)->cloudLibrary();
        $this->em->flush();

        $this->merge->apply($library, Ops::operation(Ops::hlc(1, 0, self::FIRST), [Ops::create(CatalogEntity::Tag, 'tag-1', ['name' => 'Tehanu'])]));
        // The same row created again by another device: a later field write, not a new author. Across a flush too.
        $this->merge->flush();
        $second = $this->merge->apply($library, Ops::operation(Ops::hlc(2, 0, self::SECOND), [Ops::create(CatalogEntity::Tag, 'tag-1', ['name' => 'Tehanu, again'])]));
        $this->merge->flush();

        self::assertSame([ChangeOutcome::Written], $second);
        self::assertSame(['tag|tag-1' => self::FIRST], $this->authors());
    }

    public function testTheFirstCreateCountsInTheSameFlushToo(): void
    {
        $library = new CatalogRows($this->em)->cloudLibrary();
        $this->em->flush();

        $this->merge->apply($library, Ops::operation(Ops::hlc(1, 0, self::FIRST), [Ops::create(CatalogEntity::Tag, 'tag-1', ['name' => 'Tehanu'])]));
        $this->merge->apply($library, Ops::operation(Ops::hlc(2, 0, self::SECOND), [Ops::create(CatalogEntity::Tag, 'tag-1', ['name' => 'Again'])]));
        $this->merge->flush();

        self::assertSame(['tag|tag-1' => self::FIRST], $this->authors());
    }

    public function testACreateThatIsSkippedNotesNothing(): void
    {
        $library = new CatalogRows($this->em)->cloudLibrary();
        $this->em->flush();

        // A tag with no name cannot be created; an update of a row that does not exist is skipped too.
        $outcomes = [
            ...$this->merge->apply($library, Ops::operation(Ops::hlc(1, 0, self::FIRST), [Ops::create(CatalogEntity::Tag, 'tag-1', [])])),
            ...$this->merge->apply($library, Ops::operation(Ops::hlc(2, 0, self::FIRST), [Ops::update(CatalogEntity::Tag, 'tag-2', ['name' => 'x'])])),
        ];
        $this->merge->flush();

        self::assertSame([ChangeOutcome::Skipped, ChangeOutcome::Skipped], $outcomes);
        self::assertSame([], $this->authors());
    }

    /**
     * @return array<string, string> the device that created each row, by entity and id
     */
    private function authors(): array
    {
        $authors = [];
        foreach ($this->em->getRepository(RowAuthor::class)->findAll() as $author) {
            $authors[$author->getEntity().'|'.$author->getEntityId()] = $author->getDeviceId();
        }

        return $authors;
    }
}
