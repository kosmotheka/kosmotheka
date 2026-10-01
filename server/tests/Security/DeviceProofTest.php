<?php

declare(strict_types=1);

namespace App\Tests\Security;

use App\Security\AccountException;
use App\Security\DeviceProof;
use PHPUnit\Framework\TestCase;

/** DeviceProof::verify (milestone 7.1, part 2), with a real Ed25519 key pair. */
final class DeviceProofTest extends TestCase
{
    public function testVerifyAcceptsAValidSignature(): void
    {
        $keyPair = sodium_crypto_sign_keypair();
        $publicKey = $this->encode(sodium_crypto_sign_publickey($keyPair));
        $secretKey = sodium_crypto_sign_secretkey($keyPair);
        $challenge = $this->encode(random_bytes(32));
        $deviceId = '0196b2f1-0000-7000-8000-000000000000';

        $signature = $this->sign($challenge, $deviceId, $secretKey);

        new DeviceProof()->verify($challenge, $deviceId, $publicKey, $signature);
        $this->addToAssertionCount(1);
    }

    public function testVerifyRejectsASignatureForAnotherDeviceId(): void
    {
        $keyPair = sodium_crypto_sign_keypair();
        $publicKey = $this->encode(sodium_crypto_sign_publickey($keyPair));
        $secretKey = sodium_crypto_sign_secretkey($keyPair);
        $challenge = $this->encode(random_bytes(32));
        $signature = $this->sign($challenge, 'one-device', $secretKey);

        $this->expectInvalidDeviceProof();

        new DeviceProof()->verify($challenge, 'another-device', $publicKey, $signature);
    }

    public function testVerifyRejectsASignatureForAnotherChallenge(): void
    {
        $keyPair = sodium_crypto_sign_keypair();
        $publicKey = $this->encode(sodium_crypto_sign_publickey($keyPair));
        $secretKey = sodium_crypto_sign_secretkey($keyPair);
        $deviceId = 'a-device';
        $signature = $this->sign($this->encode(random_bytes(32)), $deviceId, $secretKey);

        $this->expectInvalidDeviceProof();

        new DeviceProof()->verify($this->encode(random_bytes(32)), $deviceId, $publicKey, $signature);
    }

    public function testVerifyRejectsAMalformedPublicKey(): void
    {
        $this->expectInvalidDeviceProof();

        new DeviceProof()->verify('whatever', 'a-device', 'not-base64url-of-32-bytes', 'whatever-signature');
    }

    public function testVerifyRejectsAMalformedSignature(): void
    {
        $keyPair = sodium_crypto_sign_keypair();
        $publicKey = $this->encode(sodium_crypto_sign_publickey($keyPair));

        $this->expectInvalidDeviceProof();

        new DeviceProof()->verify('whatever', 'a-device', $publicKey, 'not-base64url-of-64-bytes');
    }

    private function expectInvalidDeviceProof(): void
    {
        $this->expectException(AccountException::class);
        $this->expectExceptionObject(AccountException::invalidDeviceProof());
    }

    private function sign(string $challenge, string $deviceId, string $secretKey): string
    {
        $message = \sprintf(DeviceProof::MESSAGE_FORMAT, $challenge, $deviceId);

        return $this->encode(sodium_crypto_sign_detached($message, $secretKey));
    }

    private function encode(string $bytes): string
    {
        return sodium_bin2base64($bytes, \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
    }
}
