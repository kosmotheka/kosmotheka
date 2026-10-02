<?php

declare(strict_types=1);

namespace App\Controller;

use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\DependencyInjection\Attribute\Autowire;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\Routing\Attribute\Route;

/**
 * What this server offers (D77: "the server says what it offers"), so the
 * app can decide what to show without hard-coding feature availability.
 * `signInWithGoogle` is the one obvious place to flip once that milestone
 * lands; `two_factor` reads TWO_FACTOR_ENABLED (D78).
 */
#[Route('/v1/server')]
final class ServerController extends AbstractController
{
    private const bool REGISTRATION = true;
    private const bool SIGN_IN_WITH_GOOGLE = false;

    public function __construct(
        #[Autowire(env: 'bool:TWO_FACTOR_ENABLED')]
        private readonly bool $twoFactorEnabled,
    ) {
    }

    #[Route('', methods: ['GET'])]
    public function show(): JsonResponse
    {
        return new JsonResponse([
            'name' => 'Kosmotheka',
            'api_version' => 1,
            'registration' => self::REGISTRATION,
            'sign_in_with_google' => self::SIGN_IN_WITH_GOOGLE,
            'two_factor' => $this->twoFactorEnabled,
        ]);
    }
}
