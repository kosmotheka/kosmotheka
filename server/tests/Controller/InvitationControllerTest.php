<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\DataFixtures\SigningDevice;
use App\Entity\CloudLibrary;
use App\Entity\Invitation;
use App\Entity\Membership;
use App\Entity\User;
use App\Security\AccountException;
use App\Security\AccountPurger;
use App\Security\RequestLimiter;
use App\Sync\Invitation\EmailFingerprint;
use App\Sync\Invitation\InvitationAcceptor;
use App\Sync\Invitation\InvitationManager;
use App\Sync\Invitation\InvitationToken;
use App\Sync\SchemaVersion;
use App\Tests\Support\DeviceAuthTestTrait;
use App\Tests\Support\RecordingHub;
use Doctrine\DBAL\Connection;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\Lock\LockFactory;
use Symfony\Component\Mercure\Update;
use Symfony\Component\Mime\Email;
use Symfony\Component\Uid\Uuid;

/**
 * The invitations of milestone 7.6.1 (D43, docs/architecture/cloud.md) and their ten hostile cases
 * (docs/plans/fase-7.md, «Casos hostiles de las invitaciones»). The cast: a family whose administrator is Laura
 * (the `family` account, with the cloud's power in the library), an editor with an account (Hugo), another
 * administrator (Marta), a new reader (Nora), and Jhonny, hostile, with an account and a library of his own.
 */
final class InvitationControllerTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private const int T = 1_727_000_000_000;
    private const string PASSWORD = 'correct horse battery';

    private const string LAURA = '01997c7e-0000-7000-8000-000000000011';
    private const string HUGO = '01997c7e-0000-7000-8000-000000000012';
    private const string EVA = '01997c7e-0000-7000-8000-000000000013';
    private const string READER = '01997c7e-0000-7000-8000-000000000014';
    private const string NORA = '01997c7e-0000-7000-8000-000000000015';
    private const string TOMAS = '01997c7e-0000-7000-8000-000000000016';
    private const string MARTA = '01997c7e-0000-7000-8000-000000000017';
    private const string OLGA = '01997c7e-0000-7000-8000-000000000018';

    private KernelBrowser $client;
    private int $time = 100;

    /** @var array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string} */
    private array $family;

    /** @var array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string} */
    private array $jhonny;

    private SigningDevice $laptop;

    /** The library the laptop founds: its id derives from the laptop's key (D97). */
    private string $local;

    protected function setUp(): void
    {
        $this->client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();
        RecordingHub::reset();
        $this->family = $this->signInNewDevice($this->client, 'family@example.com');
        $this->jhonny = $this->signInNewDevice($this->client, 'jhonny@example.com');
        $this->laptop = SigningDevice::fromSession($this->family);
        $this->local = $this->laptop->libraryId();
    }

    // ------------------------------------------------------ normal ---

    public function testALinkForANewReaderIsAcceptedOnceAndTheAccountIsThatPerson(): void
    {
        $cloud = $this->world();
        $nora = $this->account('nora@example.com');

        $created = $this->invite($this->family, $cloud, self::NORA, 'link');

        self::assertSame(201, $created['status']);
        self::assertMatchesRegularExpression('~^https://invitations\.test/i/[A-Za-z0-9_-]{43}$~', \is_string($created['body']['link'] ?? null) ? $created['body']['link'] : '');
        $token = $this->tokenOf($created);
        self::assertSame(200, $this->peek($token)['status']);

        $accepted = $this->accept($nora, $token);

        self::assertSame(200, $accepted['status']);
        self::assertSame($cloud, $accepted['body']['id'] ?? null);
        self::assertSame(self::NORA, $this->memberOf($cloud, 'nora@example.com'));
        self::assertSame($this->accountIdOfUser('nora@example.com'), $this->accountIdOf(self::NORA), 'the cloud wrote the account on the person');
        self::assertSame([$cloud], array_column($this->libraryList($this->call('GET', '/v1/cloud/libraries', $nora)['body']), 'id'));
        self::assertSame(410, $this->peek($token)['status']);
    }

    public function testAnEmailInvitationIsMailedAndKeepsTheRoleOfAPersonAlreadyInTheLibrary(): void
    {
        $cloud = $this->world();
        $eva = $this->account('eva@example.com');

        $created = $this->invite($this->family, $cloud, self::EVA, 'email', 'Eva@Example.com');

        self::assertSame(201, $created['status']);
        self::assertNull($created['body']['link'], 'the link only goes in the email');
        $accepted = $this->accept($eva, $this->tokenInMail());
        self::assertSame(200, $accepted['status']);
        self::assertSame(self::EVA, $this->memberOf($cloud, 'eva@example.com'));
        self::assertSame('editor', $this->scalarString('SELECT role FROM catalog_members WHERE id = ? AND cloud_library_id = ?', [self::EVA, $cloud]), 'the role is the person\'s: the invitation gives none');
    }

    public function testTheListShowsTheStateOfEachInvitationToWhoeverAdministersAndNeverTheTokenOrTheEmail(): void
    {
        $cloud = $this->world();
        $this->account('nora@example.com');
        $pending = $this->invite($this->family, $cloud, self::NORA, 'link');
        $this->invite($this->family, $cloud, self::EVA, 'email', 'eva@example.com');

        $list = $this->call('GET', "/v1/cloud/libraries/$cloud/invitations", $this->family);

        self::assertSame(200, $list['status']);
        self::assertCount(2, $this->rowsOf($list['body'], 'invitations'));
        $encoded = json_encode($list['body'], \JSON_THROW_ON_ERROR);
        self::assertStringNotContainsString($this->tokenOf($pending), $encoded);
        self::assertStringNotContainsString('eva@example.com', $encoded);
        self::assertSame(['pending', 'pending'], array_column($this->rowsOf($list['body'], 'invitations'), 'state'));
    }

    public function testAdministratorsRevokeAnInvitationAndItStopsWorking(): void
    {
        $cloud = $this->world();
        $nora = $this->account('nora@example.com');
        $created = $this->invite($this->family, $cloud, self::NORA, 'link');
        $id = $this->idOf($created);

        self::assertSame(204, $this->call('DELETE', "/v1/cloud/libraries/$cloud/invitations/$id", $this->family)['status']);

        self::assertSame(['error' => 'invitation_revoked'], $this->accept($nora, $this->tokenOf($created))['body']);
    }

    /** SEG-043: someone accepts between the moment revoking reads the row and the moment it writes; the acceptance must win. */
    public function testRevokingReadsTheRowAgainUnderTheLockAndNeverTurnsAnAcceptedInvitationIntoARevokedOne(): void
    {
        $cloud = $this->world();
        $this->account('nora@example.com');
        $created = $this->invite($this->family, $cloud, self::NORA, 'link');
        $em = self::getContainer()->get(EntityManagerInterface::class);
        $invitation = $em->getRepository(Invitation::class)->findOneBy([]) ?? self::fail('No invitation.');
        $admin = $em->getRepository(User::class)->findOneBy(['email' => 'family@example.com']) ?? self::fail('No administrator.');
        $library = $invitation->getCloudLibrary();
        // The other process accepted and committed after this one had read the row (stale in the identity map).
        self::getContainer()->get(Connection::class)->executeStatement('UPDATE invitation SET accepted_at = now() WHERE id = ?', [$invitation->getId()->toRfc4122()]);
        self::assertFalse($invitation->isAccepted(), 'the entity in memory is stale');
        self::assertSame(201, $created['status']);

        try {
            self::getContainer()->get(InvitationManager::class)->revoke($library, $admin, $invitation->getId()->toRfc4122());
            self::fail('Revoking an accepted invitation must fail.');
        } catch (AccountException $exception) {
            self::assertSame('invitation_used', $exception->errorCode->value);
        }
        self::assertSame('accepted', $this->stateOfOnlyInvitation());
        self::assertSame(0, $this->scalar("SELECT COUNT(*) FROM activity_log WHERE kind = 'invitation_revoked'"));
    }

    /** SEG-047: a link is for whoever opens it; an app that sends an address with it made a mistake, and must hear it. */
    public function testALinkWithAnEmailInTheBodyIsRefusedInsteadOfIgnoringTheAddress(): void
    {
        $cloud = $this->world();

        $answer = $this->invite($this->family, $cloud, self::NORA, 'link', 'someone@example.com');

        self::assertSame(['error' => 'invalid_request'], $answer['body']);
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM invitation'));
    }

    public function testThePeekShowsTheLibraryNameAndNothingOfThePeopleInIt(): void
    {
        $cloud = $this->world();
        $created = $this->invite($this->family, $cloud, self::NORA, 'link');

        $peek = $this->peek($this->tokenOf($created));

        self::assertSame(200, $peek['status']);
        self::assertSame(['expiresAt', 'kind', 'libraryName'], $this->sorted(array_keys($peek['body'])));
        self::assertSame('Family', $peek['body']['libraryName']);
        self::assertSame('link', $peek['body']['kind']);
    }

    public function testAcceptingTellsTheInvitingAdministratorTheAcceptorAndTheLibrary(): void
    {
        $cloud = $this->world();
        $nora = $this->account('nora@example.com');
        $created = $this->invite($this->family, $cloud, self::NORA, 'link');
        RecordingHub::reset();

        $this->accept($nora, $this->tokenOf($created));

        $topics = array_map(static fn (Update $update): array => $update->getTopics(), RecordingHub::$updates);
        $flat = array_merge(...$topics);
        self::assertContains('urn:kosmotheka:account:'.$this->userId('family@example.com'), $flat, 'the administrator who invited');
        self::assertContains('urn:kosmotheka:account:'.$this->userId('nora@example.com'), $flat, 'the account that came in');
        $inviter = array_values(array_filter(RecordingHub::$updates, fn (Update $update): bool => ['urn:kosmotheka:account:'.$this->userId('family@example.com')] === $update->getTopics()));
        self::assertSame(['event' => 'invitation_accepted', 'library' => $cloud], json_decode($inviter[0]->getData(), true));
    }

    public function testOnlyTheLinkRouteNeedsNoSession(): void
    {
        $cloud = $this->world();
        $created = $this->invite($this->family, $cloud, self::NORA, 'link');
        $token = $this->tokenOf($created);

        self::assertSame(200, $this->peek($token)['status']);
        self::assertSame(401, $this->call('POST', "/v1/cloud/invitations/$token/accept")['status']);
        self::assertSame(401, $this->call('GET', "/v1/cloud/libraries/$cloud/invitations")['status']);
    }

    // ------------------------------------------------------ hostile ---

    /** 1 · used, expired or revoked gives nothing; two accounts at once: one gets in. */
    public function testAUsedInvitationGivesNothingToASecondAccount(): void
    {
        $cloud = $this->world();
        $first = $this->account('nora@example.com');
        $second = $this->account('mallory@example.com');
        $token = $this->tokenOf($this->invite($this->family, $cloud, self::NORA, 'link'));

        self::assertSame(200, $this->accept($first, $token)['status']);

        self::assertSame(['error' => 'invitation_used'], $this->accept($second, $token)['body']);
        self::assertNull($this->memberOf($cloud, 'mallory@example.com'));
        self::assertSame(404, $this->call('POST', "/v1/cloud/libraries/$cloud/pull", $second, ['schemaVersion' => $this->schema(), 'vector' => new \stdClass()])['status']);
    }

    public function testAnExpiredInvitationGivesNothing(): void
    {
        $cloud = $this->world();
        $nora = $this->account('nora@example.com');
        $token = $this->tokenOf($this->invite($this->family, $cloud, self::NORA, 'link'));
        self::getContainer()->get(Connection::class)->executeStatement("UPDATE invitation SET expires_at = now() - interval '1 minute'");

        self::assertSame(['error' => 'invitation_expired'], $this->peek($token)['body']);
        self::assertSame(['error' => 'invitation_expired'], $this->accept($nora, $token)['body']);
        self::assertNull($this->memberOf($cloud, 'nora@example.com'));
    }

    public function testARevokedInvitationGivesNothing(): void
    {
        $cloud = $this->world();
        $nora = $this->account('nora@example.com');
        $created = $this->invite($this->family, $cloud, self::NORA, 'link');
        $this->call('DELETE', "/v1/cloud/libraries/$cloud/invitations/".$this->idOf($created), $this->family);

        self::assertSame(['error' => 'invitation_revoked'], $this->peek($this->tokenOf($created))['body']);
        self::assertSame(['error' => 'invitation_revoked'], $this->accept($nora, $this->tokenOf($created))['body']);
        self::assertNull($this->memberOf($cloud, 'nora@example.com'));
    }

    /** The race: another process wins between the read and the lock. The loser reads again under the lock and gets «used». */
    public function testWhoLosesTheRaceForAnInvitationReadsItAgainUnderTheLockAndGetsNothing(): void
    {
        $cloud = $this->world();
        $this->account('nora@example.com');
        $second = $this->account('mallory@example.com');
        $token = $this->tokenOf($this->invite($this->family, $cloud, self::NORA, 'link'));
        $em = self::getContainer()->get(EntityManagerInterface::class);
        $invitation = $em->getRepository(Invitation::class)->findOneBy([]) ?? self::fail('No invitation.');
        $mallory = $em->getRepository(User::class)->findOneBy(['email' => 'mallory@example.com']) ?? self::fail('No Mallory.');
        // The other process: accepted and committed after this one had read the row (stale in the identity map).
        self::getContainer()->get(Connection::class)->executeStatement('UPDATE invitation SET accepted_at = now() WHERE id = ?', [$invitation->getId()->toRfc4122()]);
        self::assertFalse($invitation->isAccepted(), 'the entity in memory is stale');

        $this->expectException(AccountException::class);
        $this->expectExceptionMessage('invitation_used');
        try {
            self::getContainer()->get(InvitationAcceptor::class)->accept($token, $mallory);
        } finally {
            self::assertNull($this->memberOf($cloud, 'mallory@example.com'));
            unset($second);
        }
    }

    /** 2 · an invitation by email, opened by an account with another email. */
    public function testAnEmailInvitationOpenedByAnAccountWithAnotherEmailGivesNothing(): void
    {
        $cloud = $this->world();
        $this->account('eva@example.com');
        $mallory = $this->account('mallory@example.com');
        $this->invite($this->family, $cloud, self::EVA, 'email', 'eva@example.com');
        $token = $this->tokenInMail();

        self::assertSame(['error' => 'invitation_not_for_this_account'], $this->accept($mallory, $token)['body']);

        self::assertNull($this->memberOf($cloud, 'mallory@example.com'));
        self::assertSame('pending', $this->stateOfOnlyInvitation(), 'the invitation is still usable by the right account');
        self::assertSame(200, $this->accept($this->login('eva@example.com'), $token)['status']);
    }

    public function testTheServerKeepsAFingerprintOfTheEmailAndOfTheTokenNeverThemselves(): void
    {
        $cloud = $this->world();
        $this->invite($this->family, $cloud, self::EVA, 'email', 'eva@example.com');
        $token = $this->tokenInMail();

        $row = self::getContainer()->get(Connection::class)->fetchAssociative('SELECT * FROM invitation');
        \assert(\is_array($row));
        $dump = json_encode($row, \JSON_THROW_ON_ERROR);
        self::assertStringNotContainsString($token, $dump);
        self::assertStringNotContainsString('eva@example.com', $dump);
        self::assertSame(hash('sha256', $token), $row['token_hash']);
        self::assertNotSame(hash('sha256', 'eva@example.com'), $row['email_hash'], 'keyed with a secret of the server, not a plain hash');
    }

    /** 3 · who does not administer, with a modified app. */
    public function testWhoDoesNotAdministerCannotInvite(): void
    {
        $cloud = $this->world();
        $hugo = $this->joinAs('hugo@example.com', self::HUGO);
        $this->shareWith($cloud, 'jhonny@example.com', self::READER);

        foreach ([['hugo editor with «can share»', $hugo], ['a reader', $this->jhonny]] as [$who, $session]) {
            $answer = $this->invite($session, $cloud, self::NORA, 'link');
            self::assertSame(403, $answer['status'], $who);
            self::assertSame(['error' => 'invitation_forbidden'], $answer['body'], $who);
            self::assertSame(403, $this->call('GET', "/v1/cloud/libraries/$cloud/invitations", $session)['status'], $who);
        }
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM invitation'));
    }

    public function testWhoIsNotInTheLibraryAtAllGetsTheSameNotFoundAsALibraryThatDoesNotExist(): void
    {
        $cloud = $this->world();

        $real = $this->invite($this->jhonny, $cloud, self::NORA, 'link');
        $none = $this->invite($this->jhonny, Uuid::v4()->toRfc4122(), self::NORA, 'link');

        self::assertSame(404, $real['status']);
        self::assertSame($none, $real);
    }

    /** 4 · administers library A, invites with the id of a person of library B: the server compares, it does not look the id up. */
    public function testAnAdministratorOfAnotherLibraryCannotInviteForAPersonOfThisOne(): void
    {
        $cloudA = $this->world();
        $cloudB = $this->jhonnysLibrary();

        // Jhonny knows the id of Nora in the family's library: it does not exist in his.
        $answer = $this->invite($this->jhonny, $cloudB, self::NORA, 'link');
        self::assertSame(422, $answer['status']);
        self::assertSame(['error' => 'invitation_person_unsuitable'], $answer['body']);
        // And the family's library is not his to invite to.
        self::assertSame(404, $this->invite($this->jhonny, $cloudA, self::NORA, 'link')['status']);
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM invitation'));
    }

    public function testAnInvitationOfALibraryOnlyGivesAccessToThatLibraryWhateverTheOtherOneHoldsWithTheSameIds(): void
    {
        $cloudA = $this->world();
        $cloudB = $this->jhonnysLibrary(withNora: true);
        $nora = $this->account('nora@example.com');
        $token = $this->tokenOf($this->invite($this->jhonny, $cloudB, self::NORA, 'link'));

        $this->accept($nora, $token);

        self::assertSame(self::NORA, $this->memberOf($cloudB, 'nora@example.com'));
        self::assertNull($this->memberOf($cloudA, 'nora@example.com'), 'the same person id in the family\'s library gives nothing');
        self::assertNull($this->accountIdOf(self::NORA, $cloudA));
    }

    public function testAnInvitationIsRevokedOnlyInTheLibraryItBelongsTo(): void
    {
        $cloudA = $this->world();
        $cloudB = $this->jhonnysLibrary();
        $created = $this->invite($this->family, $cloudA, self::NORA, 'link');

        // Jhonny administers B and knows the id of the invitation of A.
        self::assertSame(404, $this->call('DELETE', "/v1/cloud/libraries/$cloudB/invitations/".$this->idOf($created), $this->jhonny)['status']);

        self::assertSame('pending', $this->stateOfOnlyInvitation());
    }

    /** 5 · a link for who is not new or not a reader (a modified app): refused when made and when accepted. */
    public function testALinkForAPersonWhoIsNotANewReaderIsRefusedWhenMade(): void
    {
        $cloud = $this->world();

        foreach ([['an editor', self::EVA], ['a reader who already has a device', self::READER]] as [$who, $member]) {
            $answer = $this->invite($this->family, $cloud, $member, 'link');
            self::assertSame(['error' => 'invitation_person_unsuitable'], $answer['body'], $who);
        }
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM invitation'));
    }

    public function testALinkIsRefusedWhenAcceptedIfThePersonStoppedBeingANewReaderMeanwhile(): void
    {
        $cloud = $this->world();
        $mallory = $this->account('mallory@example.com');
        $token = $this->tokenOf($this->invite($this->family, $cloud, self::NORA, 'link'));
        // Nora becomes an editor after the link was made (or got a device): Mallory must not take her place.
        $this->laptopSigns('member.role', [['entity' => 'member', 'id' => self::NORA, 'action' => 'update', 'fields' => ['role' => 'editor']]]);

        $answer = $this->accept($mallory, $token);

        self::assertSame(410, $answer['status']);
        self::assertNull($this->memberOf($cloud, 'mallory@example.com'));
    }

    public function testALinkIsRefusedWhenAcceptedIfThePersonGotADeviceMeanwhile(): void
    {
        $cloud = $this->world();
        $mallory = $this->account('mallory@example.com');
        $token = $this->tokenOf($this->invite($this->family, $cloud, self::NORA, 'link'));
        $phone = SigningDevice::generate();
        $this->push($this->family, $cloud, [$phone->registration($this->local, self::T + $this->tick())]);
        $this->laptopSigns('device.assign', [['entity' => 'device', 'id' => $phone->id, 'action' => 'update', 'fields' => ['member_id' => self::NORA]]]);

        $answer = $this->accept($mallory, $token);

        self::assertSame(410, $answer['status']);
        self::assertNull($this->memberOf($cloud, 'mallory@example.com'));
    }

    /** 6 · for a person who administers, who has the access withdrawn or who already has an account. */
    public function testAnInvitationForAPersonWhoAdministersHasTheAccessWithdrawnOrAlreadyHasAnAccountIsRefusedWhenMade(): void
    {
        $cloud = $this->world();
        $this->joinAs('hugo@example.com', self::HUGO);

        foreach ([['an administrator', self::TOMAS], ['the access withdrawn', self::OLGA], ['an account', self::HUGO], ['a person that does not exist', Uuid::v4()->toRfc4122()]] as [$who, $member]) {
            foreach (['link', 'email'] as $kind) {
                $answer = $this->invite($this->family, $cloud, $member, $kind, 'someone@example.com');
                self::assertSame(['error' => 'invitation_person_unsuitable'], $answer['body'], "$who ($kind)");
            }
        }
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM invitation'));
    }

    public function testAnInvitationIsRefusedWhenAcceptedIfThePersonBecameAnAdministratorMeanwhile(): void
    {
        $cloud = $this->world();
        $eva = $this->account('eva@example.com');
        $this->invite($this->family, $cloud, self::EVA, 'email', 'eva@example.com');
        $token = $this->tokenInMail();
        $this->laptopSigns('member.role', [['entity' => 'member', 'id' => self::EVA, 'action' => 'update', 'fields' => ['role' => 'admin']]]);

        self::assertSame(410, $this->accept($eva, $token)['status']);
        self::assertNull($this->memberOf($cloud, 'eva@example.com'));
    }

    public function testAnInvitationIsRefusedWhenAcceptedIfThePersonHasAnAccountMeanwhile(): void
    {
        $cloud = $this->world();
        $eva = $this->account('eva@example.com');
        $this->invite($this->family, $cloud, self::EVA, 'email', 'eva@example.com');
        $token = $this->tokenInMail();
        // Somebody else's account becomes Eva by another way (a device with a vouched key).
        $this->joinAs('eva2@example.com', self::EVA);

        self::assertSame(410, $this->accept($eva, $token)['status']);
        self::assertNull($this->memberOf($cloud, 'eva@example.com'));
        self::assertSame(self::EVA, $this->memberOf($cloud, 'eva2@example.com'));
    }

    public function testAnInvitationIsRefusedWhenAcceptedIfThePersonHasTheAccessWithdrawnMeanwhile(): void
    {
        $cloud = $this->world();
        $nora = $this->account('nora@example.com');
        $token = $this->tokenOf($this->invite($this->family, $cloud, self::NORA, 'link'));
        $this->laptopSigns('member.withdraw', [['entity' => 'member', 'id' => self::NORA, 'action' => 'update', 'fields' => ['access_withdrawn' => true]]]);

        self::assertSame(410, $this->accept($nora, $token)['status']);
        self::assertNull($this->memberOf($cloud, 'nora@example.com'));
    }

    /** An account thrown out of the library does not come back by a link to copy, only by an email meant for it (D43, D98). */
    public function testAnAccountThatWasThrownOutDoesNotComeBackByALinkToCopyOnlyByAnEmailMeantForIt(): void
    {
        $cloud = $this->world();
        $marta = $this->joinAs('marta@example.com', self::MARTA);
        $this->laptopSigns('member.withdraw', [['entity' => 'member', 'id' => self::MARTA, 'action' => 'update', 'fields' => ['access_withdrawn' => true]]]);
        // The cloud cuts the account the next time it asks for anything.
        $this->call('GET', '/v1/cloud/libraries', $marta);
        self::assertNull($this->memberOf($cloud, 'marta@example.com'));
        $link = $this->tokenOf($this->invite($this->family, $cloud, self::NORA, 'link'));

        self::assertSame(['error' => 'invitation_not_for_this_account'], $this->accept($marta, $link)['body']);
        self::assertNull($this->memberOf($cloud, 'marta@example.com'));
        // The link is not spent: whoever it was for can still use it.
        self::assertSame(200, $this->accept($this->account('nora@example.com'), $link)['status']);

        $this->invite($this->family, $cloud, self::EVA, 'email', 'marta@example.com');
        self::assertSame(200, $this->accept($marta, $this->tokenInMail())['status']);
        self::assertSame(self::EVA, $this->memberOf($cloud, 'marta@example.com'));
    }

    /** 7 · who invited stops administering, or is thrown out, before it is accepted. */
    public function testAnInvitationDiesWithTheAdministratorWhoMadeItWhenTheyAreDemoted(): void
    {
        $cloud = $this->world();
        $marta = $this->joinAs('marta@example.com', self::MARTA);
        $nora = $this->account('nora@example.com');
        $token = $this->tokenOf($this->invite($marta, $cloud, self::NORA, 'link'));
        $this->laptopSigns('member.role', [['entity' => 'member', 'id' => self::MARTA, 'action' => 'update', 'fields' => ['role' => 'reader']]]);

        self::assertSame(['error' => 'invitation_revoked'], $this->peek($token)['body']);
        self::assertSame(['error' => 'invitation_revoked'], $this->accept($nora, $token)['body']);
        self::assertNull($this->memberOf($cloud, 'nora@example.com'));
    }

    public function testAnInvitationDiesWithTheAdministratorWhoMadeItWhenTheyAreThrownOut(): void
    {
        $cloud = $this->world();
        $marta = $this->joinAs('marta@example.com', self::MARTA);
        $nora = $this->account('nora@example.com');
        $token = $this->tokenOf($this->invite($marta, $cloud, self::NORA, 'link'));
        $this->laptopSigns('member.withdraw', [['entity' => 'member', 'id' => self::MARTA, 'action' => 'update', 'fields' => ['access_withdrawn' => true]]]);

        self::assertSame(['error' => 'invitation_revoked'], $this->accept($nora, $token)['body']);
        self::assertNull($this->memberOf($cloud, 'nora@example.com'));
    }

    public function testAnInvitationDiesIfTheLibraryLosesItsRolesOrTheCloudLosesThePowerToLinkDevices(): void
    {
        $cloud = $this->world();
        $nora = $this->account('nora@example.com');
        $token = $this->tokenOf($this->invite($this->family, $cloud, self::NORA, 'link'));
        $this->laptopSigns('library.cloud', [['entity' => 'library', 'id' => $this->local, 'action' => 'update', 'fields' => ['cloud_device_id' => '']]]);

        self::assertSame(['error' => 'invitation_revoked'], $this->accept($nora, $token)['body']);
        self::assertNull($this->memberOf($cloud, 'nora@example.com'));
    }

    public function testWhoDoesNotAdministerCannotRevokeAnInvitationEither(): void
    {
        $cloud = $this->world();
        $hugo = $this->joinAs('hugo@example.com', self::HUGO);
        $created = $this->invite($this->family, $cloud, self::NORA, 'link');

        self::assertSame(403, $this->call('DELETE', "/v1/cloud/libraries/$cloud/invitations/".$this->idOf($created), $hugo)['status']);
        self::assertSame('pending', $this->stateOfOnlyInvitation());
    }

    /** Each way a person can already have an account is enough on its own. */
    public function testAPersonWithAnAccountOnlyInTheCatalogOrOnlyInTheAccessTableIsNotInvited(): void
    {
        $cloud = $this->world();
        $connection = self::getContainer()->get(Connection::class);
        $connection->executeStatement('UPDATE catalog_members SET account_id = ? WHERE id = ?', [Uuid::v4()->toRfc4122(), self::EVA]);
        $this->shareWith($cloud, 'jhonny@example.com', self::NORA);

        self::assertSame(['error' => 'invitation_person_unsuitable'], $this->invite($this->family, $cloud, self::EVA, 'email', 'eva@example.com')['body'], 'account in the catalog');
        self::assertSame(['error' => 'invitation_person_unsuitable'], $this->invite($this->family, $cloud, self::NORA, 'email', 'nora@example.com')['body'], 'account in the access table');
    }

    public function testAnInvitationDiesIfTheCatalogSaysTheAdministratorWhoMadeItHasTheAccessWithdrawn(): void
    {
        $cloud = $this->world();
        $marta = $this->joinAs('marta@example.com', self::MARTA);
        $nora = $this->account('nora@example.com');
        $token = $this->tokenOf($this->invite($marta, $cloud, self::NORA, 'link'));
        self::getContainer()->get(Connection::class)->executeStatement('UPDATE catalog_members SET access_withdrawn = true WHERE id = ?', [self::MARTA]);

        self::assertSame(['error' => 'invitation_revoked'], $this->accept($nora, $token)['body']);
    }

    /** 8 · an account that is already another person in that library accepts one. */
    public function testAnAccountThatIsAlreadyAnotherPersonInTheLibraryCannotAcceptAnInvitation(): void
    {
        $cloud = $this->world();
        $hugo = $this->joinAs('hugo@example.com', self::HUGO);
        $token = $this->tokenOf($this->invite($this->family, $cloud, self::NORA, 'link'));

        $answer = $this->accept($hugo, $token);

        self::assertSame(['error' => 'invitation_account_already_in_library'], $answer['body']);
        self::assertSame(self::HUGO, $this->memberOf($cloud, 'hugo@example.com'), 'Hugo stays Hugo, with his role');
        self::assertSame('pending', $this->stateOfOnlyInvitation(), 'and the invitation is not spent');
    }

    public function testTheAdministratorWhoInvitedCannotAcceptHerOwnInvitationAsAnotherPerson(): void
    {
        $cloud = $this->world();
        $token = $this->tokenOf($this->invite($this->family, $cloud, self::NORA, 'link'));

        self::assertSame(['error' => 'invitation_account_already_in_library'], $this->accept($this->family, $token)['body']);
        self::assertSame(self::LAURA, $this->memberOf($cloud, 'family@example.com'));
    }

    /** 9 · guessing links. */
    public function testTokensAreLongRandomAndAnUnknownOneIsAPlainNotFound(): void
    {
        $cloud = $this->world();
        $one = $this->tokenOf($this->invite($this->family, $cloud, self::NORA, 'link'));
        $two = $this->tokenOf($this->invite($this->family, $cloud, self::NORA, 'link'));

        self::assertNotSame($one, $two);
        self::assertSame(43, \strlen($one), '256 bits in URL-safe base64');
        $nora = $this->account('nora@example.com');
        foreach ([strrev($one), str_repeat('a', 43), 'short', str_repeat('a', 500), '%00', $one.'x'] as $guess) {
            self::assertSame(['status' => 404, 'body' => ['error' => 'not_found']], $this->peek($guess), $guess);
            self::assertSame(['status' => 404, 'body' => ['error' => 'not_found']], $this->accept($nora, $guess), $guess);
        }
    }

    public function testGuessingFromOneAddressIsStoppedByTheRateLimit(): void
    {
        $statuses = [];
        for ($i = 0; $i < 40; ++$i) {
            $statuses[] = $this->peek(InvitationTokenForTests::random())['status'];
        }

        self::assertContains(429, $statuses);
        self::assertSame(404, $statuses[0]);
        self::assertLessThanOrEqual(31, array_search(429, $statuses, true));
    }

    public function testAcceptingIsLimitedPerAccount(): void
    {
        $nora = $this->account('nora@example.com');
        $statuses = [];
        for ($i = 0; $i < 14; ++$i) {
            $statuses[] = $this->accept($nora, InvitationTokenForTests::random())['status'];
        }

        self::assertSame(404, $statuses[0]);
        self::assertSame(429, $statuses[13]);
    }

    /** 10 · using the server to send mail to addresses that are not the sender's. */
    public function testEmailInvitationsAreLimitedPerAccountAndDay(): void
    {
        $cloud = $this->world();
        $statuses = [];
        foreach (range(1, 12) as $i) {
            $statuses[] = $this->invite($this->family, $cloud, self::EVA, 'email', "victim$i@example.com")['status'];
        }

        self::assertSame(array_fill(0, 10, 201), \array_slice($statuses, 0, 10));
        self::assertSame([429, 429], \array_slice($statuses, 10));
    }

    public function testOneAddressCannotBeFloodedEvenFromSeveralAccounts(): void
    {
        $cloud = $this->world();
        $marta = $this->joinAs('marta@example.com', self::MARTA);
        $statuses = [];
        foreach ([$this->family, $marta, $this->family, $marta, $this->family] as $session) {
            $statuses[] = $this->invite($session, $cloud, self::EVA, 'email', 'victim@example.com')['status'];
        }

        self::assertSame([201, 201, 201, 429, 429], $statuses);
    }

    /** SEG-044: three a day per library and address; another library\'s use of an address does not use up this one\'s. */
    public function testAStrangerFromAnotherLibraryCannotUseUpWhatTheFamilyMayMailAnAddress(): void
    {
        $cloud = $this->world();
        $theirs = $this->jhonnysLibrary(true);
        $statuses = [];
        foreach (range(1, 5) as $ignored) {
            $statuses[] = $this->invite($this->jhonny, $theirs, self::NORA, 'email', 'laura@example.com')['status'];
        }
        self::assertSame([201, 201, 201, 429, 429], $statuses, 'three a day to one address from one library');

        $family = $this->invite($this->family, $cloud, self::EVA, 'email', 'laura@example.com');

        self::assertSame(201, $family['status'], 'the family can still invite her');
    }

    /**
     * SEG-054: another invitation to the same address is between looking at the limits and spending them (its mail is
     * on its way). This one must wait its turn, not pass the same look: else twenty at once all get through.
     */
    public function testAnInvitationToAnAddressThatIsBeingMailedRightNowIsRefusedAndMailsNothing(): void
    {
        $cloud = $this->world();
        $fingerprint = self::getContainer()->get(EmailFingerprint::class)->of('victim@example.com');
        $other = self::getContainer()->get(LockFactory::class)->createLock(InvitationManager::mailLockKey($fingerprint));
        self::assertTrue($other->acquire());

        try {
            $refused = $this->invite($this->family, $cloud, self::EVA, 'email', 'victim@example.com');
        } finally {
            $other->release();
        }

        self::assertSame(429, $refused['status']);
        self::assertEmailCount(0);
        self::assertSame(201, $this->invite($this->family, $cloud, self::EVA, 'email', 'victim@example.com')['status'], 'once the other is done, it goes');
    }

    /** SEG-044: ten a day to an address in all, whatever the library; checked before anything is spent. */
    public function testNobodyMailsOneAddressMoreThanTenTimesADayInAllAndRefusingSpendsNothing(): void
    {
        $limiter = self::getContainer()->get(RequestLimiter::class);
        $fingerprint = self::getContainer()->get(EmailFingerprint::class)->of('laura@example.com');
        for ($i = 1; $i <= 10; ++$i) {
            $limiter->requireInvitationEmailOpen(Uuid::v4()->toRfc4122(), Uuid::v4()->toRfc4122(), $fingerprint);
            $limiter->spendInvitationEmail(Uuid::v4()->toRfc4122(), Uuid::v4()->toRfc4122(), $fingerprint);
        }

        $account = Uuid::v4()->toRfc4122();
        $library = Uuid::v4()->toRfc4122();
        try {
            $limiter->requireInvitationEmailOpen($account, $library, $fingerprint);
            self::fail('The eleventh to one address must be refused.');
        } catch (AccountException $exception) {
            self::assertSame('too_many_requests', $exception->errorCode->value);
        }
        // Refused, so nothing of this account nor of this pair was spent: another address is open to both.
        $other = self::getContainer()->get(EmailFingerprint::class)->of('other@example.com');
        for ($i = 1; $i <= 3; ++$i) {
            $limiter->requireInvitationEmailOpen($account, $library, $other);
            $limiter->spendInvitationEmail($account, $library, $other);
        }
        $this->expectException(AccountException::class);
        $limiter->requireInvitationEmailOpen($account, $library, $other);
    }

    /** SEG-044: an email that is not sent (here, one the mailer refuses) spends nothing of the account\'s ten. */
    public function testAnEmailThatIsNotSentSpendsNoQuota(): void
    {
        $cloud = $this->world();
        $this->client->catchExceptions(false);
        $failures = 0;
        foreach (range(1, 6) as $ignored) {
            try {
                $this->invite($this->family, $cloud, self::EVA, 'email', 'broken.@example.com');
            } catch (\Throwable) {
                ++$failures;
            }
        }
        self::assertSame(6, $failures, 'the mailer refuses that address');
        $this->client->catchExceptions(true);

        $statuses = [];
        foreach (range(1, 11) as $i) {
            $statuses[] = $this->invite($this->family, $cloud, self::EVA, 'email', "fine$i@example.com")['status'];
        }

        self::assertSame([...array_fill(0, 10, 201), 429], $statuses);
    }

    /** SEG-044: a refusal by one limit does not spend the others. */
    public function testAnAddressAlreadyAtItsLimitDoesNotUseUpTheAccountsOwn(): void
    {
        $cloud = $this->world();
        foreach (range(1, 3) as $ignored) {
            self::assertSame(201, $this->invite($this->family, $cloud, self::EVA, 'email', 'laura@example.com')['status']);
        }
        foreach (range(1, 5) as $ignored) {
            self::assertSame(429, $this->invite($this->family, $cloud, self::EVA, 'email', 'laura@example.com')['status']);
        }

        $statuses = [];
        foreach (range(1, 8) as $i) {
            $statuses[] = $this->invite($this->family, $cloud, self::EVA, 'email', "other$i@example.com")['status'];
        }

        self::assertSame([...array_fill(0, 7, 201), 429], $statuses, 'ten in all, three already used');
    }

    public function testCreatingInvitationsIsLimitedPerAccount(): void
    {
        $cloud = $this->world();
        $statuses = [];
        foreach (range(1, 25) as $ignored) {
            $statuses[] = $this->invite($this->family, $cloud, self::NORA, 'link')['status'];
        }

        self::assertSame(array_fill(0, 20, 201), \array_slice($statuses, 0, 20));
        self::assertSame(429, $statuses[24]);
    }

    /** SEG-042: nothing written by whoever invites reaches the mail, not even the name of the library. */
    public function testTheEmailSaysOnlyWhatIsFixedAndNothingOfWhatWhoeverInvitesWrote(): void
    {
        $cloud = $this->world();
        $name = "Tu cuenta caduca: entra en ejemplo-acceso.com/login\x07\u{202E}\r\nBcc: spam@example.com\nVisit https://evil.example/pay now www.evil.example";
        $this->laptopSigns('library.rename', [['entity' => 'library', 'id' => $this->local, 'action' => 'update', 'fields' => ['name' => $name]]]);

        $this->invite($this->family, $cloud, self::EVA, 'email', 'eva@example.com');

        $message = self::getMailerMessage();
        \assert($message instanceof Email);
        $everything = $message->getSubject().$message->getTextBody().$message->getHtmlBody();
        self::assertSame('You have been invited to a library on Kosmotheka', $message->getSubject());
        self::assertNull($message->getHtmlBody(), 'plain text only');
        foreach (['ejemplo-acceso', 'evil', 'Bcc', 'caduca', 'Casa', 'spam', "\x07", "\u{202E}", '«', 'family@example.com', 'Ada', 'Laura'] as $trace) {
            self::assertStringNotContainsString($trace, $everything);
        }
        self::assertStringContainsString('invitations.test/i/', $everything, 'the link is still there');
        self::assertSame('eva@example.com', $message->getTo()[0]->getAddress());
    }

    public function testWithoutAnAddressForLinksTheServerMakesNoInvitations(): void
    {
        // Same container, the setting emptied: the server of someone who did not configure it (D18).
        $cloud = $this->world();
        $_SERVER['INVITATION_BASE_URL'] = $_ENV['INVITATION_BASE_URL'] = '';
        putenv('INVITATION_BASE_URL=');
        try {
            $answer = $this->invite($this->family, $cloud, self::NORA, 'link');
            $server = $this->call('GET', '/v1/server')['body'];
        } finally {
            $_SERVER['INVITATION_BASE_URL'] = $_ENV['INVITATION_BASE_URL'] = 'https://invitations.test';
            putenv('INVITATION_BASE_URL=https://invitations.test');
        }

        self::assertSame(['status' => 409, 'body' => ['error' => 'invitations_unavailable']], $answer);
        self::assertFalse($server['invitations']);
    }

    public function testExpiredInvitationsArePurgedAfterAWhile(): void
    {
        $cloud = $this->world();
        $this->invite($this->family, $cloud, self::NORA, 'link');
        $connection = self::getContainer()->get(Connection::class);
        $connection->executeStatement("UPDATE invitation SET expires_at = now() - interval '2 days'");
        self::assertSame(['unconfirmed_users' => 0, 'expired_codes' => 0, 'invitations' => 0, 'activity' => 0, 'google_attempts' => 0, 'metadata_cache' => 0, 'covers' => 0], self::getContainer()->get(AccountPurger::class)->purge());

        $connection->executeStatement("UPDATE invitation SET expires_at = now() - interval '10 days'");

        self::assertSame(1, self::getContainer()->get(AccountPurger::class)->purge()['invitations']);
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM invitation'));
    }

    // ------------------------------------------------------ the world ---

    /**
     * The family's library in the cloud, with roles and the cloud named in it (so it can link devices), uploaded by
     * the laptop of Laura, who administers and whose account is the `family` one.
     */
    private function world(): string
    {
        $cloud = $this->call('POST', '/v1/cloud/libraries', $this->family)['body']['id'] ?? null;
        \assert(\is_string($cloud));
        $reader = SigningDevice::generate();
        $fields = ['cloud_library_id' => $cloud, 'cloud_server' => SigningDevice::SERVER, 'cloud_device_id' => $this->identityOf($cloud)];
        $response = $this->push($this->family, $cloud, [
            $this->laptop->founding(self::T + $this->tick(), self::LAURA, personName: 'Laura'),
            $reader->registration($this->local, self::T + $this->tick()),
            $this->laptop->sign($this->local, self::T + $this->tick(), 'library.link', [['entity' => 'library', 'id' => $this->local, 'action' => 'update', 'fields' => $fields]]),
            $this->laptop->sign($this->local, self::T + $this->tick(), 'family.roles', [
                ['entity' => 'member', 'id' => self::HUGO, 'action' => 'create', 'fields' => ['name' => 'Hugo', 'role' => 'editor', 'can_share' => true]],
                ['entity' => 'member', 'id' => self::EVA, 'action' => 'create', 'fields' => ['name' => 'Eva', 'role' => 'editor']],
                ['entity' => 'member', 'id' => self::READER, 'action' => 'create', 'fields' => ['name' => 'Reader', 'role' => 'reader']],
                ['entity' => 'member', 'id' => self::NORA, 'action' => 'create', 'fields' => ['name' => 'Nora', 'role' => 'reader']],
                ['entity' => 'member', 'id' => self::TOMAS, 'action' => 'create', 'fields' => ['name' => 'Tomás', 'role' => 'admin']],
                ['entity' => 'member', 'id' => self::MARTA, 'action' => 'create', 'fields' => ['name' => 'Marta', 'role' => 'admin']],
                ['entity' => 'member', 'id' => self::OLGA, 'action' => 'create', 'fields' => ['name' => 'Olga', 'role' => 'reader']],
                $this->assign($reader->id, self::READER),
            ]),
        ]);
        self::assertSame(4, $response['received']);
        $this->laptopSigns('member.withdraw', [['entity' => 'member', 'id' => self::OLGA, 'action' => 'update', 'fields' => ['access_withdrawn' => true]]]);
        // The uploader's row is filled by the server itself (SEG-030); `claim` finds it linked already.
        self::assertSame(200, $this->call('POST', '/v1/cloud/libraries/claim', $this->family)['status']);
        self::assertSame(self::LAURA, $this->memberOf($cloud, 'family@example.com'));

        return $cloud;
    }

    /** Jhonny's own library, where he administers (and, optionally, a person with the id of the family's Nora). */
    private function jhonnysLibrary(bool $withNora = false): string
    {
        $cloud = $this->call('POST', '/v1/cloud/libraries', $this->jhonny)['body']['id'] ?? null;
        \assert(\is_string($cloud));
        $device = SigningDevice::fromSession($this->jhonny);
        $local = $device->libraryId();
        $fields = ['cloud_library_id' => $cloud, 'cloud_server' => SigningDevice::SERVER, 'cloud_device_id' => $this->identityOf($cloud, $this->jhonny)];
        $jhonny = '01997c7e-0000-7000-8000-0000000000b2';
        $operations = [
            $device->founding(self::T + $this->tick(), $jhonny, name: 'Jhonny', personName: 'Jhonny'),
            $device->sign($local, self::T + $this->tick(), 'library.link', [['entity' => 'library', 'id' => $local, 'action' => 'update', 'fields' => $fields]]),
        ];
        if ($withNora) {
            $operations[] = $device->sign($local, self::T + $this->tick(), 'family.roles', [['entity' => 'member', 'id' => self::NORA, 'action' => 'create', 'fields' => ['name' => 'Nora of Jhonny', 'role' => 'reader']]]);
        }
        $this->push($this->jhonny, $cloud, $operations);
        $this->call('POST', '/v1/cloud/libraries/claim', $this->jhonny);
        self::assertSame($jhonny, $this->memberOf($cloud, 'jhonny@example.com'));

        return $cloud;
    }

    /** @return array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string} */
    private function account(string $email): array
    {
        return $this->signInNewDevice($this->client, $email);
    }

    /** @return array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string} */
    private function login(string $email): array
    {
        return $this->loginDevice($this->client, $email, self::PASSWORD, Uuid::v4()->toRfc4122(), 'Phone');
    }

    /**
     * An account whose device is already that person's (the usual way, D96): registered, assigned with its key
     * vouched by the administrator, and claimed.
     *
     * @return array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string}
     */
    private function joinAs(string $email, string $member): array
    {
        $session = null === $this->userId($email) ? $this->account($email) : $this->login($email);
        $device = SigningDevice::fromSession($session);
        $cloud = $this->scalarString('SELECT id::text FROM cloud_library LIMIT 1', []);
        $this->push($this->family, $cloud, [$device->registration($this->local, self::T + $this->tick())]);
        $this->laptopSigns('device.assign', [$this->assign($device->id, $member, $device->publicKey)]);
        self::assertSame([$cloud], $this->call('POST', '/v1/cloud/libraries/claim', $session)['body']['linked'] ?? null, $email);

        return $session;
    }

    private function shareWith(string $cloud, string $email, string $member): void
    {
        $em = self::getContainer()->get(EntityManagerInterface::class);
        $user = $em->getRepository(User::class)->findOneBy(['email' => $email]) ?? self::fail('No user.');
        $library = $em->find(CloudLibrary::class, Uuid::fromString($cloud)) ?? self::fail('No library.');
        $em->persist(new Membership($user, $library, Uuid::fromString($member)));
        $em->flush();
    }

    // ------------------------------------------------------ the doors ---
    /**
     * @param array{headers: array<string, string>} $session
     *
     * @return array{status: int, body: array<mixed>}
     */
    private function invite(array $session, string $cloud, string $member, string $kind, ?string $email = null): array
    {
        return $this->call('POST', "/v1/cloud/libraries/$cloud/invitations", $session, ['memberId' => $member, 'kind' => $kind] + (null === $email ? [] : ['email' => $email]));
    }

    /** @return array{status: int, body: array<mixed>} */
    private function peek(string $token): array
    {
        return $this->call('GET', '/v1/cloud/invitations/'.rawurlencode($token));
    }

    /**
     * @param array{headers: array<string, string>} $session
     *
     * @return array{status: int, body: array<mixed>}
     */
    private function accept(array $session, string $token): array
    {
        return $this->call('POST', '/v1/cloud/invitations/'.rawurlencode($token).'/accept', $session);
    }

    /** @param array{status: int, body: array<mixed>} $created */
    private function tokenOf(array $created): string
    {
        self::assertSame(201, $created['status'], json_encode($created['body']) ?: '');
        $link = $created['body']['link'] ?? null;
        \assert(\is_string($link));

        return substr($link, strrpos($link, '/') + 1);
    }

    private function tokenInMail(): string
    {
        $messages = self::getMailerMessages();
        $message = end($messages);
        \assert($message instanceof Email);
        preg_match('~/i/([A-Za-z0-9_-]{43})~', (string) $message->getTextBody(), $found);

        return $found[1] ?? self::fail('No link in the email.');
    }

    /**
     * @param list<array<string, mixed>> $changes
     */
    private function laptopSigns(string $intent, array $changes): void
    {
        $cloud = $this->scalarString('SELECT id::text FROM cloud_library ORDER BY created_at LIMIT 1', []);
        $response = $this->push($this->family, $cloud, [$this->laptop->sign($this->local, self::T + $this->tick(), $intent, $changes)]);
        self::assertSame(1, $response['received']);
    }

    /** @return array<string, mixed> */
    private function assign(string $device, string $member, ?string $vouched = null): array
    {
        return ['entity' => 'device', 'id' => $device, 'action' => 'update', 'fields' => ['member_id' => $member] + (null === $vouched ? [] : ['vouched_key' => $vouched])];
    }

    private function tick(): int
    {
        return $this->time++;
    }

    /**
     * @param array{headers: array<string, string>} $session
     * @param list<array<string, mixed>>            $operations
     *
     * @return array<mixed>
     */
    private function push(array $session, string $cloud, array $operations): array
    {
        $version = $this->schema();

        return $this->call('POST', "/v1/cloud/libraries/$cloud/push", $session, [
            'schemaVersion' => $version,
            'operations' => array_map(static fn (array $operation): array => ['schemaVersion' => $version, 'operation' => $operation], $operations),
        ])['body'];
    }

    /**
     * @param array{headers: array<string, string>}|null $session
     * @param array<mixed>|null                          $body
     *
     * @return array{status: int, body: array<mixed>}
     */
    private function call(string $method, string $uri, ?array $session = null, ?array $body = null): array
    {
        $this->client->request($method, $uri, server: ['CONTENT_TYPE' => 'application/json'] + ($session['headers'] ?? []), content: null === $body ? null : json_encode($body, \JSON_THROW_ON_ERROR));
        $content = (string) $this->client->getResponse()->getContent();
        $decoded = '' === $content ? [] : json_decode($content, true, flags: \JSON_THROW_ON_ERROR);
        \assert(\is_array($decoded));

        return ['status' => $this->client->getResponse()->getStatusCode(), 'body' => $decoded];
    }

    private function schema(): int
    {
        return self::getContainer()->get(SchemaVersion::class)->current();
    }

    // ------------------------------------------------------ the activity log (7.7, D66) ---

    /** Hostile 1: what the judge leaves as a request, or the server refuses, leaves no line of «changed a role». */
    public function testAnOperationThatIsOnlyARequestOrIsRefusedLeavesNoLineOfAChange(): void
    {
        $cloud = $this->world();
        $stranger = SigningDevice::generate();
        $registration = $stranger->registration($this->local, self::T + $this->tick());
        $forged = $stranger->sign($this->local, self::T + $this->tick(), 'role.forged', [['entity' => 'member', 'id' => self::HUGO, 'action' => 'update', 'fields' => ['role' => 'admin', 'can_share' => true]]]);
        $tampered = $this->laptop->sign($this->local, self::T + $this->tick(), 'role.tampered', [['entity' => 'member', 'id' => self::HUGO, 'action' => 'update', 'fields' => ['role' => 'reader']]]);
        $signature = $tampered['signature'];
        \assert(\is_string($signature));
        $tampered['signature'] = strrev($signature);
        $before = $this->activityKinds();

        $result = $this->push($this->family, $cloud, [$registration, $forged, $tampered]);

        // The registration and the forged change are stored (the second one as a request); the tampered one is refused.
        self::assertSame(['received' => 2, 'known' => 0, 'rejected' => 1], array_intersect_key($result, array_flip(['received', 'known', 'rejected'])));
        self::assertSame(1, $this->scalar("SELECT COUNT(*) FROM cloud_operation WHERE status = 'request'"));
        self::assertSame('editor', $this->scalarString('SELECT role FROM catalog_members WHERE id = ?', [self::HUGO]));
        self::assertSame($before, $this->activityKinds(), 'no line of «changed a role» or «can share»');
    }

    /** Normal, and hostile 2: an applied change is written with the server\'s own author, library, name and time. */
    public function testAnAppliedRoleChangeIsWrittenWithWhatTheServerKnowsNotWhatTheBodySays(): void
    {
        $cloud = $this->world();
        $started = new \DateTimeImmutable('-2 seconds', new \DateTimeZone('UTC'));
        $this->client->setServerParameter('HTTP_USER_AGENT', 'Kosmotheka/1.2 (Linux)');
        $operation = $this->laptop->sign($this->local, self::T + $this->tick(), 'member.role', [['entity' => 'member', 'id' => self::HUGO, 'action' => 'update', 'fields' => ['role' => 'reader', 'can_share' => false]]]);

        $this->push($this->family, $cloud, [$operation]);

        $lines = $this->activity("kind IN ('role_changed', 'can_share_changed') AND operation_id = ?", [\is_string($operation['id']) ? $operation['id'] : '']);
        self::assertCount(2, $lines);
        foreach ($lines as $line) {
            self::assertSame($this->userId('family@example.com'), $line['account_id'], 'the account of the session');
            self::assertSame($this->family['device_id'], $line['device_id'], 'the device of the session');
            self::assertSame($cloud, $line['library_id'], 'the library of the table of access');
            self::assertSame('Family', $line['library_name'], 'the name the server holds');
            self::assertSame(self::HUGO, $line['subject_id']);
            self::assertSame('Kosmotheka/1.2 (Linux)', $line['user_agent']);
            self::assertGreaterThanOrEqual($started->format('Y-m-d H:i:s'), $line['occurred_at'], 'the server\'s time, not the operation\'s (2024)');
        }
        self::assertSame(['reader', 'false'], array_column($lines, 'detail'));
    }

    /** The upload, the roles, the withdrawal and what the cloud does by itself are written; so are the invitations. */
    public function testTheLibraryTheInvitationsAndWhatTheCloudDoesAloneLeaveTheirLines(): void
    {
        $cloud = $this->world();
        $nora = $this->account('nora@example.com');
        $created = $this->invite($this->family, $cloud, self::NORA, 'link');
        $this->accept($nora, $this->tokenOf($created));
        $other = $this->invite($this->family, $cloud, self::EVA, 'email', 'eva@example.com');
        $this->call('DELETE', "/v1/cloud/libraries/$cloud/invitations/".$this->idOf($other), $this->family);

        $kinds = $this->activityKinds();
        foreach (['library_uploaded', 'cloud_power_on', 'device_assigned', 'access_withdrawn', 'account_linked', 'invitation_created', 'invitation_accepted', 'invitation_revoked'] as $expected) {
            self::assertContains($expected, $kinds);
        }
        $made = $this->activity("kind = 'invitation_created' ORDER BY occurred_at, id");
        self::assertSame(['link', 'email'], array_column($made, 'detail'), 'the kind, never the email or the link');
        self::assertSame('Family', $made[0]['library_name']);
        self::assertSame($this->idOf($created), $made[0]['subject_id']);
        $accepted = $this->activity("kind = 'invitation_accepted'")[0];
        self::assertSame($this->userId('nora@example.com'), $accepted['account_id']);
        self::assertSame('Family', $accepted['library_name']);
        $linked = $this->activity("kind = 'account_linked' AND subject_id = ?", [self::NORA]);
        self::assertCount(1, $linked);
        self::assertSame($this->userId('nora@example.com'), $linked[0]['account_id'], 'who the cloud did it for');
        self::assertSame($this->identityOf($cloud), $linked[0]['device_id'], 'the cloud signed it');
        $everything = json_encode($this->activity('TRUE'), \JSON_THROW_ON_ERROR);
        self::assertStringNotContainsString($this->tokenOf($created), $everything);
        self::assertStringNotContainsString('eva@example.com', $everything);
        self::assertStringNotContainsString('invitations.test', $everything);
    }

    /** Hostile 5: the lines of a library that no longer exists stay, with its name of then. */
    public function testTheLinesOfALibraryThatIsDeletedStay(): void
    {
        $cloud = $this->world();
        $this->invite($this->family, $cloud, self::NORA, 'link');
        $this->client->request('DELETE', '/v1/account', server: ['CONTENT_TYPE' => 'application/json'] + $this->family['headers'], content: json_encode(['password' => self::PASSWORD], \JSON_THROW_ON_ERROR));
        self::assertSame(204, $this->client->getResponse()->getStatusCode());
        self::assertSame(0, $this->scalar('SELECT COUNT(*) FROM cloud_library'));

        $lines = $this->activity('library_id = ?', [$cloud]);

        self::assertNotEmpty($lines);
        self::assertSame(['Family'], array_values(array_unique(array_filter(array_map(static fn (array $line): string => \is_string($line['library_name']) ? $line['library_name'] : '', $lines)))));
    }

    /** SEG-041: an operation of one device uploaded by another is the first one\'s, «uploaded by» the second, from the second\'s address. */
    public function testAnOperationRelayedByAnotherDeviceIsWrittenToItsAuthorAndToWhoUploadedIt(): void
    {
        $cloud = $this->world();
        $hugo = $this->joinAs('hugo@example.com', self::HUGO);
        $this->client->setServerParameter('REMOTE_ADDR', '203.0.113.77');
        $this->client->setServerParameter('HTTP_USER_AGENT', 'Hugo-tablet/1');
        $operation = $this->laptop->sign($this->local, self::T + $this->tick(), 'member.role', [['entity' => 'member', 'id' => self::EVA, 'action' => 'update', 'fields' => ['role' => 'reader']]]);

        $this->push($hugo, $cloud, [$operation]);

        $line = $this->authored("kind = 'role_changed' AND subject_id = ?", [self::EVA])[0] ?? self::fail('No line.');
        self::assertSame($this->userId('family@example.com'), $line['account_id'], 'the author: the account of the person of the device that signed');
        self::assertSame($this->laptop->id, $line['device_id'], 'the author: the device that signed');
        self::assertSame($this->userId('hugo@example.com'), $line['uploaded_by_account_id']);
        self::assertSame($hugo['device_id'], $line['uploaded_by_device_id']);
        self::assertSame('203.0.113.77', $line['ip'], 'the address of the session that uploaded it');
        self::assertSame('Hugo-tablet/1', $line['user_agent']);
    }

    /** SEG-041: the author is a device whose person has no account; the line says so and does not borrow the uploader\'s. */
    public function testAnAuthorWhoseDeviceHasNoAccountLeavesTheAccountEmptyNotTheUploaders(): void
    {
        $cloud = $this->world();
        $hugo = $this->joinAs('hugo@example.com', self::HUGO);
        $tomas = SigningDevice::generate();
        $this->push($this->family, $cloud, [$tomas->registration($this->local, self::T + $this->tick())]);
        $this->laptopSigns('device.assign', [$this->assign($tomas->id, self::TOMAS, $tomas->publicKey)]);
        $operation = $tomas->sign($this->local, self::T + $this->tick(), 'member.role', [['entity' => 'member', 'id' => self::EVA, 'action' => 'update', 'fields' => ['role' => 'reader']]]);

        $this->push($hugo, $cloud, [$operation]);

        $line = $this->authored("kind = 'role_changed' AND subject_id = ?", [self::EVA])[0] ?? self::fail('No line.');
        self::assertSame($tomas->id, $line['device_id']);
        self::assertNull($line['account_id'], 'Tomás has no account');
        self::assertSame($this->userId('hugo@example.com'), $line['uploaded_by_account_id']);
    }

    /** SEG-041, hostile: the author is the one the signature verifies, not one the body names. */
    public function testTheBodyCannotChooseTheAuthorOfALine(): void
    {
        $cloud = $this->world();
        $hugo = $this->joinAs('hugo@example.com', self::HUGO);
        $before = $this->scalar('SELECT COUNT(*) FROM activity_log');
        $forged = $this->laptop->sign($this->local, self::T + $this->tick(), 'member.role', [['entity' => 'member', 'id' => self::EVA, 'action' => 'update', 'fields' => ['role' => 'reader']]]);
        // The laptop\'s signature, but the operation says it is Hugo\'s device that dated it.
        $hlc = \is_string($forged['hlc']) ? $forged['hlc'] : '';
        $forged['hlc'] = substr($hlc, 0, 26).$hugo['device_id'];

        $result = $this->push($hugo, $cloud, [$forged]);

        self::assertSame(1, $result['rejected'] ?? null);
        self::assertSame($before, $this->scalar('SELECT COUNT(*) FROM activity_log'), 'nothing is written for what the signature does not prove');
    }

    /** What the cloud signs has the cloud as its author and, as before, the account it was done for. */
    public function testWhatTheCloudDoesAloneHasTheCloudAsAuthorAndNoUploader(): void
    {
        $cloud = $this->world();
        $nora = $this->account('nora@example.com');
        $this->accept($nora, $this->tokenOf($this->invite($this->family, $cloud, self::NORA, 'link')));

        $line = $this->authored("kind = 'account_linked' AND subject_id = ?", [self::NORA])[0] ?? self::fail('No line.');

        self::assertSame($this->userId('nora@example.com'), $line['account_id'], 'who it was done for');
        self::assertSame($this->identityOf($cloud), $line['device_id'], 'the cloud signed it');
        self::assertNull($line['uploaded_by_account_id']);
        self::assertNull($line['uploaded_by_device_id']);
    }

    /**
     * @param list<int|string> $params
     *
     * @return list<array<string, mixed>>
     */
    private function authored(string $where, array $params = []): array
    {
        return self::getContainer()->get(Connection::class)->fetchAllAssociative(
            'SELECT kind, account_id::text AS account_id, device_id, uploaded_by_account_id::text AS uploaded_by_account_id, uploaded_by_device_id, subject_id, ip, user_agent FROM activity_log WHERE '.$where.' ORDER BY occurred_at, id',
            $params,
        );
    }

    /** @return list<string> */
    private function activityKinds(): array
    {
        return array_map(static fn (array $row): string => \is_string($row['kind']) ? $row['kind'] : '', $this->activity('TRUE ORDER BY occurred_at, id'));
    }

    /**
     * @param list<int|string> $params
     *
     * @return list<array<string, mixed>>
     */
    private function activity(string $where, array $params = []): array
    {
        return self::getContainer()->get(Connection::class)->fetchAllAssociative(
            'SELECT kind, occurred_at::text AS occurred_at, account_id::text AS account_id, device_id, library_id::text AS library_id, library_name, subject_id, operation_id::text AS operation_id, detail, ip, user_agent FROM activity_log WHERE '.$where,
            $params,
        );
    }

    // ------------------------------------------------------ what the server holds ---

    private function memberOf(string $cloud, string $email): ?string
    {
        $member = self::getContainer()->get(Connection::class)->fetchOne(
            'SELECT m.member_id FROM membership m JOIN "user" u ON u.id = m.user_id WHERE m.cloud_library_id = ? AND u.email = ?',
            [$cloud, $email],
        );

        return \is_string($member) ? $member : null;
    }

    private function accountIdOf(string $member, ?string $cloud = null): ?string
    {
        $cloud ??= $this->scalarString('SELECT id::text FROM cloud_library ORDER BY created_at LIMIT 1', []);
        $account = self::getContainer()->get(Connection::class)->fetchOne('SELECT account_id FROM catalog_members WHERE id = ? AND cloud_library_id = ?', [$member, $cloud]);

        return \is_string($account) ? $account : null;
    }

    private function accountIdOfUser(string $email): string
    {
        return $this->scalarString('SELECT account_id::text FROM "user" WHERE email = ?', [$email]);
    }

    private function userId(string $email): ?string
    {
        $id = self::getContainer()->get(Connection::class)->fetchOne('SELECT id FROM "user" WHERE email = ?', [$email]);

        return \is_string($id) ? $id : null;
    }

    /** `pending`, `accepted`, `revoked` or `expired` of the one invitation there is. */
    private function stateOfOnlyInvitation(): string
    {
        $row = self::getContainer()->get(Connection::class)->fetchAssociative('SELECT accepted_at, revoked_at, expires_at < now() AS expired FROM invitation');
        \assert(\is_array($row));

        return null !== $row['accepted_at'] ? 'accepted' : (null !== $row['revoked_at'] ? 'revoked' : ($row['expired'] ? 'expired' : 'pending'));
    }

    /** @param list<int|string> $params */
    private function scalar(string $sql, array $params = []): int
    {
        $count = self::getContainer()->get(Connection::class)->fetchOne($sql, $params);
        \assert(\is_int($count));

        return $count;
    }

    /** @param list<int|string> $params */
    private function scalarString(string $sql, array $params): string
    {
        $value = self::getContainer()->get(Connection::class)->fetchOne($sql, $params);
        \assert(\is_string($value));

        return $value;
    }

    /**
     * @param array{headers: array<string, string>}|null $as
     */
    private function identityOf(string $cloud, ?array $as = null): string
    {
        foreach ($this->libraryList($this->call('GET', '/v1/cloud/libraries', $as ?? $this->family)['body']) as $library) {
            if (($library['id'] ?? null) === $cloud && \is_string($library['cloudDeviceId'] ?? null)) {
                return $library['cloudDeviceId'];
            }
        }

        self::fail('Not listed.');
    }

    /**
     * @param array<mixed> $body
     *
     * @return list<array<mixed>>
     */
    private function libraryList(array $body): array
    {
        $libraries = $body['libraries'] ?? null;
        \assert(\is_array($libraries));

        return array_values(array_filter($libraries, \is_array(...)));
    }

    /** @param array{status: int, body: array<mixed>} $created */
    private function idOf(array $created): string
    {
        $id = $created['body']['id'] ?? null;
        \assert(\is_string($id));

        return $id;
    }

    /**
     * @param array<mixed> $body
     *
     * @return list<array<mixed>>
     */
    private function rowsOf(array $body, string $key): array
    {
        $rows = $body[$key] ?? null;
        \assert(\is_array($rows));

        return array_values(array_filter($rows, \is_array(...)));
    }

    /**
     * @param array<mixed> $values
     *
     * @return list<mixed>
     */
    private function sorted(array $values): array
    {
        sort($values);

        return $values;
    }
}

/** Random well-formed tokens that nobody made. */
final class InvitationTokenForTests
{
    public static function random(): string
    {
        return InvitationToken::generate();
    }
}
