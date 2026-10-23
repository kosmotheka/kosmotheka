<?php

declare(strict_types=1);

namespace App\Metadata;

/**
 * The barcode on a box (GTIN): 8, 12, 13 or 14 digits whose last one checks the others. Anything else is refused
 * before a provider is asked, so what reaches an outgoing address is only ever digits, and a mistyped code costs no call.
 */
final readonly class Barcode
{
    private function __construct(
        public string $value,
    ) {
    }

    /** @throws \InvalidArgumentException */
    public static function parse(string $code): self
    {
        if (1 !== preg_match('/^(\d{8}|\d{12,14})$/D', $code) || !self::checks($code)) {
            throw new \InvalidArgumentException('Not a barcode.');
        }

        return new self($code);
    }

    public static function isValid(string $code): bool
    {
        try {
            self::parse($code);
        } catch (\InvalidArgumentException) {
            return false;
        }

        return true;
    }

    /** The GTIN check digit: from the right, every other digit weighs three. */
    private static function checks(string $code): bool
    {
        $digits = array_map(intval(...), str_split($code));
        $check = array_pop($digits);
        $sum = 0;
        foreach (array_reverse($digits) as $position => $digit) {
            $sum += 0 === $position % 2 ? $digit * 3 : $digit;
        }

        return (10 - $sum % 10) % 10 === $check;
    }
}
