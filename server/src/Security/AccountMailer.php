<?php

declare(strict_types=1);

namespace App\Security;

use App\Entity\User;
use Symfony\Component\Mailer\MailerInterface;
use Symfony\Component\Mime\Email;
use Symfony\Contracts\Translation\TranslatorInterface;

/**
 * Sends the account emails (D49, D50): plain text, subject and body from the
 * `emails` translation domain in the user's locale. The only place that calls
 * the mailer for account mail, so every email goes through the per-address
 * rate limit (docs/architecture/accounts.md).
 */
final readonly class AccountMailer
{
    public function __construct(
        private MailerInterface $mailer,
        private TranslatorInterface $translator,
        private RequestLimiter $requestLimiter,
        private string $mailerFrom,
    ) {
    }

    public function sendConfirmationCode(User $user, string $code, int $expiryMinutes): void
    {
        $this->send($user, 'confirm_email.subject', 'confirm_email.body', [
            '%code%' => $code,
            '%minutes%' => $expiryMinutes,
        ]);
    }

    public function sendPasswordResetCode(User $user, string $code, int $expiryMinutes): void
    {
        $this->send($user, 'reset_password.subject', 'reset_password.body', [
            '%code%' => $code,
            '%minutes%' => $expiryMinutes,
        ]);
    }

    /** Sent instead of a code when someone tries to register an already-confirmed address. */
    public function sendExistingAccountNotice(User $user): void
    {
        $this->send($user, 'existing_account.subject', 'existing_account.body', []);
    }

    /**
     * Sent after a password change or reset (D50), whichever device triggered
     * it. Not rate limited: the change already happened, so a 429 here would
     * report a failure that did not occur, and only someone holding the
     * password or a code can trigger it.
     */
    public function sendPasswordChangedNotice(User $user): void
    {
        $this->send($user, 'password_changed.subject', 'password_changed.body', [], false);
    }

    /** Sent when TOTP is enabled (D78). Not rate limited, like the other "something changed" notices. */
    public function sendTwoFactorEnabledNotice(User $user): void
    {
        $this->send($user, 'two_factor_enabled.subject', 'two_factor_enabled.body', [], false);
    }

    /** Sent when the second factor is removed (D78), whether by the account or by the console command. */
    public function sendTwoFactorDisabledNotice(User $user): void
    {
        $this->send($user, 'two_factor_disabled.subject', 'two_factor_disabled.body', [], false);
    }

    /**
     * The invitation to a library (D43), to an address that may have no account. Fixed text in the language of the
     * account that invites, plain text only; the only thing that comes from the one who invites is the library's
     * name, cleaned (`libraryNameForMail`), and nothing of their name or email goes in the subject or the body. The
     * limits are the invitation's own (`RequestLimiter::consumeInvitationEmail*`), taken by the caller.
     */
    public function sendInvitation(string $to, string $locale, string $libraryName, string $link): void
    {
        $this->deliver($to, $locale, 'invitation.subject', 'invitation.body', [
            '%library%' => self::libraryNameForMail($libraryName),
            '%link%' => $link,
        ]);
    }

    /**
     * A library's name written by whoever administers it, made safe to quote in a plain-text email: control
     * characters and line breaks out, anything that looks like an address out (it would be a link in somebody
     * else's words), and short.
     */
    public static function libraryNameForMail(string $name): string
    {
        $name = preg_replace('/[\p{C}\p{Zl}\p{Zp}]+/u', ' ', $name) ?? '';
        $name = preg_replace('~\S*(?:://|www\.)\S*~iu', ' ', $name) ?? '';
        $name = trim((string) preg_replace('/\s+/u', ' ', $name));

        return mb_strimwidth($name, 0, 60, '…');
    }

    /** @param array<string, string|int> $parameters */
    private function send(User $user, string $subjectKey, string $bodyKey, array $parameters, bool $limited = true): void
    {
        if ($limited) {
            $this->requestLimiter->consumeEmailPerAddress($user->getEmail());
        }

        $this->deliver($user->getEmail(), $user->getLocale(), $subjectKey, $bodyKey, $parameters);
    }

    /** @param array<string, string|int> $parameters */
    private function deliver(string $to, string $locale, string $subjectKey, string $bodyKey, array $parameters): void
    {
        $subject = $this->translator->trans($subjectKey, [], 'emails', $locale);
        $body = $this->translator->trans($bodyKey, $parameters, 'emails', $locale);

        $email = new Email()
            ->from($this->mailerFrom)
            ->to($to)
            ->subject($subject)
            ->text($body);

        $this->mailer->send($email);
    }
}
