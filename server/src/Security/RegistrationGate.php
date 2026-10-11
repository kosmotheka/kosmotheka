<?php

declare(strict_types=1);

namespace App\Security;

use Symfony\Component\DependencyInjection\Attribute\Autowire;

/**
 * Who may make an account on this server. With `REGISTRATION_ALLOWED_EMAILS` empty, anybody (the default, and what
 * someone who hosts their own server expects); with addresses in it, only those: a server that is on the internet
 * before it is opened to everybody. The list is never published. Whoever already has an account signs in as always.
 */
final readonly class RegistrationGate
{
    /** @var list<string> */
    private array $allowed;

    public function __construct(
        #[Autowire(env: 'REGISTRATION_ALLOWED_EMAILS')]
        string $allowedEmails,
    ) {
        $addresses = preg_split('/[\s,;]+/', $allowedEmails, flags: \PREG_SPLIT_NO_EMPTY);
        $this->allowed = array_map(mb_strtolower(...), false === $addresses ? [] : $addresses);
    }

    public function isRestricted(): bool
    {
        return [] !== $this->allowed;
    }

    /** @param string $normalisedEmail as the account routes normalise it: trimmed and lower-cased */
    public function allows(string $normalisedEmail): bool
    {
        return !$this->isRestricted() || \in_array($normalisedEmail, $this->allowed, true);
    }
}
