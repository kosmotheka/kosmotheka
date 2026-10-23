<?php

declare(strict_types=1);

namespace App\Dto;

use App\Metadata\Barcode;
use Symfony\Component\Validator\Constraints as Assert;
use Symfony\Component\Validator\Context\ExecutionContextInterface;

/** Body of `POST /v1/metadata/gameupc/lookup`: a POST, like the searches, so that what a person scans is never in an address. */
final readonly class GameUpcLookupRequest
{
    public function __construct(
        #[Assert\Length(max: 14)]
        #[Assert\Callback([self::class, 'checkCode'])]
        public string $code,
    ) {
    }

    public function barcode(): Barcode
    {
        return Barcode::parse($this->code);
    }

    public static function checkCode(string $code, ExecutionContextInterface $context): void
    {
        if (!Barcode::isValid($code)) {
            $context->buildViolation('The code must be a barcode: 8, 12, 13 or 14 digits with a right check digit.')->addViolation();
        }
    }
}
