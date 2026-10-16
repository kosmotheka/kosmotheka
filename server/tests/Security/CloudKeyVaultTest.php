<?php

declare(strict_types=1);

namespace App\Tests\Security;

use App\Security\CloudKeyVault;
use App\Security\CloudKeyVaultException;
use PHPUnit\Framework\TestCase;

/** The vault of the cloud's private keys (D95, SEG-074). */
final class CloudKeyVaultTest extends TestCase
{
    private const string LIBRARY = '01997c7e-0000-7000-8000-00000000aaaa';
    private const string ANOTHER = '01997c7e-0000-7000-8000-00000000bbbb';

    private function secret(string $seed): string
    {
        return base64_encode(hash('sha256', $seed, true));
    }

    public function testWhatIsSealedOpensAgain(): void
    {
        $vault = new CloudKeyVault($this->secret('one'));
        $key = sodium_crypto_sign_secretkey(sodium_crypto_sign_keypair());

        self::assertSame($key, $vault->open(self::LIBRARY, $vault->seal(self::LIBRARY, $key)));
    }

    /** SEG-074: the sealed key of a library, copied into the row of another, does not open there. */
    public function testWhatIsSealedForOneLibraryDoesNotOpenForAnother(): void
    {
        $vault = new CloudKeyVault($this->secret('one'));
        $sealed = $vault->seal(self::LIBRARY, 'a private key');

        $this->expectException(CloudKeyVaultException::class);
        $vault->open(self::ANOTHER, $sealed);
    }

    public function testAValueHasAVersionAndNoPlainText(): void
    {
        $vault = new CloudKeyVault($this->secret('one'));
        $sealed = $vault->seal(self::LIBRARY, 'same');

        self::assertStringStartsWith('v1.', $sealed);
        self::assertFalse(CloudKeyVault::isLegacy($sealed));
        self::assertStringNotContainsString('same', sodium_base642bin(substr($sealed, 3), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING));
    }

    /** What was stored before SEG-074: `secretbox` with a random nonce in front, no version, tied to nothing. */
    public function testAValueOfTheOldKindStillOpensAndIsRecognisedAsOld(): void
    {
        $secret = $this->secret('one');
        $nonce = random_bytes(\SODIUM_CRYPTO_SECRETBOX_NONCEBYTES);
        $old = sodium_bin2base64($nonce.sodium_crypto_secretbox('an old private key', $nonce, sodium_base642bin($secret, \SODIUM_BASE64_VARIANT_ORIGINAL)), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);

        self::assertTrue(CloudKeyVault::isLegacy($old));
        self::assertSame('an old private key', new CloudKeyVault($secret)->open(self::LIBRARY, $old));
    }

    public function testAnotherSecretCannotOpenItAndGivesNoBytes(): void
    {
        $sealed = new CloudKeyVault($this->secret('one'))->seal(self::LIBRARY, 'a private key');

        $this->expectException(CloudKeyVaultException::class);
        new CloudKeyVault($this->secret('two'))->open(self::LIBRARY, $sealed);
    }

    public function testTwoSealsOfTheSameValueDiffer(): void
    {
        $vault = new CloudKeyVault($this->secret('one'));

        self::assertNotSame($vault->seal(self::LIBRARY, 'same'), $vault->seal(self::LIBRARY, 'same'));
    }

    public function testADamagedValueDoesNotOpen(): void
    {
        $vault = new CloudKeyVault($this->secret('one'));
        $bytes = sodium_base642bin(substr($vault->seal(self::LIBRARY, 'a private key'), 3), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
        $bytes[\strlen($bytes) - 1] = \chr(\ord($bytes[\strlen($bytes) - 1]) ^ 1);

        $this->expectException(CloudKeyVaultException::class);
        $vault->open(self::LIBRARY, 'v1.'.sodium_bin2base64($bytes, \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING));
    }

    public function testAnotherVersionDoesNotOpen(): void
    {
        $vault = new CloudKeyVault($this->secret('one'));
        $sealed = $vault->seal(self::LIBRARY, 'a private key');

        // Read as an old value, which it is not: it must fail, not give bytes.
        $this->expectException(CloudKeyVaultException::class);
        $vault->open(self::LIBRARY, 'v2.'.substr($sealed, 3));
    }

    public function testWithoutASecretItIsDisabledAndRefusesToSeal(): void
    {
        $vault = new CloudKeyVault('');

        self::assertFalse($vault->isEnabled());
        $this->expectException(CloudKeyVaultException::class);
        $vault->seal(self::LIBRARY, 'x');
    }

    public function testASecretOfTheWrongSizeIsRefused(): void
    {
        $this->expectException(CloudKeyVaultException::class);
        new CloudKeyVault(base64_encode('too short'))->seal(self::LIBRARY, 'x');
    }
}
