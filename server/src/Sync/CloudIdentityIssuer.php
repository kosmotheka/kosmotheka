<?php

declare(strict_types=1);

namespace App\Sync;

use App\Entity\CloudDeviceKey;
use App\Entity\CloudLibrary;
use App\Security\CloudKeyVault;
use Doctrine\DBAL\LockMode;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Uid\Uuid;

/**
 * Gives a cloud library its own device (D76, D95): an Ed25519 key pair the server generates and the id that derives from its public key (CloudDeviceId), the
 * private half encrypted by CloudKeyVault. The public half is registered at once as the key of that id in
 * the library (`CloudDeviceKey`), so the rule that a registered device never changes its key stops anybody
 * from taking the id or signing in its name. Once generated it never changes. With no secret on the server
 * (D18) nothing is generated and the library has no cloud device.
 */
final readonly class CloudIdentityIssuer
{
    public function __construct(private EntityManagerInterface $entityManager, private CloudKeyVault $vault)
    {
    }

    /**
     * For a library that is being created: nobody else knows it yet, so there is nothing to lock. Persists, does not flush.
     */
    public function issueForNew(CloudLibrary $library): void
    {
        if (!$this->vault->isEnabled() || $library->hasCloudIdentity()) {
            return;
        }
        $this->generate($library);
    }

    /**
     * For a library created before the server had a secret, or by an older version. Takes the library's lock
     * inside a transaction, looks again, and only then generates, so two requests cannot give it two identities.
     */
    public function ensure(CloudLibrary $library): void
    {
        if (!$this->vault->isEnabled() || $library->hasCloudIdentity()) {
            return;
        }
        $this->entityManager->wrapInTransaction(function () use ($library): void {
            $this->entityManager->refresh($library, LockMode::PESSIMISTIC_WRITE);
            if (!$library->hasCloudIdentity()) {
                $this->generate($library);
                $this->entityManager->flush();
            }
        });
    }

    private function generate(CloudLibrary $library): void
    {
        $pair = sodium_crypto_sign_keypair();
        $publicKey = sodium_bin2base64(sodium_crypto_sign_publickey($pair), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
        // The id comes from the key (D95): nobody can register that id with another key and be believed as the cloud.
        $deviceId = Uuid::fromString(CloudDeviceId::fromPublicKey($publicKey) ?? throw new \LogicException('A freshly generated key is always a key.'));
        $library->giveCloudIdentity($deviceId, $publicKey, $this->vault->seal(sodium_crypto_sign_secretkey($pair)));
        $this->entityManager->persist(new CloudDeviceKey($library, $deviceId->toRfc4122(), $publicKey));
        sodium_memzero($pair);
    }
}
