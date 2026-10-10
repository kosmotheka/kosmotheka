<?php

declare(strict_types=1);

namespace App\Tests\Security;

use App\Security\CloudKeyVault;
use App\Security\CloudKeyVaultException;
use PHPUnit\Framework\TestCase;

/** The vault of the cloud's private keys (D95). */
final class CloudKeyVaultTest extends TestCase
{
    private function secret(string $seed): string
    {
        return base64_encode(hash('sha256', $seed, true));
    }

    public function testWhatIsSealedOpensAgain(): void
    {
        $vault = new CloudKeyVault($this->secret('one'));
        $key = sodium_crypto_sign_secretkey(sodium_crypto_sign_keypair());

        self::assertSame($key, $vault->open($vault->seal($key)));
    }

    public function testAnotherSecretCannotOpenItAndGivesNoBytes(): void
    {
        $sealed = new CloudKeyVault($this->secret('one'))->seal('a private key');

        $this->expectException(CloudKeyVaultException::class);
        new CloudKeyVault($this->secret('two'))->open($sealed);
    }

    public function testTwoSealsOfTheSameValueDiffer(): void
    {
        $vault = new CloudKeyVault($this->secret('one'));

        self::assertNotSame($vault->seal('same'), $vault->seal('same'));
        self::assertStringNotContainsString('same', sodium_base642bin($vault->seal('same'), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING));
    }

    public function testADamagedValueDoesNotOpen(): void
    {
        $vault = new CloudKeyVault($this->secret('one'));
        $bytes = sodium_base642bin($vault->seal('a private key'), \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING);
        $bytes[\strlen($bytes) - 1] = \chr(\ord($bytes[\strlen($bytes) - 1]) ^ 1);

        $this->expectException(CloudKeyVaultException::class);
        $vault->open(sodium_bin2base64($bytes, \SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING));
    }

    public function testWithoutASecretItIsDisabledAndRefusesToSeal(): void
    {
        $vault = new CloudKeyVault('');

        self::assertFalse($vault->isEnabled());
        $this->expectException(CloudKeyVaultException::class);
        $vault->seal('x');
    }

    public function testASecretOfTheWrongSizeIsRefused(): void
    {
        $this->expectException(CloudKeyVaultException::class);
        new CloudKeyVault(base64_encode('too short'))->seal('x');
    }
}
