<?php

declare(strict_types=1);

namespace App\Security;

use Symfony\Component\DependencyInjection\Attribute\Autowire;

/**
 * The address this server is known by (`DEFAULT_URI`, the public one), written the one way the app writes it too:
 * `scheme://host[:port]`, in lower case, no trailing slash, and the port only when it is not the scheme's own. It is
 * part of what a device signs to sign in (D100), so a signature made for another server is worth nothing here.
 * `app/src-tauri/src/account/origin.rs` is the other side of this and must give the same text.
 *
 * `SERVER_EXTRA_ORIGINS` (empty by default; in production always empty) lists other addresses by which this same
 * server is reached, for development: the desktop at `localhost:8095`, the Android emulator at `10.0.2.2:8095`, a
 * phone by the address of the home network. A signature is good for the main address or any of these, never another.
 */
final readonly class ServerOrigin
{
    private string $origin;

    /** @var list<string> */
    private array $accepted;

    public function __construct(
        #[Autowire(env: 'DEFAULT_URI')]
        string $uri,
        #[Autowire(env: 'SERVER_EXTRA_ORIGINS')]
        string $extra = '',
    ) {
        $this->origin = $this->normalise($uri, 'DEFAULT_URI must be the public address of the server, with http or https and a host (docs/development/server.md).');

        $accepted = [$this->origin];
        foreach (preg_split('/[\s,;]+/', $extra, flags: \PREG_SPLIT_NO_EMPTY) ?: [] as $address) {
            $accepted[] = $this->normalise($address, 'SERVER_EXTRA_ORIGINS holds addresses with http or https and a host, separated by commas.');
        }
        $this->accepted = array_values(array_unique($accepted));
    }

    /** The main address, the one the server gives out (Google's way back, for one). */
    public function toString(): string
    {
        return $this->origin;
    }

    /** @return list<string> the main address and the extra ones, each written the one way */
    public function accepted(): array
    {
        return $this->accepted;
    }

    private function normalise(string $uri, string $message): string
    {
        $parts = parse_url($uri);
        $scheme = \is_array($parts) ? strtolower($parts['scheme'] ?? '') : '';
        $host = \is_array($parts) ? strtolower($parts['host'] ?? '') : '';
        if (!\in_array($scheme, ['http', 'https'], true) || '' === $host) {
            throw new \LogicException($message);
        }

        $port = $parts['port'] ?? null;
        $default = 'https' === $scheme ? 443 : 80;

        return $scheme.'://'.$host.(null === $port || $default === $port ? '' : ':'.$port);
    }
}
