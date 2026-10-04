<?php

declare(strict_types=1);

namespace App\Sync;

use App\Entity\User;
use App\Repository\CloudLibraryRepository;
use App\Security\AccountException;
use Psr\Clock\ClockInterface;
use Symfony\Component\Mercure\Jwt\Grant;
use Symfony\Component\Mercure\Jwt\LcobucciFactory;
use Symfony\Component\Mercure\ProtocolVersion;

/**
 * The token a device uses to subscribe to the hub (D75). The topics come from the account's
 * own membership rows (the access table, D84), never from anything the client sends. It is
 * signed with the subscriber key only, so it can read notices and never publish one.
 */
final readonly class EventsToken
{
    public const int LIFETIME_SECONDS = 3600;

    public function __construct(
        private HubSettings $settings,
        private CloudLibraryRepository $libraries,
        private ClockInterface $clock,
    ) {
    }

    /**
     * @return array{token: string, topics: list<string>, expiresAt: string}
     *
     * @throws AccountException when this server has no hub
     */
    public function issue(User $user): array
    {
        if (!$this->settings->isEnabled()) {
            throw AccountException::eventsUnavailable();
        }

        $topics = [
            NoticeTopic::account($user),
            ...array_map(NoticeTopic::library(...), $this->libraries->findAllAccessibleBy($user)),
        ];
        $expires = new \DateTimeImmutable('@'.($this->clock->now()->getTimestamp() + self::LIFETIME_SECONDS));

        $token = $this->factory()->create([new Grant([Grant::ACTION_SUBSCRIBE], $topics)], [
            'iss' => $this->settings->issuer,
            'aud' => $this->settings->url,
            'sub' => $user->getId()->toRfc4122(),
            'client_id' => 'kosmotheka-app',
            'exp' => $expires,
        ]);

        return ['token' => $token, 'topics' => $topics, 'expiresAt' => $expires->setTimezone(new \DateTimeZone('UTC'))->format('Y-m-d\TH:i:s\Z')];
    }

    private function factory(): LcobucciFactory
    {
        \assert('' !== $this->settings->subscriberKey);

        return new LcobucciFactory($this->settings->subscriberKey, 'hmac.sha256', self::LIFETIME_SECONDS, '', ProtocolVersion::V1);
    }
}
