<?php

declare(strict_types=1);

use Symfony\Component\Dotenv\Dotenv;

require __DIR__.'/../vendor/autoload.php';

// No secret lives in the repository: each test run makes up its own, unless
// .env.test.local or the environment already sets one. Dotenv never overrides it.
if (!isset($_SERVER['APP_SECRET']) && false === getenv('APP_SECRET')) {
    $_SERVER['APP_SECRET'] = $_ENV['APP_SECRET'] = bin2hex(random_bytes(32));
}

if (method_exists(Dotenv::class, 'bootEnv')) {
    new Dotenv()->bootEnv(__DIR__.'/../.env');
}

if ($_SERVER['APP_DEBUG']) {
    umask(0000);
}
