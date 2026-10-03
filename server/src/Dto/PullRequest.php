<?php

declare(strict_types=1);

namespace App\Dto;

use Symfony\Component\Validator\Constraints as Assert;

/** Body of `POST /v1/cloud/libraries/{id}/pull` (docs/architecture/cloud.md). */
final readonly class PullRequest
{
    /**
     * @param array<string, string> $vector the last HLC the client has from each author, by device id
     */
    public function __construct(
        public int $schemaVersion,
        #[Assert\Count(max: 1000)]
        #[Assert\All([new Assert\Type('string'), new Assert\Regex(OperationPayload::HLC_PATTERN)])]
        public array $vector,
    ) {
    }
}
