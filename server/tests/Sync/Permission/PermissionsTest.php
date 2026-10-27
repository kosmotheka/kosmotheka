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

    public function testRolesDecideWhatEachDeviceMayDo(): void
    {
        $delete = $this->op([Ops::delete(CatalogEntity::Tag, self::TAG)]);

        self::assertFalse(Permissions::isAllowed($delete, $this->view(Role::Reader)));
        self::assertFalse(Permissions::isAllowed($delete, $this->view(Role::Editor)));
        self::assertTrue(Permissions::isAllowed($delete, $this->view(Role::Admin)));
    }

    private const string FOUNDER_KEY = 'iojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w';
    private const string OTHER_KEY = 'jojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w';
    private const string NONCE = 'AAECAwQFBgcICQoLDA0ODw';
    private const string FOUNDED_ID = '2c33de53-4834-8a05-b014-c42dc60bd4e1';

    /** The operation that founds a library (D97): the device creates its own row with its key, a person who is an administrator and the library row with the nonce. */
    private function founding(string $key, string $nonce, string $library): OperationPayload
    {
        $operation = Ops::operation(Ops::hlc(1727222222222, 0, self::ME), []);
        $changes = [
            Ops::create(CatalogEntity::Device, self::ME, ['public_key' => $key, 'name' => 'laptop', 'member_id' => self::MY_MEMBER, 'vouched_key' => $key]),
            Ops::create(CatalogEntity::Member, self::MY_MEMBER, ['name' => 'Ana', 'role' => 'admin', 'can_share' => true]),
            Ops::create(CatalogEntity::Library, $library, ['name' => 'Casa', 'founding_nonce' => $nonce]),
        ];

        return new OperationPayload($operation->id, $library, $operation->hlc, 'library.found', $changes, null, 'signed');
    }

    public function testTheKeyTheIdDerivesFromFoundsTheLibraryAndNobodyElseDoes(): void
    {
        $unfounded = new FakeView(founded: false);

        self::assertFalse(Permissions::isAllowed($this->founding(self::OTHER_KEY, self::NONCE, self::FOUNDED_ID), $unfounded), 'another key');
        self::assertFalse(Permissions::isAllowed($this->founding(self::FOUNDER_KEY, 'AAECAwQFBgcICQoLDA0ODg', self::FOUNDED_ID), $unfounded), 'another nonce');
        self::assertFalse(Permissions::isAllowed($this->founding(self::FOUNDER_KEY, self::NONCE, '01997c7e-0000-7000-8000-00000000aaaa'), $unfounded), 'an id that is not derived');
        self::assertFalse(Permissions::isAllowed($this->founding(self::FOUNDER_KEY, 'short', self::FOUNDED_ID), $unfounded), 'a nonce that is not 16 bytes');
        self::assertTrue(Permissions::isAllowed($this->founding(self::FOUNDER_KEY, self::NONCE, self::FOUNDED_ID), $unfounded));
    }

    public function testALibraryIsFoundedOnce(): void
    {
        self::assertFalse(Permissions::isAllowed($this->founding(self::FOUNDER_KEY, self::NONCE, self::FOUNDED_ID), $this->view(null)));
        self::assertFalse(Permissions::isAllowed($this->founding(self::FOUNDER_KEY, self::NONCE, self::FOUNDED_ID), $this->view(Role::Editor)));
    }

    public function testTheNonceIsWrittenByTheFoundingAloneEvenForAnAdministrator(): void
    {
        $rewrite = $this->op([Ops::update(CatalogEntity::Library, '01997c7e-0000-7000-8000-00000000aaaa', ['founding_nonce' => 'AAECAwQFBgcICQoLDA0ODg'])]);
        $rename = $this->op([Ops::update(CatalogEntity::Library, '01997c7e-0000-7000-8000-00000000aaaa', ['name' => 'Otra'])]);

        self::assertFalse(Permissions::isAllowed($rewrite, $this->view(Role::Admin)));
        self::assertTrue(Permissions::isAllowed($rename, $this->view(Role::Admin)));
    }

    public function testAFencedFounderFoundsNothing(): void
    {
        self::assertFalse(Permissions::isAllowed($this->founding(self::FOUNDER_KEY, self::NONCE, self::FOUNDED_ID), new FakeView(founded: false, fenced: true)));
    }

    public function testInAnUnfoundedLibraryNobodyWritesButRegistersItsOwnDevice(): void
    {
        $unfounded = new FakeView(founded: false);

        self::assertFalse(Permissions::isAllowed($this->op([Ops::create(CatalogEntity::Tag, self::TAG, ['name' => 'Tehanu'])]), $unfounded));
        self::assertTrue(Permissions::isAllowed($this->op([Ops::create(CatalogEntity::Device, self::ME, ['public_key' => self::FOUNDER_KEY, 'name' => 'laptop'])]), $unfounded));
    }

    public function testAnEditorDeletesWhatHerPersonCreatedButTheTrashThingsOnlyThroughTheTrash(): void
    {
        $mine = $this->view(Role::Editor, samePerson: true);

        // A link is deleted straight, as before the trash; a tag goes to the trash first (D105).
        self::assertTrue(Permissions::isAllowed($this->op([Ops::delete(CatalogEntity::WorkTag, self::TAG)]), $mine));
        self::assertFalse(Permissions::isAllowed($this->op([Ops::delete(CatalogEntity::Tag, self::TAG)]), $mine));
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
        $cloud = new FakeView(founded: true, cloud: true);
        $linkAndVouch = $this->op([Ops::update(CatalogEntity::Device, $other, ['member_id' => '01997c7e-0000-7000-8000-000000000013', 'vouched_key' => 'AAAA'])]);
        self::assertFalse(Permissions::isAllowed($vouch($other), $cloud), 'alone');
        self::assertTrue(Permissions::isAllowed($linkAndVouch, $cloud), 'with the link it makes');
        $linkAlone = $this->op([Ops::update(CatalogEntity::Device, $other, ['member_id' => '01997c7e-0000-7000-8000-000000000013'])]);
        self::assertFalse(Permissions::isAllowed($linkAlone, $cloud), 'member_id alone');
    }

    public function testOnlyAnAdministratorWritesAccessWithdrawn(): void
    {
        // Another person than the one the signing device belongs to: leaving oneself is rule 15, tested apart.
        $member = self::OTHER_MEMBER;
        $withdraw = fn (bool $value): OperationPayload => $this->op([Ops::update(CatalogEntity::Member, $member, ['access_withdrawn' => $value])]);

        foreach ([true, false] as $value) {
            self::assertFalse(Permissions::isAllowed($withdraw($value), $this->view(Role::Editor)), 'an editor');
            self::assertFalse(Permissions::isAllowed($withdraw($value), $this->view(Role::Reader, samePerson: true)), 'a reader');
            self::assertFalse(Permissions::isAllowed($withdraw($value), $this->view(null)), 'a device of nobody');
            self::assertTrue(Permissions::isAllowed($withdraw($value), $this->view(Role::Admin)));
            self::assertFalse(Permissions::isAllowed($withdraw($value), new FakeView(founded: true, cloud: true)), 'the cloud');
        }
        $born = $this->op([Ops::create(CatalogEntity::Member, $member, ['name' => 'Hugo', 'access_withdrawn' => true])]);
        self::assertFalse(Permissions::isAllowed($born, $this->view(Role::Editor)), 'not at creation');
    }

    public function testAPersonLeavesByMarkingHerselfAndNeverTakesTheMarkOffNorMarksAnother(): void
    {
        $mark = fn (string $member, bool $value): OperationPayload => $this->op([Ops::update(CatalogEntity::Member, $member, ['access_withdrawn' => $value])]);

        foreach ([Role::Editor, Role::Reader] as $role) {
            self::assertTrue(Permissions::isAllowed($mark(self::MY_MEMBER, true), $this->view($role)), 'herself, '.$role->value);
            self::assertFalse(Permissions::isAllowed($mark(self::MY_MEMBER, false), $this->view($role)), 'not to take it off');
            self::assertFalse(Permissions::isAllowed($mark(self::OTHER_MEMBER, true), $this->view($role)), 'not another person');
            self::assertFalse(Permissions::isAllowed($mark(self::MY_MEMBER, true), $this->view($role, fenced: true)), 'not a fenced device');
        }
        self::assertFalse(Permissions::isAllowed($mark(self::MY_MEMBER, true), $this->view(null)), 'a device of nobody');
        self::assertFalse(Permissions::isAllowed($this->op([Ops::update(CatalogEntity::Member, self::MY_MEMBER, ['access_withdrawn' => true, 'role' => 'admin'])]), $this->view(Role::Editor)), 'nothing more than the mark');
    }

    public function testAnUnsignedOperationIsNeverAllowed(): void
    {
        $add = $this->op([Ops::create(CatalogEntity::Tag, self::TAG, ['name' => 'Tehanu'])]);
        self::assertTrue(Permissions::isAllowed($add, $this->view(Role::Editor)));

        $unsigned = $this->op([Ops::create(CatalogEntity::Tag, self::TAG, ['name' => 'Tehanu'])], signed: false);
        self::assertFalse(Permissions::isAllowed($unsigned, $this->view(Role::Admin)));
        self::assertFalse(Permissions::isAllowed($unsigned, $this->view(null)), 'nor a device of nobody');
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
        // Nor in a library nobody founded.
        self::assertFalse(Permissions::isAllowed($account, new FakeView(founded: false)));
    }

    public function testAFencedCloudDeviceHasNoPower(): void
    {
        $account = $this->op([Ops::update(CatalogEntity::Member, self::OTHER_MEMBER, ['account_id' => '01997c7e-0000-7000-8000-0000000000a1'])]);

        self::assertTrue(Permissions::isAllowed($account, $this->cloud(null)));
        self::assertFalse(Permissions::isAllowed($account, new FakeView(founded: true, fenced: true, cloud: true)));
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
        return new FakeView(founded: true, role: $role, samePerson: $samePerson, fenced: $fenced);
    }

    /**
     * @param array<string, mixed> $more
     */
    private function link(string $device, ?string $member, array $more = []): OperationPayload
    {
        return $this->op([Ops::update(CatalogEntity::Device, $device, ['member_id' => $member, 'vouched_key' => null === $member ? null : 'k'] + $more)]);
    }

    /** The cloud device is a device of nobody, or of the person given. */
    private function cloud(?Role $role): FakeView
    {
        return new FakeView(founded: true, role: $role, cloud: true);
    }
}
