<?php

declare(strict_types=1);

namespace App\Metadata;

/**
 * An ISBN-13, the only form the server keeps (the cache has one row per book). ISBN-10 and hyphens are accepted on the
 * way in, as in the app (`crates/domain/src/isbn.rs`), and turned into the thirteen digits.
 */
final readonly class Isbn
{
    private function __construct(
        public string $value,
    ) {
    }

    /** @throws \InvalidArgumentException when it is not an ISBN */
    public static function parse(string $input): self
    {
        // Far longer than any ISBN with its hyphens: refuse before looking at it.
        if (\strlen($input) > 40) {
            throw new \InvalidArgumentException('Not an ISBN.');
        }

        $cleaned = strtoupper(str_replace(['-', ' '], '', $input));

        // `D`: a `$` alone would let a final line break through.
        if (1 === preg_match('/^\d{13}$/D', $cleaned)) {
            return self::thirteen($cleaned);
        }
        if (1 === preg_match('/^\d{9}[\dX]$/D', $cleaned)) {
            return self::ten($cleaned);
        }

        throw new \InvalidArgumentException('Not an ISBN.');
    }

    private static function thirteen(string $digits): self
    {
        if (!str_starts_with($digits, '978') && !str_starts_with($digits, '979')) {
            throw new \InvalidArgumentException('Not an ISBN.');
        }
        if (self::checkDigit13(substr($digits, 0, 12)) !== (int) $digits[12]) {
            throw new \InvalidArgumentException('Not an ISBN.');
        }

        return new self($digits);
    }

    private static function ten(string $digits): self
    {
        $sum = 0;
        for ($i = 0; $i < 10; ++$i) {
            $sum += (10 - $i) * ('X' === $digits[$i] ? 10 : (int) $digits[$i]);
        }
        if (0 !== $sum % 11) {
            throw new \InvalidArgumentException('Not an ISBN.');
        }

        $body = '978'.substr($digits, 0, 9);

        return new self($body.self::checkDigit13($body));
    }

    private static function checkDigit13(string $twelve): int
    {
        $sum = 0;
        for ($i = 0; $i < 12; ++$i) {
            $sum += (int) $twelve[$i] * (0 === $i % 2 ? 1 : 3);
        }

        return (10 - $sum % 10) % 10;
    }
}
