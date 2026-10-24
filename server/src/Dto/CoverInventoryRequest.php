<?php

declare(strict_types=1);

namespace App\Dto;

use App\Sync\CloudCovers;
use Symfony\Component\Validator\Constraints as Assert;

/** Body of `POST /v1/cloud/libraries/{id}/covers/inventory`: the cover files of that library the device has (D82). */
final readonly class CoverInventoryRequest
{
    /**
     * @param list<string> $have
     */
    public function __construct(
        #[Assert\Count(max: CloudCovers::MAX_LISTED)]
        #[Assert\All([new Assert\Regex('/\A'.CloudCovers::NAME.'\z/')])]
        public array $have = [],
    ) {
    }
}
