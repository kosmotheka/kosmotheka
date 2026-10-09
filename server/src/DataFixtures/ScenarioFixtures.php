<?php

declare(strict_types=1);

namespace App\DataFixtures;

use App\Dto\PushedOperation;
use App\Dto\PushRequest;
use App\Dto\PushResult;
use App\Entity\CloudLibrary;
use App\Entity\Device;
use App\Entity\User;
use App\Repository\DeviceRepository;
use App\Security\AccountException;
use App\Sync\CloudLibraryAccess;
use App\Sync\CloudLibraryCreator;
use App\Sync\CloudPush;
use App\Sync\SchemaVersion;
use App\Validator\ValidOperationValidator;
use Doctrine\Bundle\FixturesBundle\Fixture;
use Doctrine\DBAL\Connection;
use Doctrine\Persistence\ObjectManager;
use Symfony\Component\PasswordHasher\Hasher\UserPasswordHasherInterface;

/**
 * The cloud as a story (docs/development/fixtures.md): the normal cases and
 * every seam of D84, one method per scenario. Everything goes in through the
 * services the routes use (CloudLibraryCreator, CloudPush, CloudLibraryAccess),
 * never by inserting rows, so the database holds only what the rules let in.
 * The attempts that must be refused run too, and the load stops if one got
 * through: a fixture that loads means the rules held.
 *
 * Accounts and devices are persisted directly: signing in is a controller
 * (AuthController), not a service. What is fixed is every id, key, operation and
 * HLC; what is not is the cloud library id (the server chooses it, D84) and the
 * timestamps the server stamps.
 */
final class ScenarioFixtures extends Fixture
{
    public function __construct(
        private readonly UserPasswordHasherInterface $passwordHasher,
        private readonly CloudLibraryCreator $creator,
        private readonly CloudLibraryAccess $access,
        private readonly CloudPush $cloudPush,
        private readonly DeviceRepository $devices,
        private readonly SchemaVersion $schemaVersion,
        private readonly Connection $connection,
        private readonly ScenarioReport $report,
        private readonly CastFile $castFile,
    ) {
    }

    public function load(ObjectManager $manager): void
    {
        $this->report->clear();

        $ana = $this->account($manager, Cast::ANA, 'Ana García');
        $hugo = $this->account($manager, Cast::HUGO, 'Hugo García');
        $kenji = $this->account($manager, Cast::KENJI, 'Kenji Tanaka');
        $jhonny = $this->account($manager, Cast::JHONNY, 'Jhonny');
        $this->signIn($manager, $ana, Cast::anaLaptop(), "Ana's laptop");
        $this->signIn($manager, $ana, Cast::anaPhone(), "Ana's phone");
        $this->signIn($manager, $hugo, Cast::hugoPhone(), "Hugo's phone");
        $this->signIn($manager, $kenji, Cast::kenjiLaptop(), "Kenji's laptop");
        $this->signIn($manager, $jhonny, Cast::jhonnyPhone(), "Jhonny's phone");
        $manager->flush();

        $garcia = $this->anaUploadsBibliotecaGarcia($ana);
        $this->anasPhoneAddsABookLater($garcia->library);
        $this->kenjiUploadsHisDefaultLibrary($kenji);

        $this->jhonnyCopiesTheGarciasOperationIds($jhonny, $garcia);
        $this->jhonnyUsesTheDefaultIdToo($jhonny);
        $this->jhonnySignsInWithTheIdOfAnasPhone($manager, $jhonny);

        $this->anOldOperationArrivesLate($garcia->library);
        $this->jhonnyCannotReachTheGarcias($jhonny, $garcia->library);
        $this->jhonnyPushesWithoutALink($jhonny);
        $this->nothingForeignGetsIntoTheGarcias($garcia->library);

        $this->report->add('Cast for the app', 'written to server/var/fixtures/'.basename($this->castFile->write()).' (make app-fixtures turns it into one app data folder per device)');
    }

    // ------------------------------------------------------------ normal ---

    /**
     * Ana's laptop uploads «Biblioteca García»: the link and the history of the
     * library, which also holds what Hugo and Lucía did.
     */
    private function anaUploadsBibliotecaGarcia(User $ana): Uploaded
    {
        $scenario = 'Familia García: Ana uploads «Biblioteca García»';
        $library = $this->creator->create($ana);
        $cloud = $library->getId()->toRfc4122();
        $local = Cast::GARCIA_LOCAL;
        [$laptop, $phone, $hugosPhone] = [Cast::anaLaptop(), Cast::anaPhone(), Cast::hugoPhone()];
        $member = static fn (string $name): string => Cast::id("member:$name");

        $operations = [
            // The member comes before the device that names it, in the same operation: a row cannot point at one that is not there yet.
            $this->operation($laptop, $local, 0, 'device.register', [
                $this->create('member', 'ana', ['name' => 'Ana', 'role' => 'admin', 'can_share' => true]),
                $this->create('device', 'ana-laptop', ['public_key' => $laptop->publicKey, 'name' => "Ana's laptop", 'member_id' => $member('ana')], $laptop->id),
            ]),
            $this->operation($laptop, $local, 1, 'library.create', [
                ['entity' => 'library', 'id' => $local, 'action' => 'create', 'fields' => ['name' => 'Biblioteca García']],
            ]),
            $this->operation($laptop, $local, 3, 'member.add', [$this->create('member', 'hugo', ['name' => 'Hugo', 'role' => 'reader'])]),
            // Lucía is a child with no account and no device: she is only a row here.
            $this->operation($laptop, $local, 4, 'member.add', [$this->create('member', 'lucia', ['name' => 'Lucía'])]),
            $this->operation($phone, $local, 5, 'device.register', [
                $this->create('device', 'ana-phone', ['public_key' => $phone->publicKey, 'name' => "Ana's phone"], $phone->id),
            ]),
            $this->operation($hugosPhone, $local, 6, 'device.register', [
                $this->create('device', 'hugo-phone', ['public_key' => $hugosPhone->publicKey, 'name' => "Hugo's phone"], $hugosPhone->id),
            ]),
            // A device registers itself; saying whose it is, once roles are on, is the administrator's (D73).
            $this->operation($laptop, $local, 7, 'device.assign', [
                ['entity' => 'device', 'id' => $phone->id, 'action' => 'update', 'fields' => ['member_id' => $member('ana')]],
                ['entity' => 'device', 'id' => $hugosPhone->id, 'action' => 'update', 'fields' => ['member_id' => $member('hugo')]],
            ]),
            $this->operation($laptop, $local, 10, 'book.add', $this->book('cien-anos', 'Cien años de soledad', 'Gabriel García Márquez', '9780060883287', $member('ana'), 'Salón')),
            $this->operation($laptop, $local, 11, 'book.add', $this->book('principito', 'El principito', 'Antoine de Saint-Exupéry', '9780156012195', $member('lucia'), 'Cuarto de Lucía')),
            // Hugo is a reader: a mark is his own, a change to the catalog is only a request (D68).
            $this->operation($hugosPhone, $local, 30, 'mark.set', [[
                'entity' => 'mark',
                'id' => ValidOperationValidator::markId($member('hugo'), Cast::id('work:cien-anos')),
                'action' => 'create',
                'fields' => ['member_id' => $member('hugo'), 'work_id' => Cast::id('work:cien-anos'), 'status' => 'finished', 'status_on' => '2025-09-20', 'rating' => 4, 'favorite' => true],
            ]]),
            $this->operation($hugosPhone, $local, 31, 'request.add', [$this->create('change_request', 'hugo-wants-another-book', ['note' => 'Can we buy the next one by García Márquez?'])]),
            // ...and a book is not his to add: it is stored in the log as a request, and the catalog never gets it.
            $this->operation($hugosPhone, $local, 32, 'book.add', $this->book('hugos-wish', 'Ficciones', 'Jorge Luis Borges', '9780802130303', $member('hugo'), 'Salón')),
            $this->operation($laptop, $local, 60, 'library.link', [[
                'entity' => 'library',
                'id' => $local,
                'action' => 'update',
                'fields' => ['cloud_library_id' => $cloud, 'cloud_server' => SigningDevice::SERVER],
            ]]),
        ];

        $this->expectStored($this->push($library, $operations), \count($operations), $scenario);
        $this->report->add($scenario, \sprintf('loaded: cloud library %s linked to local id %s, %d operations (3 authors, 3 members, 2 books, a mark and a request of Hugo)', $cloud, $local, \count($operations)));
        $this->report->add($scenario, 'Lucía has no account and no device: her copies are rows of the library operations, nothing else');

        return new Uploaded($library, $operations);
    }

    /** A later push, by another device of the same account. */
    private function anasPhoneAddsABookLater(CloudLibrary $garcia): void
    {
        $scenario = 'Familia García: Ana\'s phone adds a book later';
        $phone = Cast::anaPhone();
        // The phone's only operation in the upload is its registration; this one chains to it (D87).
        $registration = Cast::id(\sprintf('operation:device.register:5:%s', $phone->id));
        $operation = $phone->sign(Cast::GARCIA_LOCAL, Cast::at(3000), 'book.add', $this->book('rayuela', 'Rayuela', 'Julio Cortázar', '9788437604572', Cast::id('member:ana'), 'Estudio'), Cast::id('operation:rayuela'), $registration);

        $this->expectStored($this->push($garcia, [$operation]), 1, $scenario);
        $this->report->add($scenario, 'loaded: 1 operation, signed by the key the phone registered in the upload');
    }

    private function kenjiUploadsHisDefaultLibrary(User $kenji): void
    {
        $scenario = 'Familia Tanaka: Kenji uploads his library with the default id';
        $library = $this->creator->create($kenji);
        $laptop = Cast::kenjiLaptop();
        $local = Cast::DEFAULT_LOCAL;
        $operations = [
            $this->operation($laptop, $local, 0, 'device.register', [
                $this->create('member', 'kenji', ['name' => 'Kenji', 'role' => 'admin']),
                $this->create('device', 'kenji-laptop', ['public_key' => $laptop->publicKey, 'name' => "Kenji's laptop", 'member_id' => Cast::id('member:kenji')], $laptop->id),
            ]),
            $this->operation($laptop, $local, 1, 'library.create', [
                ['entity' => 'library', 'id' => $local, 'action' => 'create', 'fields' => ['name' => 'Biblioteca Tanaka']],
            ]),
            $this->operation($laptop, $local, 3, 'book.add', $this->book('kokoro', 'Kokoro', 'Natsume Sōseki', '9780804818957', Cast::id('member:kenji'), 'Estantería')),
            $this->operation($laptop, $local, 4, 'library.link', [[
                'entity' => 'library',
                'id' => $local,
                'action' => 'update',
                'fields' => ['cloud_library_id' => $library->getId()->toRfc4122(), 'cloud_server' => SigningDevice::SERVER],
            ]]),
        ];

        $this->expectStored($this->push($library, $operations), \count($operations), $scenario);
        $this->report->add($scenario, \sprintf('loaded: cloud library %s linked to the fixed default local id, %d operations', $library->getId()->toRfc4122(), \count($operations)));
    }

    // ------------------------------------------------------------- seams ---

    /** Seam 1: operation ids are unique within a cloud library, not across them. */
    private function jhonnyCopiesTheGarciasOperationIds(User $jhonny, Uploaded $garcia): void
    {
        $scenario = 'Seam 1: Jhonny pushes the Garcías\' operation ids into his own library';
        $library = $this->creator->create($jhonny);
        $link = Cast::jhonnyPhone()->link(Cast::GARCIA_LOCAL, $library->getId()->toRfc4122(), Cast::at(0), Cast::id('operation:jhonny-links-the-garcia-id'));
        $this->expectStored($this->push($library, [$link]), 1, $scenario);

        $this->expectStored($this->push($library, $garcia->operations), \count($garcia->operations), $scenario);
        $this->report->add($scenario, \sprintf('loaded: Jhonny\'s library %s, linked to the Garcías\' local id, holds %d of their operations under the same ids', $library->getId()->toRfc4122(), \count($garcia->operations)));
        $this->report->add($scenario, 'the Garcías\' own copy is untouched: the same operation_id is in two cloud libraries');
    }

    /** Seam 2: one local id in two libraries makes two cloud libraries. Seam 4: a key registered under somebody else's device id stays inside this library. */
    private function jhonnyUsesTheDefaultIdToo(User $jhonny): void
    {
        $scenario = 'Seams 2 and 4: Jhonny uses the default local id too, and registers Ana\'s laptop id with his key';
        $library = $this->creator->create($jhonny);
        $impostor = Cast::jhonnyAsAnaLaptop();
        $operations = [
            Cast::jhonnyPhone()->link(Cast::DEFAULT_LOCAL, $library->getId()->toRfc4122(), Cast::at(0), Cast::id('operation:jhonny-links-the-default-id')),
            $impostor->registration(Cast::DEFAULT_LOCAL, Cast::at(1), "Ana's laptop", operationId: Cast::id('operation:jhonny-registers-anas-laptop-id')),
        ];

        $this->expectStored($this->push($library, $operations), \count($operations), $scenario);
        $this->report->add($scenario, \sprintf('loaded: Jhonny\'s library %s uses the fixed default local id, like Kenji\'s: two cloud libraries, one local id', $library->getId()->toRfc4122()));
        $this->report->add($scenario, 'loaded: in it, Ana\'s laptop id registered with Jhonny\'s key (cloud_device_key); in the Garcías\' library the same id keeps Ana\'s key');
    }

    /** Seam 6 (SEG-016): a device is its id together with its key, so two accounts may hold rows with one id. */
    private function jhonnySignsInWithTheIdOfAnasPhone(ObjectManager $manager, User $jhonny): void
    {
        $scenario = 'Seam 6: Jhonny signs in a device with the id of Ana\'s phone and his own key';
        $this->signIn($manager, $jhonny, Cast::jhonnyAsAnaPhone(), "Ana's phone");
        $manager->flush();

        $this->report->add($scenario, 'loaded: two `device` rows with the same device_id, different accounts and different keys');
    }

    /**
     * Seam 7 (DAT-001): nothing is refused for arriving late. With the chain of each author (D87) that holds
     * between authors: the laptop's older operation arrives after the phone's newer one.
     */
    private function anOldOperationArrivesLate(CloudLibrary $garcia): void
    {
        $scenario = 'Seam 7: an older operation of Ana\'s laptop arrives after a newer one of her phone';
        $laptop = Cast::anaLaptop();
        // The laptop's last operation in the upload is the link, at +60 minutes.
        $link = Cast::id(\sprintf('operation:library.link:60:%s', $laptop->id));
        $older = $laptop->sign(Cast::GARCIA_LOCAL, Cast::at(2900), 'copy.move', [[
            'entity' => 'copy',
            'id' => Cast::id('copy:cien-anos'),
            'action' => 'update',
            'fields' => ['location' => 'Estudio'],
        ]], Cast::id('operation:cien-anos-moves'), $link);

        $this->expectStored($this->push($garcia, [$older]), 1, $scenario);
        $this->report->add($scenario, 'loaded: the laptop\'s operation at +2900 minutes is stored after the phone\'s at +3000');
    }

    // ----------------------------------------------- refused, no rows left ---

    private function jhonnyCannotReachTheGarcias(User $jhonny, CloudLibrary $garcia): void
    {
        $before = $this->stored();
        try {
            $this->access->require($garcia->getId()->toRfc4122(), $jhonny);
        } catch (AccountException $exception) {
            if (404 === $exception->statusCode && $before === $this->stored()) {
                $this->report->add('Refused: Jhonny asks for the Garcías\' library', '404 not_found, the same answer as for an id that does not exist; nothing stored');

                return;
            }
        }

        throw new RuleBrokenException('Jhonny got through the door of the Garcías\' library.');
    }

    /** The library is created (that is not a push) but stays unlinked and empty. */
    private function jhonnyPushesWithoutALink(User $jhonny): void
    {
        $library = $this->creator->create($jhonny);
        $this->expectRefused('Refused: a push without a link', 'no operation enters a library nobody linked', $library, [
            Cast::jhonnyPhone()->registration(Cast::GARCIA_LOCAL, Cast::at(1)),
            Cast::jhonnyPhone()->publisher(Cast::GARCIA_LOCAL, Cast::at(2)),
        ]);
        $this->report->add('Refused: a push without a link', \sprintf('the library itself exists: %s, local_library_id NULL, no operations', $library->getId()->toRfc4122()));
    }

    private function nothingForeignGetsIntoTheGarcias(CloudLibrary $garcia): void
    {
        $laptop = Cast::anaLaptop();
        $unsigned = $laptop->publisher(Cast::GARCIA_LOCAL, Cast::at(4000), 'Unsigned');
        unset($unsigned['signature']);
        $tampered = $laptop->publisher(Cast::GARCIA_LOCAL, Cast::at(4001), 'Signed as this');
        $tampered['changes'] = [$this->create('publisher', 'tampered', ['name' => 'Changed after signing'])];

        $this->expectRefused('Refused: an operation of another local id', 'Ana\'s laptop, validly signed, for a library that is not this one', $garcia, [
            $laptop->publisher(Cast::OTHER_LOCAL, Cast::at(4002)),
        ]);
        $this->expectRefused('Refused: an unsigned operation', 'no signature, no entry (the cloud has no operations from before D56)', $garcia, [$unsigned]);
        $this->expectRefused('Refused: a forged signature', 'the body was changed after signing', $garcia, [$tampered]);
        $this->expectRefused('Refused: a forged signature', 'signed with Jhonny\'s key under Ana\'s laptop id, which this library registered with Ana\'s key', $garcia, [
            Cast::jhonnyAsAnaLaptop()->publisher(Cast::GARCIA_LOCAL, Cast::at(4003)),
        ]);
    }

    // ----------------------------------------------------------- helpers ---

    private function account(ObjectManager $manager, string $email, string $name): User
    {
        $user = new User($email, $name, 'es');
        $user->setPassword($this->passwordHasher->hashPassword($user, Cast::PASSWORD));
        $user->confirm(new \DateTimeImmutable('now', new \DateTimeZone('UTC')));
        $manager->persist($user);

        return $user;
    }

    /** A device of the account, signed in with the fixed token of the cast. */
    private function signIn(ObjectManager $manager, User $user, SigningDevice $device, string $name): void
    {
        $manager->persist(new Device($device->id, $user, $name, $device->publicKey, hash('sha256', Cast::token($device))));
    }

    /**
     * @param list<array<string, mixed>> $changes
     *
     * @return array<string, mixed>
     */
    private function operation(SigningDevice $by, string $library, int $minute, string $intent, array $changes): array
    {
        return $by->sign($library, Cast::at($minute), $intent, $changes, Cast::id(\sprintf('operation:%s:%d:%s', $intent, $minute, $by->id)));
    }

    /**
     * @param array<string, mixed> $fields
     *
     * @return array{entity: string, id: string, action: string, fields: array<string, mixed>}
     */
    private function create(string $entity, string $name, array $fields, ?string $id = null): array
    {
        return ['entity' => $entity, 'id' => $id ?? Cast::id("$entity:$name"), 'action' => 'create', 'fields' => $fields];
    }

    /**
     * A book of the kind the app records: the work, its author, a publisher, one
     * edition and the copy on a shelf.
     *
     * @return list<array{entity: string, id: string, action: string, fields: array<string, mixed>}>
     */
    private function book(string $key, string $title, string $author, string $isbn, string $owner, string $location): array
    {
        $work = Cast::id("work:$key");
        $edition = Cast::id("edition:$key");

        return [
            $this->create('person', $key, ['name' => $author]),
            $this->create('work', $key, ['kind' => 'book', 'title' => $title]),
            $this->create('work_contributor', $key, ['work_id' => $work, 'person_id' => Cast::id("person:$key"), 'role' => 'author', 'position' => 0]),
            $this->create('work_genre', $key, ['work_id' => $work, 'genre' => 'novel']),
            $this->create('publisher', $key, ['name' => 'Alfaguara']),
            $this->create('edition', $key, ['work_id' => $work, 'title' => $title, 'publisher_id' => Cast::id("publisher:$key"), 'language' => 'es']),
            $this->create('book_details', $key, ['isbn13' => $isbn], $edition),
            $this->create('copy', $key, ['edition_id' => $edition, 'owner_id' => $owner, 'format' => 'paperback', 'quantity' => 1, 'location' => $location]),
        ];
    }

    /**
     * @param list<array<string, mixed>> $operations
     */
    private function push(CloudLibrary $library, array $operations): PushResult
    {
        $version = $this->schemaVersion->current();
        // Every scenario uploads from a device of the library's owner.
        $uploader = $this->devices->findOneBy(['user' => $library->getOwner()]) ?? throw new \LogicException('The owner of the library has no device.');

        return $this->cloudPush->push($library, new PushRequest($version, array_map(
            static function (array $operation) use ($version): PushedOperation {
                $decoded = json_decode(json_encode($operation, \JSON_THROW_ON_ERROR), false, flags: \JSON_THROW_ON_ERROR);
                \assert($decoded instanceof \stdClass);

                return new PushedOperation($version, $decoded);
            },
            $operations,
        )), $uploader);
    }

    /**
     * @throws RuleBrokenException when the rules refused something a normal case needs
     */
    private function expectStored(PushResult $result, int $received, string $scenario): void
    {
        if ($result->received !== $received || 0 !== $result->known || 0 !== $result->rejected) {
            throw new RuleBrokenException(\sprintf('%s: expected %d stored, got %d stored, %d known, %d rejected.', $scenario, $received, $result->received, $result->known, $result->rejected));
        }
    }

    /**
     * @param list<array<string, mixed>> $operations
     *
     * @throws RuleBrokenException when any of them was stored
     */
    private function expectRefused(string $scenario, string $why, CloudLibrary $into, array $operations): void
    {
        $before = $this->stored();
        $result = $this->push($into, $operations);
        if (0 !== $result->received || \count($operations) !== $result->rejected || $before !== $this->stored()) {
            throw new RuleBrokenException(\sprintf('%s got through: %d stored, %d rejected.', $scenario, $result->received, $result->rejected));
        }

        $this->report->add($scenario, \sprintf('refused: %s; nothing stored', $why));
    }

    /**
     * What the attempts must not change.
     *
     * @return array{operations: int, keys: int}
     */
    private function stored(): array
    {
        return ['operations' => $this->rows('cloud_operation'), 'keys' => $this->rows('cloud_device_key')];
    }

    private function rows(string $table): int
    {
        $count = $this->connection->fetchOne("SELECT COUNT(*) FROM $table");
        \assert(\is_int($count));

        return $count;
    }
}
