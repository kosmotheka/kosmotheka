<?php

declare(strict_types=1);

namespace App\Tests\Security;

use App\Enum\DeviceProofPurpose;
use App\Security\AccountException;
use App\Security\DeviceProof;
use App\Security\ServerOrigin;
use PHPUnit\Framework\TestCase;

/** DeviceProof::verify (milestone 7.1, part 2, bound to server and purpose by D100), with a real Ed25519 key pair. */
final class DeviceProofTest extends TestCase
{
    private const string ORIGIN = 'https://api.kosmotheka.app';

    public function testVerifyAcceptsAValidSignature(): void
    {
        [$publicKey, $secretKey] = $this->keys();
        $challenge = $this->encode(random_bytes(32));
        $deviceId = '0196b2f1-0000-7000-8000-000000000000';

        $signature = $this->sign($challenge, $deviceId, $secretKey);

        $this->proof()->verify(DeviceProofPurpose::Login, $challenge, $deviceId, $publicKey, $signature);
        $this->addToAssertionCount(1);
    }

    public function testVerifyRejectsASignatureForAnotherDeviceId(): void
    {
        [$publicKey, $secretKey] = $this->keys();
        $challenge = $this->encode(random_bytes(32));
        $signature = $this->sign($challenge, 'one-device', $secretKey);

        $this->expectInvalidDeviceProof();

        $this->proof()->verify(DeviceProofPurpose::Login, $challenge, 'another-device', $publicKey, $signature);
    }

    public function testVerifyRejectsASignatureForAnotherChallenge(): void
    {
        [$publicKey, $secretKey] = $this->keys();
        $deviceId = 'a-device';
        $signature = $this->sign($this->encode(random_bytes(32)), $deviceId, $secretKey);

        $this->expectInvalidDeviceProof();

        $this->proof()->verify(DeviceProofPurpose::Login, $this->encode(random_bytes(32)), $deviceId, $publicKey, $signature);
    }

    public function testVerifyRejectsASignatureMadeForAnotherServer(): void
    {
        [$publicKey, $secretKey] = $this->keys();
        $challenge = $this->encode(random_bytes(32));
        $signature = $this->sign($challenge, 'a-device', $secretKey, origin: 'https://api.elsewhere.example');

        $this->expectInvalidDeviceProof();

        $this->proof()->verify(DeviceProofPurpose::Login, $challenge, 'a-device', $publicKey, $signature);
    }

    public function testVerifyRejectsASignatureMadeForAnotherPurpose(): void
    {
        [$publicKey, $secretKey] = $this->keys();
        $challenge = $this->encode(random_bytes(32));
        $signature = $this->sign($challenge, 'a-device', $secretKey, DeviceProofPurpose::Google);

        $this->expectInvalidDeviceProof();

        $this->proof()->verify(DeviceProofPurpose::Login, $challenge, 'a-device', $publicKey, $signature);
    }

    public function testVerifyRejectsTheOldFormat(): void
    {
        [$publicKey, $secretKey] = $this->keys();
        $challenge = $this->encode(random_bytes(32));
        $signature = $this->encode(sodium_crypto_sign_detached(\sprintf("kosmotheka-login\n%s\n%s", $challenge, 'a-device'), $secretKey));

        $this->expectInvalidDeviceProof();

        $this->proof()->verify(DeviceProofPurpose::Login, $challenge, 'a-device', $publicKey, $signature);
    }

    public function testVerifyAcceptsTheOtherAddressesOfThisServerAndNoOthers(): void
    {
        [$publicKey, $secretKey] = $this->keys();
        $challenge = $this->encode(random_bytes(32));
        $proof = new DeviceProof(new ServerOrigin(self::ORIGIN, ' http://10.0.2.2:8095 , HTTP://192.168.1.20:8095/ '));

        foreach ([self::ORIGIN, 'http://10.0.2.2:8095', 'http://192.168.1.20:8095'] as $origin) {
            $proof->verify(DeviceProofPurpose::Login, $challenge, 'a-device', $publicKey, $this->sign($challenge, 'a-device', $secretKey, origin: $origin));
        }
        $this->addToAssertionCount(1);

        // Neither the main address nor an extra: a signature for any other server is worth nothing.
        foreach (['https://api.elsewhere.example', 'http://10.0.2.2:8096', 'http://10.0.2.3:8095', 'HTTP://10.0.2.2:8095'] as $origin) {
            try {
                $proof->verify(DeviceProofPurpose::Login, $challenge, 'a-device', $publicKey, $this->sign($challenge, 'a-device', $secretKey, origin: $origin));
                self::fail('Accepted a signature for '.$origin);
            } catch (AccountException $exception) {
                self::assertSame(AccountException::invalidDeviceProof()->errorCode, $exception->errorCode);
            }
        }
    }

    public function testWithNoExtraAddressesOnlyTheMainOneIsAccepted(): void
    {
        [$publicKey, $secretKey] = $this->keys();
        $challenge = $this->encode(random_bytes(32));

        $this->expectInvalidDeviceProof();

        new DeviceProof(new ServerOrigin(self::ORIGIN, ''))->verify(DeviceProofPurpose::Login, $challenge, 'a-device', $publicKey, $this->sign($challenge, 'a-device', $secretKey, origin: 'http://10.0.2.2:8095'));
    }

    public function testAnExtraAddressThatIsNotAnOriginIsRefusedWhenTheServerStarts(): void
    {
        $this->expectException(\LogicException::class);

        new ServerOrigin(self::ORIGIN, 'http://10.0.2.2:8095, not-an-address');
    }

    public function testVerifyRejectsAMalformedPublicKey(): void
    {
        $this->expectInvalidDeviceProof();

        $this->proof()->verify(DeviceProofPurpose::Login, 'whatever', 'a-device', 'not-base64url-of-32-bytes', 'whatever-signature');
    }

    public function testVerifyRejectsAMalformedSignature(): void
    {
        [$publicKey] = $this->keys();

        $this->expectInvalidDeviceProof();

        $this->proof()->verify(DeviceProofPurpose::Login, 'whatever', 'a-device', $publicKey, 'not-base64url-of-64-bytes');
    }

    /** The origin is written one way, and the app writes it the same (app/src-tauri/src/account/origin.rs). */
    public function testTheOriginIsWrittenInOneWay(): void
    {
        self::assertSame('https://api.kosmotheka.app', new ServerOrigin('https://API.Kosmotheka.app/')->toString());
        self::assertSame('https://api.kosmotheka.app', new ServerOrigin('https://api.kosmotheka.app:443/v1/server')->toString());
        self::assertSame('http://localhost', new ServerOrigin('http://localhost:80')->toString());
        self::assertSame('http://localhost:8095', new ServerOrigin('http://localhost:8095')->toString());
        self::assertSame('https://localhost:80', new ServerOrigin('https://localhost:80')->toString());
    }

    public function testAnAddressThatIsNotAnOriginIsRefusedWhenTheServerStarts(): void
    {
        $this->expectException(\LogicException::class);

        new ServerOrigin('api.kosmotheka.app');
    }

    private function proof(): DeviceProof
    {
        return new DeviceProof(new ServerOrigin(self::ORIGIN));
    }

    /** @return array{0: string, 1: non-empty-string} public key (encoded) and secret key */
    private function keys(): array
    {
        $keyPair = sodium_crypto_sign_keypair();
        $secret = sodium_crypto_sign_secretkey($keyPair);

        return [$this->encode(sodium_crypto_sign_publickey($keyPair)), $secret];
    }

    private function expectInvalidDeviceProof(): void
    {
        $this->expectException(AccountException::class);
        $this->expectExceptionObject(AccountException::invalidDeviceProof());
    }

    /** @param non-empty-string $secretKey */
    private function sign(string $challenge, string $deviceId, string $secretKey, DeviceProofPurpose $purpose = DeviceProofPurpose::Login, string $origin = self::ORIGIN): string
    {
        $message = \sprintf(DeviceProof::MESSAGE_FORMAT, $origin, $purpose->value, $challenge, $deviceId);

        return $this->encode(sodium_crypto_sign_detached($message, $secretKey));
    }

    private function encode(string $bytes): string
    {
        return sodium_bin2base64($bytes, \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
    }
}
