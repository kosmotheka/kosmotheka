<?php

declare(strict_types=1);

namespace App\DataFixtures;

use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Event\ConsoleTerminateEvent;
use Symfony\Component\EventDispatcher\Attribute\AsEventListener;

/**
 * The fixtures cannot reach the console, so this prints their report once
 * the load command has finished.
 */
final readonly class ScenarioReportPrinter
{
    public function __construct(private ScenarioReport $report)
    {
    }

    #[AsEventListener]
    public function print(ConsoleTerminateEvent $event): void
    {
        if ('doctrine:fixtures:load' !== $event->getCommand()?->getName() || Command::SUCCESS !== $event->getExitCode()) {
            return;
        }

        $output = $event->getOutput();
        foreach ($this->report->all() as $scenario => $lines) {
            $output->writeln(\sprintf("\n<info>%s</info>", $scenario));
            foreach ($lines as $line) {
                $output->writeln('  '.$line);
            }
        }
    }
}
