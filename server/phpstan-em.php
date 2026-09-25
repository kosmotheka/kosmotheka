<?php

declare(strict_types=1);

// Gives phpstan/phpstan-doctrine a real EntityManager. Without it, it
// understands the entities but cannot validate DQL; with it, a query with a
// misspelled field fails the analysis instead of production.

use App\Kernel;
use Doctrine\ORM\EntityManagerInterface;
use Symfony\Component\Dotenv\Dotenv;

require __DIR__.'/vendor/autoload.php';

(new Dotenv())->bootEnv(__DIR__.'/.env');

$kernel = new Kernel($_SERVER['APP_ENV'] ?? 'dev', (bool) ($_SERVER['APP_DEBUG'] ?? true));
$kernel->boot();

$em = $kernel->getContainer()->get('doctrine')->getManager();
assert($em instanceof EntityManagerInterface);

return $em;
