<?php

declare(strict_types=1);

namespace App\Tests\Sync\Permission;

use App\Dto\ChangePayload;
use App\Dto\OperationPayload;
use App\Entity\Catalog\Member;
use App\Enum\CatalogEntity;
use App\Enum\OperationStatus;
use App\Tests\Support\CatalogRows;
use App\Tests\Support\Judged;
use App\Tests\Support\JudgedTestTrait;
use App\Tests\Support\Ops;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\Test\KernelTestCase;

/**
 * What a device that knows the ids of another library, or an `EntityManager` that remembers, must not be
 * able to do. Each test is written against the defence it checks and was seen failing without it.
 */
final class CatalogLibraryViewHostileTest extends KernelTestCase
{
    use JudgedTestTrait;

    /** The same text ids in two libraries (an id of the app is only unique inside its library). */
    private const string DEVICE = '01997c7e-0000-7000-8000-0000000000a1';
    private const string MEMBER = '01997c7e-0000-7000-8000-0000000000c1';
    private const string OTHER_DEVICE = '01997c7e-0000-7000-8000-0000000000b1';
    private const string OTHER_MEMBER = '01997c7e-0000-7000-8000-0000000000c2';
    private const string ADMIN_DEVICE = '01997c7e-0000-7000-8000-0000000000a9';
    private const string ADMIN_MEMBER = '01997c7e-0000-7000-8000-0000000000c9';
    private const string TAG = '01997c7e-0000-7000-8000-0000000000d1';

    private EntityManagerInterface $em;
    private int $clock = 0;

    protected function setUp(): void
    {
        self::bootKernel();
        $this->em = self::getContainer()->get(EntityManagerInterface::class);
    }

    /**
     * Jhonny's library: his device belongs to an administrator. The family's: a device and a member with
     * those same ids, but the member is a reader. What he does there is judged by the family's table.
     */
    public function testTheRoleOfADeviceIsLookedUpInsideItsOwnLibrary(): void
    {
        $jhonny = $this->library();
        $this->register($jhonny, self::DEVICE, self::MEMBER, 'admin');
        $family = $this->library();
        $this->register($family, self::ADMIN_DEVICE, self::ADMIN_MEMBER, 'admin');
        $this->register($family, self::DEVICE, self::MEMBER, 'reader', self::ADMIN_DEVICE);

        $status = $family->deliver($this->op(self::DEVICE, [Ops::create(CatalogEntity::Tag, self::TAG, ['name' => 'Not for readers'])]));

        self::assertSame(OperationStatus::Request, $status);
    }

    /**
     * The id of an operation is public. In Jhonny's library `X` is written by his device; in the family's,
     * `X` is a request of Laura. Jhonny, a reader in the family's, tries to withdraw `X` there.
     */
    public function testWhoAskedIsLookedUpInsideTheLibraryOfTheRequest(): void
    {
        $x = '01997c7e-0000-7000-8000-0000000000f1';
        $jhonny = $this->library();
        $this->register($jhonny, self::DEVICE, self::MEMBER, null);
        $jhonny->deliver($this->op(self::DEVICE, [Ops::create(CatalogEntity::Tag, self::TAG, ['name' => 'Jhonny wrote X'])], $x));

        $family = $this->library();
        $this->register($family, self::ADMIN_DEVICE, self::ADMIN_MEMBER, 'admin');
        $this->register($family, self::DEVICE, self::MEMBER, 'reader', self::ADMIN_DEVICE);
        $this->register($family, self::OTHER_DEVICE, self::OTHER_MEMBER, 'reader', self::ADMIN_DEVICE);
        self::assertSame(OperationStatus::Request, $family->deliver($this->op(self::OTHER_DEVICE, [Ops::create(CatalogEntity::Tag, self::TAG, ['name' => 'Laura asks'])], $x)));

        $withdraw = $family->deliver($this->op(self::DEVICE, [Ops::create(CatalogEntity::ChangeRequest, $x, ['status' => 'withdrawn'])]));

        self::assertSame(OperationStatus::Request, $withdraw);
    }

    /**
     * The merge writes below the ORM. The `Member` of Laura, an editor, is loaded in the `EntityManager`;
     * her demotion is delivered; her next edit must be judged by the demotion, not by the copy in memory.
     */
    public function testTheRoleComesFromTheDatabaseNeverFromAnEntityInMemory(): void
    {
        $family = $this->library();
        $this->register($family, self::ADMIN_DEVICE, self::ADMIN_MEMBER, 'admin');
        $this->register($family, self::OTHER_DEVICE, self::OTHER_MEMBER, 'editor', self::ADMIN_DEVICE);
        $laura = $this->em->getRepository(Member::class)->findOneBy(['cloudLibrary' => $family->library, 'id' => self::OTHER_MEMBER]) ?? self::fail('Laura is not there.');
        self::assertSame('editor', $laura->getRole());

        self::assertSame(OperationStatus::Applied, $family->deliver($this->op(self::ADMIN_DEVICE, [Ops::update(CatalogEntity::Member, self::OTHER_MEMBER, ['role' => 'reader'])])));
        $edit = $family->deliver($this->op(self::OTHER_DEVICE, [Ops::create(CatalogEntity::Tag, self::TAG, ['name' => 'After the demotion'])]));

        self::assertSame(OperationStatus::Request, $edit);
        self::assertSame('editor', $laura->getRole(), 'the entity in memory is still the old one: the view did not use it');
    }

    private function library(): Judged
    {
        $library = new CatalogRows($this->em)->cloudLibrary();
        $this->em->flush();

        return $this->judged($library);
    }

    /**
     * Registers a device (it registers itself) and its member with that role, written by `$by`: the device
     * itself for the first one, since a library without roles lets everybody do everything, and an
     * administrator after that.
     */
    private function register(Judged $judged, string $device, string $member, ?string $role, ?string $by = null): void
    {
        $judged->deliver($this->op($device, [Ops::create(CatalogEntity::Device, $device, ['public_key' => 'k', 'name' => 'Device'])]));
        $fields = ['name' => 'Someone'] + (null === $role ? [] : ['role' => $role]);
        $judged->deliver($this->op($by ?? $device, [
            Ops::create(CatalogEntity::Member, $member, $fields),
            Ops::update(CatalogEntity::Device, $device, ['member_id' => $member]),
        ]));
    }

    /**
     * @param list<ChangePayload> $changes
     */
    private function op(string $device, array $changes, ?string $id = null): OperationPayload
    {
        return Judged::op(++$this->clock, $device, $changes, null, $id);
    }
}
