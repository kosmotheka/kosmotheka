<?php

declare(strict_types=1);

namespace App\Controller;

use App\Metadata\BookProvider;
use App\Metadata\ComicVineSource;
use App\Metadata\RawgSource;
use App\Metadata\TmdbSource;
use App\Security\Google\GoogleSignIn;
use App\Security\RegistrationGate;
use App\Sync\HubSettings;
use App\Sync\Invitation\InvitationSettings;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\DependencyInjection\Attribute\Autowire;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\Routing\Attribute\Route;

/**
 * What this server offers (D77: "the server says what it offers"), so the
 * app can decide what to show without hard-coding feature availability.
 * `sign_in_with_google` is whether a Google client is configured (D77); `two_factor` reads TWO_FACTOR_ENABLED (D78) and `events` is
 * whether a Mercure hub is configured (D75) and `invitations`
 * whether the server has an address for invitation links, which `invitation_base`
 * gives (D43, D79).
 */
#[Route('/v1/server')]
final class ServerController extends AbstractController
{
    private const bool REGISTRATION = true;

    public function __construct(
        #[Autowire(env: 'bool:TWO_FACTOR_ENABLED')]
        private readonly bool $twoFactorEnabled,
        private readonly HubSettings $hub,
        private readonly InvitationSettings $invitations,
        private readonly RegistrationGate $registrationGate,
        private readonly GoogleSignIn $google,
        private readonly BookProvider $books,
        private readonly TmdbSource $tmdb,
        private readonly RawgSource $rawg,
        private readonly ComicVineSource $comicVine,
    ) {
    }

    #[Route('', methods: ['GET'])]
    public function show(): JsonResponse
    {
        return new JsonResponse([
            'name' => 'Kosmotheka',
            'api_version' => 1,
            'registration' => self::REGISTRATION,
            // Only some addresses may make an account; which ones is never said.
            'registration_restricted' => $this->registrationGate->isRestricted(),
            'sign_in_with_google' => $this->google->isConfigured(),
            'two_factor' => $this->twoFactorEnabled,
            'events' => $this->hub->isEnabled(),
            'invitations' => $this->invitations->isEnabled(),
            'invitation_base' => $this->invitations->baseUrl(),
            // Catalogues the server looks books up in for its accounts (7.8, D83); a new one is added here and in the app.
            'metadata' => array_values(array_filter([
                $this->books->isConfigured() ? $this->books->name() : null,
                $this->tmdb->isConfigured() ? TmdbSource::NAME : null,
                $this->rawg->isConfigured() ? RawgSource::NAME : null,
                $this->comicVine->isConfigured() ? ComicVineSource::NAME : null,
            ])),
        ]);
    }
}
