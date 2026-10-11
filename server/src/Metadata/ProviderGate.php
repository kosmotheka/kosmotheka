<?php

declare(strict_types=1);

namespace App\Metadata;

use App\Security\AccountException;
use Psr\Cache\CacheItemPoolInterface;
use Symfony\Component\Clock\ClockInterface;
use Symfony\Component\DependencyInjection\Attribute\Target;

/**
 * What every outside catalogue is asked through (7.8): the pause after it says its quota is gone, and asking again
 * when it answers that it is broken. Nothing here knows what is asked or who asked.
 */
final readonly class ProviderGate
{
    /** How long the server leaves a provider alone after it said no on quota. */
    private const int PAUSE_SECONDS = 1800;
    /** The most real calls one question may cost, when the provider answers that it is broken. */
    private const int ATTEMPTS = 3;
    private const float SECONDS_BETWEEN_ATTEMPTS = 0.2;

    public function __construct(
        private ClockInterface $clock,
        #[Target('cache.metadata')]
        private CacheItemPoolInterface $pauses,
    ) {
    }

    /** @throws AccountException with AccountErrorCode::MetadataQuotaExhausted while the pause lasts */
    public function requireOpen(string $provider): void
    {
        $item = $this->pauses->getItem($this->pauseKey($provider));
        if ($item->isHit()) {
            $until = $item->get();
            $left = \is_int($until) ? $until - $this->clock->now()->getTimestamp() : self::PAUSE_SECONDS;

            throw AccountException::metadataQuotaExhausted($left);
        }
    }

    public function pause(string $provider): void
    {
        $item = $this->pauses->getItem($this->pauseKey($provider));
        $item->set($this->clock->now()->getTimestamp() + self::PAUSE_SECONDS)->expiresAfter(self::PAUSE_SECONDS);
        $this->pauses->save($item);
    }

    /**
     * Makes one question to the provider: `$spend` is called before every real call (it may refuse, with the account
     * exception of the day's cap), and the question is asked again when the provider broke itself. A slow provider is
     * not asked again, so nobody waits three times as long.
     *
     * @template T
     *
     * @param \Closure(): void $spend
     * @param \Closure(): T    $call
     *
     * @return T
     *
     * @throws AccountException with AccountErrorCode::MetadataQuotaExhausted or MetadataFailed
     */
    public function call(string $provider, \Closure $spend, \Closure $call): mixed
    {
        for ($attempt = 1;; ++$attempt) {
            $spend();

            try {
                return $call();
            } catch (MetadataProviderException $exception) {
                if ($exception->quota) {
                    $this->pause($provider);

                    throw AccountException::metadataQuotaExhausted(self::PAUSE_SECONDS);
                }
                if (!$exception->brokeItself || $attempt >= self::ATTEMPTS) {
                    throw AccountException::metadataFailed();
                }
            }

            $this->clock->sleep(self::SECONDS_BETWEEN_ATTEMPTS);
        }
    }

    private function pauseKey(string $provider): string
    {
        return 'pause_'.$provider;
    }
}
