<?php

declare(strict_types=1);

namespace App\Tests\Support;

use App\Security\DeviceProof;
use OTPHP\TOTP;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Component\Mime\Email;
use Symfony\Component\Uid\Uuid;

/**
 * Registers, confirms and logs a device in with a real Ed25519 key pair
 * (milestone 7.1, part 2), so a test can call an authenticated endpoint
 * without repeating the whole sign-in dance. Mixed into a WebTestCase that
 * also uses Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait, whose
 * static methods it calls as if they were its own.
 */
trait DeviceAuthTestTrait
{
    /**
     * @return array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string}
     */
    private function signInNewDevice(KernelBrowser $client, string $email, string $password = 'correct horse battery', string $deviceName = 'Test device'): array
    {
        $this->registerAndConfirmForLogin($client, $email, $password);

        return $this->loginDevice($client, $email, $password, Uuid::v4()->toRfc4122(), $deviceName);
    }

    private function registerAndConfirmForLogin(KernelBrowser $client, string $email, string $password): void
    {
        $client->request('POST', '/v1/account/register', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode([
            'email' => $email,
            'password' => $password,
            'name' => 'Ada',
            'locale' => 'en',
        ], \JSON_THROW_ON_ERROR));

        $message = self::getMailerMessage();
        \assert($message instanceof Email);
        preg_match('/\d{6}/', (string) $message->getTextBody(), $matches);

        $client->request('POST', '/v1/account/confirm', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode([
            'email' => $email,
            'code' => $matches[0] ?? self::fail('No code in the email.'),
        ], \JSON_THROW_ON_ERROR));
    }

    /**
     * @return array{challenge: string, expires_at: string}
     */
    private function requestChallenge(KernelBrowser $client): array
    {
        $client->request('POST', '/v1/auth/challenge', server: ['CONTENT_TYPE' => 'application/json']);

        /** @var array{challenge: string, expires_at: string} $data */
        $data = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);

        return $data;
    }

    /**
     * @return array{headers: array<string, string>, device_id: string, token: string, public_key: string, secret_key: string, email: string}
     */
    private function loginDevice(KernelBrowser $client, string $email, string $password, string $deviceId, string $deviceName): array
    {
        $keyPair = sodium_crypto_sign_keypair();
        $publicKey = $this->encode(sodium_crypto_sign_publickey($keyPair));
        $secretKey = sodium_crypto_sign_secretkey($keyPair);

        $challenge = $this->requestChallenge($client)['challenge'];
        $signature = $this->sign($challenge, $deviceId, $secretKey);

        $this->requestLogin($client, $email, $password, $deviceId, $deviceName, $publicKey, $challenge, $signature);

        /** @var array{token: string, user: array<string, mixed>} $data */
        $data = json_decode((string) $client->getResponse()->getContent(), true, flags: \JSON_THROW_ON_ERROR);

        return [
            'headers' => $this->deviceHeaders($deviceId, $data['token']),
            'device_id' => $deviceId,
            'token' => $data['token'],
            'public_key' => $publicKey,
            'secret_key' => $this->encode($secretKey),
            'email' => $email,
        ];
    }

    private function requestLogin(KernelBrowser $client, string $email, string $password, string $deviceId, string $deviceName, string $publicKey, string $challenge, string $signature): void
    {
        $client->request('POST', '/v1/auth/login', server: ['CONTENT_TYPE' => 'application/json'], content: json_encode([
            'email' => $email,
            'password' => $password,
            'device_id' => $deviceId,
            'device_name' => $deviceName,
            'public_key' => $publicKey,
            'challenge' => $challenge,
            'signature' => $signature,
        ], \JSON_THROW_ON_ERROR));
    }

    /** @return array{HTTP_X-Device-Id: string, HTTP_Authorization: string} */
    private function deviceHeaders(string $deviceId, string $token): array
    {
        return [
            'HTTP_X-Device-Id' => $deviceId,
            'HTTP_Authorization' => 'Bearer '.$token,
        ];
    }

    private function sign(string $challenge, string $deviceId, string $secretKey): string
    {
        $message = \sprintf(DeviceProof::MESSAGE_FORMAT, $challenge, $deviceId);
        \assert('' !== $secretKey);

        return $this->encode(sodium_crypto_sign_detached($message, $secretKey));
    }

    private function encode(string $bytes): string
    {
        return sodium_bin2base64($bytes, \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
    }

    /** The TOTP code an authenticator app would show for `$secret` at `$at`. */
    private static function totpAt(string $secret, string $at): string
    {
        $time = new \DateTimeImmutable($at)->getTimestamp();
        \assert('' !== $secret && $time >= 0);

        return TOTP::createFromSecret($secret)->at($time);
    }
}
