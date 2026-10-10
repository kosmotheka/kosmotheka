<?php

declare(strict_types=1);

namespace App\Dto;

use App\Enum\InvitationKind;
use Symfony\Component\Validator\Constraints as Assert;

/**
 * Body of `POST /v1/cloud/libraries/{id}/invitations` (docs/architecture/cloud.md). The person is a `member` id of
 * the client, so the server only compares it with that library's catalog (D84). It carries no role: whoever enters
 * by an invitation keeps the role their person has, and a new person is created as a reader by the administrator's
 * own signed operation, before.
 */
final readonly class CreateInvitationRequest
{
    public function __construct(
        #[Assert\NotBlank]
        #[Assert\Uuid]
        public string $memberId,
        public InvitationKind $kind,
        /** Only for the `email` kind, and then required. */
        #[Assert\Length(max: 180)]
        #[Assert\Email]
        public ?string $email = null,
    ) {
    }
}
