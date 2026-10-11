<?php

declare(strict_types=1);

namespace App\Tests\Support;

use Psr\Log\AbstractLogger;

/**
 * The application log, in memory (test environment only): everything the kernel and the services would write is kept
 * as text, so a test can say that something never reached a log. Static, because the test client boots a new kernel
 * (and so a new logger) for every request.
 */
final class RecordingLogger extends AbstractLogger
{
    /** @var list<string> */
    public static array $lines = [];

    public static function reset(): void
    {
        self::$lines = [];
    }

    public static function everything(): string
    {
        return implode("\n", self::$lines);
    }

    /** @param array<array-key, mixed> $context */
    public function log(mixed $level, string|\Stringable $message, array $context = []): void
    {
        $line = \sprintf('[%s] %s', \is_scalar($level) ? (string) $level : 'log', $message);
        foreach ($context as $key => $value) {
            if ($value instanceof \Throwable) {
                $line .= ' | '.$value::class.': '.$value->getMessage();
            } elseif (\is_scalar($value) || $value instanceof \Stringable) {
                $line .= \sprintf(' | %s=%s', $key, $value);
            }
        }
        self::$lines[] = $line;
    }
}
