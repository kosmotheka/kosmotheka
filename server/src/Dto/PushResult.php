<?php

declare(strict_types=1);

namespace App\Dto;

/** What a push did, as counts: stored now, already there, refused. */
final readonly class PushResult
{
    public function __construct(
        public int $received,
        public int $known,
        public int $rejected,
    ) {
    }

    /**
     * @return array{received: int, known: int, rejected: int}
     */
    public function toArray(): array
    {
        return ['received' => $this->received, 'known' => $this->known, 'rejected' => $this->rejected];
    }
}
