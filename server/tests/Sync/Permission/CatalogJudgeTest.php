<?php

declare(strict_types=1);

namespace App\Tests\Sync\Permission;

use App\Entity\Catalog\Tag;
use App\Enum\CatalogEntity;
use App\Enum\OperationStatus;
use App\Tests\Support\CatalogRows;
use App\Tests\Support\Judged;
use App\Tests\Support\JudgedTestTrait;
use App\Tests\Support\Ops;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;

/**
 * When the judge looks at the whole library again, and when it does not.
 */
final class CatalogJudgeTest extends KernelTestCase
{
    use JudgedTestTrait;

    private const string ADMIN_DEVICE = '01997c7e-0000-7000-8000-0000000000a1';
    private const string HUGO_DEVICE = '01997c7e-0000-7000-8000-0000000000b1';
    private const string ADMIN = '01997c7e-0000-7000-8000-0000000000c1';
    private const string HUGO = '01997c7e-0000-7000-8000-0000000000c2';

    private EntityManagerInterface $em;

    protected function setUp(): void
    {
        self::bootKernel();
        $this->em = self::getContainer()->get(EntityManagerInterface::class);
    }

    public function testALateOperationRebuildsTheLibrary(): void
    {
        $judged = $this->library();
        $judged->deliver(Judged::founding(10, self::ADMIN_DEVICE, self::ADMIN));
        $judged->deliver(Judged::op(40, self::ADMIN_DEVICE, [Ops::create(CatalogEntity::Tag, '01997c7e-0000-7000-8000-0000000000d1', ['name' => 'Newer'])]));
        $this->ghost($judged);

        $late = $judged->deliver(Judged::op(30, self::ADMIN_DEVICE, [Ops::create(CatalogEntity::Tag, '01997c7e-0000-7000-8000-0000000000d2', ['name' => 'Older'])]));

        self::assertSame(OperationStatus::Applied, $late);
        self::assertSame(['Newer', 'Older'], $this->tags($judged), 'the library was judged again from its log');
    }

    /**
     * `a_fence_that_arrives_before_its_operation_is_judged_again_when_it_arrives` of sync.rs: the demotion of
     * Hugo, with the mark of his last operation, reaches the server before that operation. At first all he
     * wrote looks fenced; when the marked operation comes, the library is judged again and what he wrote up to
     * it counts (D87).
     */
    public function testAFenceThatArrivesBeforeItsOperationIsJudgedAgainWhenItArrives(): void
    {
        $judged = $this->library();
        $judged->deliver($founding = Judged::founding(1, self::ADMIN_DEVICE, self::ADMIN));
        $judged->deliver($registerHugo = Judged::op(2, self::HUGO_DEVICE, [Ops::create(CatalogEntity::Device, self::HUGO_DEVICE, ['public_key' => 'k', 'name' => 'Device'])]));
        $roles = Judged::op(3, self::ADMIN_DEVICE, [
            Ops::create(CatalogEntity::Member, self::HUGO, ['name' => 'Hugo', 'role' => 'editor', 'can_share' => false]),
            Ops::update(CatalogEntity::Device, self::HUGO_DEVICE, ['member_id' => self::HUGO]),
        ], $founding->id);
        $older = Judged::op(4, self::HUGO_DEVICE, [Ops::create(CatalogEntity::Tag, '01997c7e-0000-7000-8000-0000000000d1', ['name' => 'Older edit'])], $registerHugo->id);
        $known = Judged::op(5, self::HUGO_DEVICE, [Ops::create(CatalogEntity::Tag, '01997c7e-0000-7000-8000-0000000000d2', ['name' => 'Known edit'])], $older->id);
        $demotion = Judged::op(6, self::ADMIN_DEVICE, [
            Ops::update(CatalogEntity::Member, self::HUGO, ['role' => 'reader']),
            Ops::update(CatalogEntity::Device, self::HUGO_DEVICE, ['trusted_through' => $known->id]),
        ], $roles->id);

        $judged->deliver($roles);
        self::assertSame(OperationStatus::Applied, $judged->deliver($demotion));
        // Hugo's operations come after the demotion that fences them, and the mark is not in the log yet.
        self::assertSame(OperationStatus::Request, $judged->deliver($older), 'fenced: the mark is not known yet');
        self::assertSame(OperationStatus::Applied, $judged->deliver($known), 'the marked operation arrives');

        self::assertSame(OperationStatus::Applied, $judged->status($older->id), 'judged again, it is inside the fence');
        self::assertSame(['Known edit', 'Older edit'], $this->tags($judged));
    }

    private function library(): Judged
    {
        $library = new CatalogRows($this->em)->cloudLibrary();
        $this->em->flush();

        return $this->judged($library);
    }

    /** A row that is in the catalog but not in the log: only a rebuild gets rid of it. */
    private function ghost(Judged $judged): void
    {
        new CatalogRows($this->em)->row(Tag::class, $judged->library, '01997c7e-0000-7000-8000-0000000000ee', ['name' => 'Ghost']);
        $this->em->flush();
        $this->em->clear();
    }

    /**
     * @return list<string> the names of the tags of the library
     */
    private function tags(Judged $judged): array
    {
        $names = array_map(
            static fn (Tag $tag): string => $tag->getName(),
            $this->em->getRepository(Tag::class)->findBy(['cloudLibrary' => $this->em->getReference($judged->library::class, $judged->library->getId())]),
        );
        sort($names);
        $this->em->clear();

        return $names;
    }
}
