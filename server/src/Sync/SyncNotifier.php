<?php

declare(strict_types=1);

namespace App\Sync;

use App\Entity\CloudLibrary;
use App\Entity\User;
use Psr\Log\LoggerInterface;
use Symfony\Component\DependencyInjection\Attribute\Autowire;
use Symfony\Component\Mercure\HubInterface;
use Symfony\Component\Mercure\Update;

/**
 * Tells connected devices that something changed (D75). A notice is private and carries no
 * data, only «look again»: what a device may see is still decided by its pull. Call it after
 * the commit, never inside a transaction. It never throws: a failed notice must not fail a sync.
 */
final readonly class SyncNotifier
{
    public function __construct(
        // Lazy, so a server without a hub never builds one (its keys are empty).
        #[Autowire(service: HubInterface::class, lazy: true)]
        private HubInterface $hub,
        private HubSettings $settings,
        private LoggerInterface $logger,
    ) {
    }

    public function libraryChanged(CloudLibrary $library): void
    {
        $this->publish(NoticeTopic::library($library));
    }

    public function librariesOfAccountChanged(User $user): void
    {
        $this->publish(NoticeTopic::account($user));
    }

    /**
     * Tells the administrator who invited that someone accepted (D43): their account's topic only, so the others do
     * not hear it. It says that an invitation was accepted in that library and nothing else; the app asks the rest.
     */
    public function invitationAccepted(User $inviter, CloudLibrary $library): void
    {
        $this->publish(NoticeTopic::account($inviter), json_encode(['event' => 'invitation_accepted', 'library' => $library->getId()->toRfc4122()], \JSON_THROW_ON_ERROR));
    }

    private function publish(string $topic, string $data = '{}'): void
    {
        if (!$this->settings->isEnabled()) {
            return;
        }

        try {
            $this->hub->publish(new Update($topic, $data, true));
        } catch (\Throwable $exception) {
            $this->logger->warning('A live notice could not be published.', ['topic' => $topic, 'exception' => $exception]);
        }
    }
}
