<?php

declare(strict_types=1);

namespace App\Tests\Controller;

use App\Tests\Support\DeviceAuthTestTrait;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\MailerAssertionsTrait;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;
use Symfony\Component\Uid\Uuid;

/**
 * D100: the signature a device makes to sign in names the server it is for and what it is for, so that one made for
 * another server, or for signing in with Google, is worth nothing here. The format is
 * `kosmotheka-login-v2\n{origin}\n{purpose}\n{challenge}\n{device_id}`; the old one is not accepted.
 */
final class DeviceProofBindingTest extends WebTestCase
{
    use DeviceAuthTestTrait;
    use MailerAssertionsTrait;

    private const string ORIGIN = 'http://localhost';

    public function testASignatureThatNamesThisServerAndLoginSignsIn(): void
    {
        $client = $this->account('binding-ok@example.com');

        $this->loginSignedAs($client, 'binding-ok@example.com', "kosmotheka-login-v2\n".self::ORIGIN."\nlogin\n%s\n%s");

        self::assertResponseStatusCodeSame(200);
    }

    public function testTheOldFormatIsNotAcceptedAnyMore(): void
    {
        $client = $this->account('binding-old@example.com');

        $this->loginSignedAs($client, 'binding-old@example.com', "kosmotheka-login\n%s\n%s");

        self::assertResponseStatusCodeSame(400);
        self::assertSame('{"error":"invalid_device_proof"}', $client->getResponse()->getContent());
    }

    public function testASignatureMadeForAnotherServerIsRefused(): void
    {
        $client = $this->account('binding-origin@example.com');

        foreach (['https://api.other.example', 'http://localhost:8080', 'https://localhost', ''] as $origin) {
            $this->loginSignedAs($client, 'binding-origin@example.com', "kosmotheka-login-v2\n".$origin."\nlogin\n%s\n%s");
            self::assertResponseStatusCodeSame(400, 'Origin: '.$origin);
            self::assertSame('{"error":"invalid_device_proof"}', $client->getResponse()->getContent());
        }
    }

    public function testASignatureMadeForGoogleIsRefusedForSigningInWithAPassword(): void
    {
        $client = $this->account('binding-google@example.com');

        $this->loginSignedAs($client, 'binding-google@example.com', "kosmotheka-login-v2\n".self::ORIGIN."\ngoogle\n%s\n%s");

        self::assertResponseStatusCodeSame(400);
        self::assertSame('{"error":"invalid_device_proof"}', $client->getResponse()->getContent());
    }

    public function testASpellingOfThisServersOriginThatIsNotTheCanonicalOneIsRefused(): void
    {
        $client = $this->account('binding-spelling@example.com');

        foreach (['HTTP://LOCALHOST', 'http://localhost/', 'http://localhost:80'] as $origin) {
            $this->loginSignedAs($client, 'binding-spelling@example.com', "kosmotheka-login-v2\n".$origin."\nlogin\n%s\n%s");
            self::assertResponseStatusCodeSame(400, 'Origin: '.$origin);
        }
    }

    public function testAnExtraAddressOfThisServerIsAcceptedAndAnyOtherIsNot(): void
    {
        $_ENV['SERVER_EXTRA_ORIGINS'] = $_SERVER['SERVER_EXTRA_ORIGINS'] = 'http://10.0.2.2:8095, http://192.168.1.20:8095';
        try {
            $client = $this->account('binding-extra@example.com');

            $this->loginSignedAs($client, 'binding-extra@example.com', "kosmotheka-login-v2\nhttp://10.0.2.2:8095\nlogin\n%s\n%s");
            self::assertResponseStatusCodeSame(200);

            foreach (['http://10.0.2.2:8096', 'https://api.other.example'] as $origin) {
                $this->loginSignedAs($client, 'binding-extra@example.com', "kosmotheka-login-v2\n".$origin."\nlogin\n%s\n%s");
                self::assertResponseStatusCodeSame(400, $origin);
                self::assertSame('{"error":"invalid_device_proof"}', $client->getResponse()->getContent());
            }
        } finally {
            $_ENV['SERVER_EXTRA_ORIGINS'] = $_SERVER['SERVER_EXTRA_ORIGINS'] = '';
        }
    }

    private function account(string $email): KernelBrowser
    {
        $client = self::createClient();
        self::getContainer()->get('cache.rate_limiter')->clear();
        self::getContainer()->get('cache.auth_challenge')->clear();
        $this->registerAndConfirmForLogin($client, $email, 'correct horse battery');

        return $client;
    }

    /** @param string $format what is signed, with the challenge and the device id where the two `%s` are */
    private function loginSignedAs(KernelBrowser $client, string $email, string $format): void
    {
        $keyPair = sodium_crypto_sign_keypair();
        $deviceId = Uuid::v4()->toRfc4122();
        $challenge = $this->requestChallenge($client)['challenge'];
        $signature = sodium_crypto_sign_detached(\sprintf($format, $challenge, $deviceId), sodium_crypto_sign_secretkey($keyPair));

        $this->requestLogin($client, $email, 'correct horse battery', $deviceId, 'Phone', $this->encode(sodium_crypto_sign_publickey($keyPair)), $challenge, $this->encode($signature));
    }
}
