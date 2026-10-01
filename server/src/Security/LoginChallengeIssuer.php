<?php

declare(strict_types=1);

namespace App\Security;

use Psr\Cache\CacheItemPoolInterface;
use Symfony\Component\DependencyInjection\Attribute\Autowire;

/**
 * Login challenges (milestone 7.1, part 2): 32 random bytes, valid 5 minutes,
 * single use. Kept in their own cache pool (config/packages/cache.yaml) so
 * clearing the rate limiter cache in tests does not also clear these.
 */
final readonly class LoginChallengeIssuer
{
    private const int EXPIRY_SECONDS = 300;

    public function __construct(
        #[Autowire(service: 'cache.auth_challenge')]
        private CacheItemPoolInterface $cache,
    ) {
    }

    /** @return array{challenge: string, expiresAt: \DateTimeImmutable} */
    public function issue(): array
    {
        $challenge = sodium_bin2base64(random_bytes(32), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
        $expiresAt = new \DateTimeImmutable('now', new \DateTimeZone('UTC'))->modify(\sprintf('+%d seconds', self::EXPIRY_SECONDS));

        $item = $this->cache->getItem($this->key($challenge));
        $item->set(true);
        $item->expiresAt($expiresAt);
        $this->cache->save($item);

        return ['challenge' => $challenge, 'expiresAt' => $expiresAt];
    }

    /**
     * Consumes the challenge whether or not it was valid, so it cannot be
     * replayed either way.
     *
     * @throws AccountException with AccountErrorCode::InvalidChallenge
     */
    public function consume(string $challenge): void
    {
        $key = $this->key($challenge);
        $isValid = $this->cache->getItem($key)->isHit();
        $this->cache->deleteItem($key);

        if (!$isValid) {
            throw AccountException::invalidChallenge();
        }
    }

    private function key(string $challenge): string
    {
        return 'login_challenge_'.$challenge;
    }
}
