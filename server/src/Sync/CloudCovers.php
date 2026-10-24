<?php

declare(strict_types=1);

namespace App\Sync;

use App\Entity\CloudCover;
use App\Entity\CloudLibrary;
use App\Enum\CoverRejection;
use App\Repository\CloudCoverRepository;
use Doctrine\DBAL\Exception\UniqueConstraintViolationException;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;
use Symfony\Component\DependencyInjection\Attribute\Autowire;
use Symfony\Component\Filesystem\Filesystem;
use Symfony\Component\Uid\Uuid;

/**
 * The cover files of the cloud libraries (D82): a device says which covers of a library it has, the server says
 * what it will send and what it wants, and each file goes on its own. Only covers an edition of that library points
 * to are kept or given; a file is kept under the name every device gives it, after checking that its bytes give
 * that hash and are an image of that type. The server never decodes an image: it reads its first bytes only.
 *
 * Who may ask is decided before, by CloudLibraryAccess: nothing here reads an id from the request.
 */
final readonly class CloudCovers
{
    /** `<sha256>.<ext>`, as the app names every cover. */
    public const string NAME = '[0-9a-f]{64}\.(?:jpg|png|webp|gif)';
    /** Far above a reduced cover (about 40 KB) and enough for one stored before covers were reduced. */
    public const int MAX_BYTES = 2 * 1024 * 1024;
    public const int MAX_PER_LIBRARY = 200 * 1024 * 1024;
    /** Across all the libraries an account owns: creating libraries has no limit, the room for covers does. */
    public const int MAX_PER_OWNER = 300 * 1024 * 1024;
    /** At most this many names in an inventory: far more covers than any library has. */
    public const int MAX_LISTED = 50_000;
    /** A cover nobody points to is removed after this, not at once: its edition may be about to arrive. */
    public const string UNREFERENCED_GRACE = '1 day';

    public function __construct(
        private CloudCoverRepository $covers,
        private EntityManagerInterface $em,
        private ClockInterface $clock,
        #[Autowire(param: 'app.covers_dir')]
        private string $directory,
        #[Autowire(param: 'app.covers_server_max_bytes')]
        private int $serverMaxBytes,
        private Filesystem $filesystem,
    ) {
    }

    /**
     * What the server sends and what it wants, among the covers the library's editions point to.
     *
     * @param list<string> $theirs the names the device says it has
     *
     * @return array{send: list<string>, want: list<string>}
     */
    public function swap(CloudLibrary $library, array $theirs): array
    {
        $referenced = array_fill_keys($this->covers->referencedIn($library), true);
        // Kept means on disk: a folder lost or restored without its files is asked for again.
        $kept = array_fill_keys(array_filter($this->covers->namesIn($library), fn (string $name): bool => is_file($this->path($library, $name))), true);
        $has = array_fill_keys(\array_slice($theirs, 0, self::MAX_LISTED), true);
        $send = $want = [];
        foreach (array_keys($referenced) as $name) {
            $name = (string) $name;
            if (isset($kept[$name]) && !isset($has[$name])) {
                $send[] = $name;
            } elseif (!isset($kept[$name]) && isset($has[$name])) {
                $want[] = $name;
            }
        }

        return ['send' => $send, 'want' => $want];
    }

    /** @throws CoverRejectedException `NotFound` unless an edition of the library points to it and the server has it */
    public function read(CloudLibrary $library, string $name): string
    {
        $path = $this->path($library, $name);
        if (!$this->covers->isReferencedIn($library, $name) || !$this->covers->findIn($library, $name) instanceof CloudCover || !is_file($path)) {
            throw new CoverRejectedException(CoverRejection::NotFound);
        }

        return (string) file_get_contents($path);
    }

    /**
     * Keeps a cover a device sent. One the server has already is taken as kept.
     *
     * @throws CoverRejectedException
     */
    public function keep(CloudLibrary $library, string $name, string $bytes): void
    {
        if (!$this->covers->isReferencedIn($library, $name)) {
            throw new CoverRejectedException(CoverRejection::NotFound);
        }
        $path = $this->path($library, $name);
        $kept = $this->covers->findIn($library, $name);
        if ($kept instanceof CloudCover && is_file($path)) {
            return;
        }
        if (\strlen($bytes) > self::MAX_BYTES) {
            throw new CoverRejectedException(CoverRejection::TooLarge);
        }
        [$hash, $extension] = explode('.', $name, 2);
        if (!hash_equals($hash, hash('sha256', $bytes))) {
            throw new CoverRejectedException(CoverRejection::NotTheCover);
        }
        if (!$this->isImageOfType($bytes, $extension)) {
            throw new CoverRejectedException(CoverRejection::NotAnImage);
        }
        if (!$kept instanceof CloudCover) {
            $this->checkRoom($library, \strlen($bytes));
        }
        // Written whole to a temporary file and renamed: never half a cover on disk.
        $this->filesystem->dumpFile($path, $bytes);
        if ($kept instanceof CloudCover) {
            return;
        }
        try {
            $this->em->persist(new CloudCover($library, $name, \strlen($bytes), $this->clock->now()));
            $this->em->flush();
        } catch (UniqueConstraintViolationException) {
            // Two devices sent it at once: the same bytes, so the file written is the same.
        }
    }

    /**
     * Removes the covers no edition points to any more, and the folders of libraries that are gone.
     *
     * @return int how many covers were removed
     */
    public function purge(): int
    {
        $removed = 0;
        foreach ($this->covers->unreferencedBefore($this->clock->now()->modify('-'.self::UNREFERENCED_GRACE)) as $cover) {
            $this->filesystem->remove($this->path($cover->getCloudLibrary(), $cover->getName()));
            $this->em->remove($cover);
            ++$removed;
        }
        $this->em->flush();
        foreach ($this->foldersOfGoneLibraries() as $folder) {
            $this->filesystem->remove($folder);
        }

        return $removed;
    }

    /**
     * Room for `$size` more bytes: in the library, in all the libraries of its owner (creating libraries has no
     * limit), and in the whole server.
     *
     * @throws CoverRejectedException
     */
    private function checkRoom(CloudLibrary $library, int $size): void
    {
        if ($this->covers->sizeOf($library) + $size > self::MAX_PER_LIBRARY) {
            throw new CoverRejectedException(CoverRejection::LibraryFull);
        }
        if ($this->covers->sizeOfOwner($library->getOwner()) + $size > self::MAX_PER_OWNER) {
            throw new CoverRejectedException(CoverRejection::AccountFull);
        }
        if ($this->covers->totalSize() + $size > $this->serverMaxBytes) {
            throw new CoverRejectedException(CoverRejection::ServerFull);
        }
    }

    private function path(CloudLibrary $library, string $name): string
    {
        return $this->directory.'/'.$library->getId()->toRfc4122().'/'.$name;
    }

    /** @return list<string> */
    private function foldersOfGoneLibraries(): array
    {
        if (!is_dir($this->directory)) {
            return [];
        }
        $gone = [];
        foreach (scandir($this->directory) ?: [] as $entry) {
            if (!Uuid::isValid($entry)) {
                continue;
            }
            if (!$this->em->find(CloudLibrary::class, Uuid::fromString($entry)) instanceof CloudLibrary) {
                $gone[] = $this->directory.'/'.$entry;
            }
        }

        return $gone;
    }

    /** By the first bytes only: the signature of each format. */
    private function isImageOfType(string $bytes, string $extension): bool
    {
        return match ($extension) {
            'jpg' => str_starts_with($bytes, "\xFF\xD8\xFF"),
            'png' => str_starts_with($bytes, "\x89PNG\r\n\x1A\n"),
            'gif' => str_starts_with($bytes, 'GIF87a') || str_starts_with($bytes, 'GIF89a'),
            'webp' => str_starts_with($bytes, 'RIFF') && 'WEBP' === substr($bytes, 8, 4),
            default => false,
        };
    }
}
