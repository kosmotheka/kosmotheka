<?php

declare(strict_types=1);

namespace App\Dto;

use App\Metadata\Barcode;
use App\Metadata\GameUpcGame;
use Symfony\Component\Serializer\Attribute\SerializedName;
use Symfony\Component\Validator\Constraints as Assert;

/** Body of `POST /v1/metadata/gameupc/choice`: the barcode and the BoardGameGeek number of the game a person chose for it. */
final readonly class GameUpcChoiceRequest
{
    public function __construct(
        #[Assert\Length(max: 14)]
        #[Assert\Callback([GameUpcLookupRequest::class, 'checkCode'])]
        public string $code,
        #[SerializedName('external_id')]
        #[Assert\Range(min: 1, max: GameUpcGame::MAX_ID)]
        public int $externalId,
    ) {
    }

    public function barcode(): Barcode
    {
        return Barcode::parse($this->code);
    }
}
