<?php

declare(strict_types=1);

namespace App\Security;

use App\Entity\Device;
use App\Entity\User;
use App\Enum\AccountErrorCode;
use App\Enum\ActivityKind;
use App\Repository\DeviceRepository;
use App\Sync\Invitation\EmailFingerprint;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Clock\ClockInterface;

/**
 * The last step of every sign-in, whichever way the account was proved (password, or Google): the device, as its id
 * and its key together, and the token it is given. The proof of the key was made before this is called.
 */
final readonly class DeviceSignIn
{
    public function __construct(
        private DeviceRepository $deviceRepository,
        private EntityManagerInterface $entityManager,
        private TwoFactor $twoFactor,
        private RequestLimiter $requestLimiter,
        private ActivityLog $activityLog,
        private EmailFingerprint $fingerprints,
        private ClockInterface $clock,
    ) {
    }

    /**
     * D78: asked after the account is proved and its status looked at, before the device itself. The challenge is
     * already consumed by this point, so a wrong or missing code means the app has to request a new one before retrying.
     *
     * @throws AccountException with two_factor_required or invalid_two_factor_code
     */
    public function requireTwoFactor(User $user, ?string $totpCode): void
    {
        if (!$user->isTwoFactorEnabled()) {
            return;
        }

        if (null === $totpCode) {
            throw AccountException::twoFactorRequired();
        }

        try {
            $this->twoFactor->verify($user, $totpCode);
        } catch (AccountException $exception) {
            // A wrong code, not a limit being hit: that one says nothing about who is trying.
            if (AccountErrorCode::InvalidTwoFactorCode === $exception->errorCode) {
                $this->requestLimiter->noteLoginFailure($this->fingerprints->of($user->getEmail()));
                $this->activityLog->recordNow(ActivityKind::LoginFailed, $user);
            }

            throw $exception;
        }
    }

    /** @return array{id: string, email: string, name: string, locale: string} */
    public function userPayload(User $user): array
    {
        return [
            'id' => $user->getId()->toRfc4122(),
            'email' => $user->getEmail(),
            'name' => $user->getName(),
            'locale' => $user->getLocale(),
        ];
    }

    /** @throws AccountException with device_removed, device_taken or device_key_mismatch */
    public function device(User $user, string $deviceId, string $deviceName, string $publicKey): Device
    {
        // A device is its id together with its key: the id alone is public
        // (D84, SEG-016), and the key is what the login just proved.
        $device = $this->deviceRepository->findOneByDeviceIdAndKey($deviceId, $publicKey);
        if (!$device instanceof Device) {
            if ($this->deviceRepository->findOneOfUser($user, $deviceId) instanceof Device) {
                throw AccountException::deviceKeyMismatch();
            }

            // Another account's row with this id and another key is somebody
            // else's install (or an intruder's useless one): it does not matter.
            $device = new Device($deviceId, $user, $deviceName, $publicKey);
            $this->entityManager->persist($device);

            return $device;
        }

        // Before device_taken: a removed install must start again (D81),
        // whichever account tries it.
        if ($device->isRemoved()) {
            $device->noticeRemoval($this->clock->now()->setTimezone(new \DateTimeZone('UTC')));
            $this->entityManager->flush();

            throw AccountException::deviceRemoved();
        }

        if (!$device->getUser()->getId()->equals($user->getId())) {
            throw AccountException::deviceTaken();
        }

        $device->setName($deviceName);

        return $device;
    }

    /** A new token, which invalidates the previous one; only its hash is kept, and the token is returned once. */
    public function issueToken(Device $device): string
    {
        $token = sodium_bin2base64(random_bytes(32), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
        $device->setTokenHash(hash('sha256', $token));

        return $token;
    }
}
