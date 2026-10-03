<?php

declare(strict_types=1);

namespace App\DataFixtures;

/**
 * What each scenario loaded and what the rules refused, in the order it
 * happened. ScenarioReportPrinter shows it when `doctrine:fixtures:load` ends.
 */
final class ScenarioReport
{
    /** @var array<string, list<string>> */
    private array $lines = [];

    public function add(string $scenario, string $line): void
    {
        $this->lines[$scenario][] = $line;
    }

    /**
     * @return array<string, list<string>>
     */
    public function all(): array
    {
        return $this->lines;
    }

    public function clear(): void
    {
        $this->lines = [];
    }
}
