<?php

declare(strict_types=1);

namespace App\Security;

use Psr\Log\LoggerInterface;
use Symfony\Component\EventDispatcher\Attribute\AsEventListener;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\RequestStack;
use Symfony\Component\HttpKernel\KernelEvents;
use Symfony\Component\Mailer\MailerInterface;
use Symfony\Component\Mime\Email;

/**
 * The mail of the account routes whose answer must not depend on whether an address has an account (D100). During a
 * request it is only kept; it goes out in `kernel.terminate`, when with PHP-FPM the response has already left, so the
 * time the answer takes says nothing. Outside a request (the console, a test of a service) it goes out at once. No
 * Messenger transport: a queue would be one more thing to run for a self-hoster, and the mail is one `send` away.
 */
#[AsEventListener(event: KernelEvents::TERMINATE, method: 'flush')]
final class MailOutbox
{
    /** @var list<Email> */
    private array $waiting = [];

    public function __construct(
        private readonly MailerInterface $mailer,
        private readonly RequestStack $requests,
        private readonly LoggerInterface $logger,
    ) {
    }

    public function send(Email $email): void
    {
        if (!$this->requests->getMainRequest() instanceof Request) {
            $this->mailer->send($email);

            return;
        }

        $this->waiting[] = $email;
    }

    /** One that fails does not stop the rest, and says nothing of whom it was for: the log is not the place for an address. */
    public function flush(): void
    {
        $waiting = $this->waiting;
        $this->waiting = [];

        foreach ($waiting as $email) {
            try {
                $this->mailer->send($email);
            } catch (\Throwable $exception) {
                $this->logger->error('A mail of the account could not be sent after the response.', ['exception_class' => $exception::class]);
            }
        }
    }
}
