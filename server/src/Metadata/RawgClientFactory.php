<?php

declare(strict_types=1);

namespace App\Metadata;

use Symfony\Component\HttpClient\HttpClient;
use Symfony\Contracts\HttpClient\HttpClientInterface;

/**
 * The http client RAWG is asked with. RAWG only takes its key as a parameter of the address, so this client is made
 * apart and never given a logger: the framework's own client logs every request with its address (at the `info`
 * level, which a server turned up to debug would write), and here that address holds the key.
 */
final class RawgClientFactory
{
    public static function create(): HttpClientInterface
    {
        return HttpClient::create();
    }
}
