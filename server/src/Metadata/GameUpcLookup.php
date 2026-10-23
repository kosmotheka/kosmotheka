<?php

declare(strict_types=1);

namespace App\Metadata;

use App\Entity\MetadataCacheEntry;
use App\Repository\MetadataCacheRepository;
use App\Security\AccountException;
use App\Security\RequestLimiter;
use Doctrine\DBAL\Exception\UniqueConstraintViolationException;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;

/**
 * Which board game a barcode is, for an account (D62): the cache first, then GameUPC, within what the server allows
 * itself; and the game a person chose, sent back to GameUPC, which lives on those choices.
 *
 * A row keeps the barcode, like the ISBN of a book, and never who asked. A verified answer is kept 30 days; a guess or
 * nothing, a day, since somebody's choice can turn it into a verified one. A failure is never kept.
 */
final readonly class GameUpcLookup
{
    public function __construct(
        private GameUpcSource $source,
        private MetadataCacheRepository $cache,
        private EntityManagerInterface $entityManager,
        private RequestLimiter $limiter,
        private ClockInterface $clock,
        private ProviderGate $gate,
        private GameUpcUserId $userIds,
    ) {
    }

    /** @throws AccountException with AccountErrorCode::MetadataQuotaExhausted or MetadataFailed */
    public function lookup(Barcode $code, string $accountId): GameUpcAnswer
    {
        $now = $this->clock->now();
        $entry = $this->cache->findOneByProviderAndCode(GameUpcSource::NAME, $code->value);
        if ($entry instanceof MetadataCacheEntry && $entry->isFreshAt($now)) {
            return GameUpcAnswer::fromArray($entry->getPayload());
        }

        $this->gate->requireOpen(GameUpcSource::NAME);
        $answer = $this->gate->call(
            GameUpcSource::NAME,
            fn () => $this->limiter->consumeGameUpcCallOf($accountId),
            fn (): GameUpcAnswer => $this->source->lookup($code),
        );

        $found = GameUpcAnswer::VERIFIED === $answer->status;
        if ($entry instanceof MetadataCacheEntry) {
            $entry->renew($found, $answer->toArray(), $now);
        } else {
            $this->entityManager->persist(new MetadataCacheEntry(GameUpcSource::NAME, $code->value, $found, $answer->toArray(), $now));
        }
        $this->flush();

        return $answer;
    }

    /**
     * Tells GameUPC the game a person chose for this barcode, under the account's GameUPC id, and forgets what was
     * kept of the barcode, so the next look asks again.
     *
     * @throws AccountException with AccountErrorCode::TooManyRequests, MetadataQuotaExhausted or MetadataFailed
     */
    public function choose(Barcode $code, int $gameId, string $accountId): void
    {
        $this->gate->requireOpen(GameUpcSource::NAME);
        $this->gate->call(
            GameUpcSource::NAME,
            fn () => $this->limiter->consumeGameUpcChoiceOf($accountId),
            fn () => $this->source->choose($code, $gameId, $this->userIds->of($accountId)),
        );

        $entry = $this->cache->findOneByProviderAndCode(GameUpcSource::NAME, $code->value);
        if ($entry instanceof MetadataCacheEntry) {
            $this->entityManager->remove($entry);
            $this->flush();
        }
    }

    private function flush(): void
    {
        try {
            $this->entityManager->flush();
        } catch (UniqueConstraintViolationException) {
            // Another request stored the same barcode a moment ago: its row is as good as ours.
        }
    }
}
