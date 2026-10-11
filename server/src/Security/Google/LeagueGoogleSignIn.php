<?php

declare(strict_types=1);

namespace App\Security\Google;

use KnpU\OAuth2ClientBundle\Client\ClientRegistry;
use League\OAuth2\Client\Provider\Google;
use League\OAuth2\Client\Provider\GoogleUser;
use League\OAuth2\Client\Token\AccessToken;
use Symfony\Component\DependencyInjection\Attribute\Autowire;
use Symfony\Component\Routing\Generator\UrlGeneratorInterface;

/**
 * Google, for real (D77): `knpuniversity/oauth2-client-bundle` holds the client's configuration and
 * `league/oauth2-google` talks to Google. The state and the PKCE secret are the server's own, bound to the attempt and
 * to the browser (not the bundle's session), so the code that comes back is checked against both. The access token Google
 * gives is used once, to ask who signed in, and dropped.
 */
final readonly class LeagueGoogleSignIn implements GoogleSignIn
{
    private const array SCOPES = ['openid', 'email', 'profile'];

    public function __construct(
        private ClientRegistry $clients,
        private UrlGeneratorInterface $urls,
        #[Autowire(env: 'GOOGLE_CLIENT_ID')]
        private string $clientId,
        #[Autowire(env: 'GOOGLE_CLIENT_SECRET')]
        private string $clientSecret,
    ) {
    }

    public function isConfigured(): bool
    {
        return '' !== trim($this->clientId) && '' !== trim($this->clientSecret);
    }

    public function authorization(string $state): GoogleAuthorization
    {
        $verifier = rtrim(strtr(base64_encode(random_bytes(48)), '+/', '-_'), '=');
        $url = $this->provider()->getAuthorizationUrl([
            'state' => $state,
            'scope' => self::SCOPES,
            'redirect_uri' => $this->redirectUri(),
            'code_challenge' => rtrim(strtr(base64_encode(hash('sha256', $verifier, true)), '+/', '-_'), '='),
            'code_challenge_method' => 'S256',
        ]);

        return new GoogleAuthorization($url, $verifier);
    }

    public function identify(string $code, string $pkceVerifier): GoogleIdentity
    {
        try {
            $provider = $this->provider();
            $token = $provider->getAccessToken('authorization_code', [
                'code' => $code,
                'redirect_uri' => $this->redirectUri(),
                'code_verifier' => $pkceVerifier,
            ]);
            \assert($token instanceof AccessToken);
            $owner = $provider->getResourceOwner($token);
        } catch (\Throwable) {
            // What failed is not said: the exception may carry the code or the answer of Google.
            throw new GoogleSignInException('Google could not confirm the sign-in.');
        }
        if (!$owner instanceof GoogleUser) {
            throw new GoogleSignInException('Google could not confirm the sign-in.');
        }

        $data = $owner->toArray();

        return new GoogleIdentity(
            $this->text($owner->getId()),
            $this->text($owner->getEmail()),
            true === ($data['email_verified'] ?? false),
            $this->text($owner->getName()),
        );
    }

    private function text(mixed $value): string
    {
        return \is_string($value) ? $value : '';
    }

    private function provider(): Google
    {
        $provider = $this->clients->getClient('google')->getOAuth2Provider();
        \assert($provider instanceof Google);

        return $provider;
    }

    /** The address registered at Google for the way back: `DEFAULT_URI` and the route, the same whoever asks. */
    private function redirectUri(): string
    {
        return $this->urls->generate('google_callback', [], UrlGeneratorInterface::ABSOLUTE_URL);
    }
}
