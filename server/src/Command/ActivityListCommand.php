<?php

declare(strict_types=1);

namespace App\Command;

use App\Entity\ActivityEntry;
use App\Enum\ActivityKind;
use App\Repository\ActivityEntryRepository;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;
use Symfony\Component\Console\Style\SymfonyStyle;
use Symfony\Component\Uid\Uuid;

/**
 * Reads the activity log (D66): the only way to see it until there is an admin panel (D52, Phase 11). There is no
 * HTTP route to it, and nothing here changes it. Output is cleaned of control characters, whatever a line holds.
 */
#[AsCommand(name: 'app:activity:list', description: 'Lists the activity log, newest first')]
final class ActivityListCommand extends Command
{
    private const int DEFAULT_LIMIT = 50;
    private const int MAX_LIMIT = 1000;

    public function __construct(
        private readonly ActivityEntryRepository $entries,
    ) {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this
            ->addOption('account', null, InputOption::VALUE_REQUIRED, 'Only this account (its id): lines it did, or that its session uploaded')
            ->addOption('library', null, InputOption::VALUE_REQUIRED, 'Only this cloud library (its id)')
            ->addOption('kind', null, InputOption::VALUE_REQUIRED, 'Only this kind of event')
            ->addOption('since', null, InputOption::VALUE_REQUIRED, 'From this moment on, UTC (e.g. 2026-10-01)')
            ->addOption('until', null, InputOption::VALUE_REQUIRED, 'Until this moment, UTC, not included')
            ->addOption('limit', null, InputOption::VALUE_REQUIRED, \sprintf('How many lines at most (default %d, never more than %d)', self::DEFAULT_LIMIT, self::MAX_LIMIT));
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $io = new SymfonyStyle($input, $output);
        try {
            $account = $this->uuid($input->getOption('account'), 'account');
            $library = $this->uuid($input->getOption('library'), 'library');
            $kind = $this->kind($input->getOption('kind'));
            $since = $this->moment($input->getOption('since'), 'since');
            $until = $this->moment($input->getOption('until'), 'until');
        } catch (\InvalidArgumentException $exception) {
            $io->error($exception->getMessage());

            return Command::INVALID;
        }
        $asked = $input->getOption('limit');
        $limit = max(1, min(self::MAX_LIMIT, \is_string($asked) && ctype_digit($asked) ? (int) $asked : self::DEFAULT_LIMIT));

        $entries = $this->entries->search($account, $library, $kind, $since, $until, $limit);
        $io->table(
            ['When (UTC)', 'What', 'Account', 'Device', 'Uploaded by', 'Library', 'About', 'Detail', 'IP', 'Agent'],
            array_map($this->row(...), $entries),
        );
        $io->writeln(\sprintf('%d line(s)%s.', \count($entries), \count($entries) === $limit ? ' (the limit; narrow it or raise --limit)' : ''));

        return Command::SUCCESS;
    }

    /** @return list<string> */
    private function row(ActivityEntry $entry): array
    {
        $library = $entry->getLibraryId()?->toRfc4122();
        $name = $entry->getLibraryName();

        return array_map($this->clean(...), [
            $entry->getOccurredAt()->format('Y-m-d H:i:s'),
            $entry->getKind()->value,
            $entry->getAccountId()?->toRfc4122() ?? '-',
            $entry->getDeviceId() ?? '-',
            !$entry->getUploadedByAccountId() instanceof Uuid && null === $entry->getUploadedByDeviceId() ? '-' : ($entry->getUploadedByAccountId()?->toRfc4122() ?? '-').' / '.($entry->getUploadedByDeviceId() ?? '-'),
            null === $library ? '-' : $library.(null === $name ? '' : ' «'.$name.'»'),
            $entry->getSubjectId() ?? '-',
            $entry->getDetail() ?? '-',
            $entry->getIp() ?? ('console' === $entry->getChannel() ? 'console' : '-'),
            $entry->getUserAgent() ?? '-',
        ]);
    }

    /** What the console shows is text, never a command: control characters and the console markup are taken out. */
    private function clean(string $text): string
    {
        $text = (string) preg_replace('/[\x00-\x1F\x7F]+/u', ' ', mb_scrub($text));

        return str_replace(['<', '>'], ['‹', '›'], $text);
    }

    private function uuid(mixed $value, string $name): ?Uuid
    {
        if (null === $value || '' === $value) {
            return null;
        }
        if (!\is_string($value) || !Uuid::isValid($value)) {
            throw new \InvalidArgumentException(\sprintf('--%s must be an id.', $name));
        }

        return Uuid::fromString($value);
    }

    private function kind(mixed $value): ?ActivityKind
    {
        if (null === $value || '' === $value) {
            return null;
        }

        return \is_string($value) ? ActivityKind::tryFrom($value) ?? throw new \InvalidArgumentException(\sprintf('--kind must be one of: %s.', implode(', ', array_map(static fn (ActivityKind $kind): string => $kind->value, ActivityKind::cases())))) : null;
    }

    private function moment(mixed $value, string $name): ?\DateTimeImmutable
    {
        if (null === $value || '' === $value) {
            return null;
        }
        try {
            return new \DateTimeImmutable(\is_string($value) ? $value : '', new \DateTimeZone('UTC'));
        } catch (\Exception) {
            throw new \InvalidArgumentException(\sprintf('--%s is not a date.', $name));
        }
    }
}
