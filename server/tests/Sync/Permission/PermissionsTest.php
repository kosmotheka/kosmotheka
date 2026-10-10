<?php

declare(strict_types=1);

namespace App\Tests\Sync\Permission;

use App\Dto\ChangePayload;
use App\Dto\OperationPayload;
use App\Enum\CatalogEntity;
use App\Enum\Role;
use App\Sync\Permission\Permissions;
use App\Tests\Support\FakeView;
use App\Tests\Support\Ops;
use PHPUnit\Framework\TestCase;

/**
 * The rules alone, with a made-up view of the library: the tests of the `tests` module of
 * crates/domain/src/permission.rs, with the same names (in camelCase).
 */
final class PermissionsTest extends TestCase
{
    private const string ME = '01997c7e-0000-7000-8000-0000000000fb';
    private const string TAG = '01997c7e-0000-7000-8000-000000000061';
    private const string WORK = '01997c7e-0000-7000-8000-0000000000d1';
    private const string MY_MEMBER = '01997c7e-0000-7000-8000-000000000013';
    private const string OTHER_MEMBER = '01997c7e-0000-7000-8000-000000000012';

    public function testEveryoneWithAPersonKeepsTheirOwnMarksAndOnlyTheirs(): void
    {
        $mark = fn (string $member): OperationPayload => $this->op([Ops::create(CatalogEntity::Mark, '01997c7e-0000-7000-8000-0000000000e1', ['member_id' => $member, 'work_id' => self::WORK, 'status' => 'finished'])]);
        $update = $this->op([Ops::update(CatalogEntity::Mark, '01997c7e-0000-7000-8000-0000000000e1', ['rating' => 5])]);
        $reader = $this->view(Role::Reader);
        $mine = $this->view(Role::Reader, samePerson: true);

        self::assertTrue(Permissions::isAllowed($mark(self::MY_MEMBER), $reader));
        self::assertFalse(Permissions::isAllowed($mark(self::OTHER_MEMBER), $this->view(Role::Editor)));
        self::assertTrue(Permissions::isAllowed($mark(self::OTHER_MEMBER), $this->view(Role::Admin)));
        self::assertTrue(Permissions::isAllowed($update, $mine));
        self::assertFalse(Permissions::isAllowed($update, $reader), "someone else's mark");
        self::assertFalse(Permissions::isAllowed($mark(self::MY_MEMBER), $this->view(null)), 'a device of nobody');
        // The owner of a mark is compared without telling upper from lower case.
        self::assertTrue(Permissions::isAllowed($mark(strtoupper(self::MY_MEMBER)), $reader));
    }

    public function testRolesDecideOnlyOnceTheLibraryHasThem(): void
    {
        $delete = $this->op([Ops::delete(CatalogEntity::Tag, self::TAG)]);
        $none = new FakeView(roles: false);

        self::assertTrue(Permissions::isAllowed($delete, $none));
        self::assertFalse(Permissions::isAllowed($delete, $this->view(Role::Reader)));
        self::assertFalse(Permissions::isAllowed($delete, $this->view(Role::Editor)));
        self::assertTrue(Permissions::isAllowed($delete, $this->view(Role::Admin)));
    }

    public function testAnEditorDeletesWhatHerPersonCreated(): void
    {
        $delete = $this->op([Ops::delete(CatalogEntity::Tag, self::TAG)]);

        self::assertTrue(Permissions::isAllowed($delete, $this->view(Role::Editor, samePerson: true)));
    }

    public function testADeviceRegistersItselfButDoesNotSayWhoseItIs(): void
    {
        $register = $this->op([Ops::create(CatalogEntity::Device, self::ME, ['public_key' => 'k', 'name' => 'Phone'])]);
        $claim = $this->op([Ops::update(CatalogEntity::Device, self::ME, ['member_id' => '01997c7e-0000-7000-8000-000000000011'])]);

        self::assertTrue(Permissions::isAllowed($register, $this->view(null)));
        self::assertFalse(Permissions::isAllowed($claim, $this->view(Role::Editor)));
    }

    public function testOnlyAnAdministratorVouchesAndNeverTheCloud(): void
    {
        $other = '01997c7e-0000-7000-8000-0000000000fc';
        $vouch = fn (string $device): OperationPayload => $this->op([Ops::update(CatalogEntity::Device, $device, ['vouched_key' => 'iojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w'])]);

        self::assertFalse(Permissions::isAllowed($vouch(self::ME), $this->view(Role::Editor)), 'its own');
        self::assertFalse(Permissions::isAllowed($vouch($other), $this->view(Role::Editor)));
        self::assertFalse(Permissions::isAllowed($vouch(self::ME), $this->view(null)), 'a device of nobody');
        self::assertTrue(Permissions::isAllowed($vouch($other), $this->view(Role::Admin)));
        $cloud = new FakeView(roles: true, cloud: true);
        $linkAndVouch = $this->op([Ops::update(CatalogEntity::Device, $other, ['member_id' => '01997c7e-0000-7000-8000-000000000013', 'vouched_key' => 'AAAA'])]);
        self::assertFalse(Permissions::isAllowed($vouch($other), $cloud));
        self::assertFalse(Permissions::isAllowed($linkAndVouch, $cloud), 'not beside a link');
    }

    public function testAnUnsignedOperationIsNeverAllowed(): void
    {
        $add = $this->op([Ops::create(CatalogEntity::Tag, self::TAG, ['name' => 'Tehanu'])]);
        self::assertTrue(Permissions::isAllowed($add, $this->view(Role::Editor)));

        $unsigned = $this->op([Ops::create(CatalogEntity::Tag, self::TAG, ['name' => 'Tehanu'])], signed: false);
        self::assertFalse(Permissions::isAllowed($unsigned, $this->view(Role::Admin)));
        self::assertFalse(Permissions::isAllowed($unsigned, $this->view(null)), 'not even without roles');
    }

    public function testWhoeverAskedWithdrawsAndNothingMore(): void
    {
        $request = '01997c7e-0000-7000-8000-000000000077';
        $withdraw = $this->op([Ops::create(CatalogEntity::ChangeRequest, $request, ['status' => 'withdrawn'])]);
        $accept = $this->op([Ops::create(CatalogEntity::ChangeRequest, $request, ['status' => 'accepted'])]);
        $asker = $this->view(Role::Reader, samePerson: true);

        self::assertTrue(Permissions::isAllowed($withdraw, $asker));
        self::assertFalse(Permissions::isAllowed($accept, $asker));
        self::assertFalse(Permissions::isAllowed($withdraw, $this->view(Role::Editor)));
    }

    public function testAFencedDeviceCanOnlyDoWhatADeviceOfNobodyCan(): void
    {
        $delete = $this->op([Ops::delete(CatalogEntity::Tag, self::TAG)]);
        $register = $this->op([Ops::update(CatalogEntity::Device, self::ME, ['name' => 'Phone'])]);
        $fenced = $this->view(Role::Admin, fenced: true);

        self::assertTrue(Permissions::isAllowed($delete, $this->view(Role::Admin)));
        self::assertFalse(Permissions::isAllowed($delete, $fenced));
        self::assertTrue(Permissions::isAllowed($register, $fenced), 'renaming itself is for all');
    }

    public function testADeviceNeverSetsItsOwnFence(): void
    {
        $own = $this->op([Ops::update(CatalogEntity::Device, self::ME, ['trusted_through' => ''])]);
        $other = $this->op([Ops::update(CatalogEntity::Device, '01997c7e-0000-7000-8000-0000000000fc', ['trusted_through' => ''])]);

        self::assertFalse(Permissions::isAllowed($own, $this->view(Role::Editor)));
        self::assertFalse(Permissions::isAllowed($other, $this->view(Role::Editor)));
        self::assertTrue(Permissions::isAllowed($other, $this->view(Role::Admin)));
        self::assertFalse(Permissions::isAllowed($own, $this->view(null)));
    }

    public function testADeviceChangesOnlyItsOwnRow(): void
    {
        $rename = fn (string $id): OperationPayload => $this->op([Ops::update(CatalogEntity::Device, $id, ['name' => 'Tablet'])]);
        $other = '01997c7e-0000-7000-8000-0000000000fc';

        self::assertTrue(Permissions::isAllowed($rename(self::ME), $this->view(Role::Editor)));
        self::assertFalse(Permissions::isAllowed($rename($other), $this->view(Role::Editor)));
        self::assertFalse(Permissions::isAllowed($rename($other), $this->view(Role::Reader)));
        self::assertTrue(Permissions::isAllowed($rename($other), $this->view(Role::Admin)));
    }

    public function testTheCloudDeviceWritesAccountsAndLinksDevicesOfNobodyAndNothingElse(): void
    {
        $other = '01997c7e-0000-7000-8000-0000000000fc';
        $account = $this->op([Ops::update(CatalogEntity::Member, self::OTHER_MEMBER, ['account_id' => '01997c7e-0000-7000-8000-0000000000a1'])]);
        $cloud = $this->cloud(null);
        $reader = $this->view(Role::Reader);

        self::assertTrue(Permissions::isAllowed($account, $cloud));
        self::assertTrue(Permissions::isAllowed($this->link($other, self::MY_MEMBER), $cloud), 'a person with an account');
        self::assertFalse(Permissions::isAllowed($this->link($other, self::OTHER_MEMBER), $cloud), 'a person without one');
        self::assertFalse(Permissions::isAllowed($this->link($other, null), $cloud), 'taking the person away');
        self::assertFalse(Permissions::isAllowed($this->link($other, self::MY_MEMBER, ['trusted_through' => '']), $cloud), 'beside a fence');
        self::assertFalse(Permissions::isAllowed($this->link(self::ME, self::MY_MEMBER), $cloud), 'its own device');
        self::assertFalse(Permissions::isAllowed($this->op([Ops::update(CatalogEntity::Member, self::OTHER_MEMBER, ['role' => 'admin'])]), $cloud));
        self::assertFalse(Permissions::isAllowed($this->op([Ops::create(CatalogEntity::Tag, self::TAG, ['name' => 'Tehanu'])]), $cloud));
        self::assertFalse(Permissions::isAllowed($this->op([Ops::update(CatalogEntity::Member, self::OTHER_MEMBER, ['account_id' => '01997c7e-0000-7000-8000-0000000000a1', 'name' => 'Laura'])]), $cloud), 'account_id and nothing else');
        // Not the cloud device: a reader, and an administrator.
        self::assertFalse(Permissions::isAllowed($account, $reader));
        self::assertFalse(Permissions::isAllowed($account, $this->view(Role::Admin)), 'not even an administrator');
        self::assertFalse(Permissions::isAllowed($this->op([Ops::create(CatalogEntity::Member, self::OTHER_MEMBER, ['name' => 'Laura', 'account_id' => '01997c7e-0000-7000-8000-0000000000a1'])]), $this->view(Role::Admin)), 'nor at creation');
        // Without roles everyone may.
        self::assertTrue(Permissions::isAllowed($account, new FakeView(roles: false)));
    }

    public function testAFencedCloudDeviceHasNoPower(): void
    {
        $account = $this->op([Ops::update(CatalogEntity::Member, self::OTHER_MEMBER, ['account_id' => '01997c7e-0000-7000-8000-0000000000a1'])]);

        self::assertTrue(Permissions::isAllowed($account, $this->cloud(null)));
        self::assertFalse(Permissions::isAllowed($account, new FakeView(roles: true, fenced: true, cloud: true)));
    }

    /**
     * @param list<ChangePayload> $changes
     */
    private function op(array $changes, bool $signed = true): OperationPayload
    {
        $operation = Ops::operation(Ops::hlc(1727222222222, 0, self::ME), $changes);

        return new OperationPayload($operation->id, $operation->library, $operation->hlc, 'test.op', $changes, null, $signed ? 'signed' : null);
    }

    private function view(?Role $role, bool $samePerson = false, bool $fenced = false): FakeView
    {
        return new FakeView(roles: true, role: $role, samePerson: $samePerson, fenced: $fenced);
    }

    /**
     * @param array<string, mixed> $more
     */
    private function link(string $device, ?string $member, array $more = []): OperationPayload
    {
        return $this->op([Ops::update(CatalogEntity::Device, $device, ['member_id' => $member] + $more)]);
    }

    /** The cloud device is a device of nobody, or of the person given. */
    private function cloud(?Role $role): FakeView
    {
        return new FakeView(roles: true, role: $role, cloud: true);
    }
}
