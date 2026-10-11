<?php

declare(strict_types=1);

namespace App\Security\Google;

/**
 * Everything the server asks of Google, behind one door (D77): tests put a double here and never touch the network.
 * The real one is `LeagueGoogleSignIn`.
 */
interface GoogleSignIn
{
    /** Whether this server has a Google client configured. Without one, no route of Google does anything. */
    public function isConfigured(): bool;

    /** The address to send the browser to. `$state` is the server's own, bound to that browser. */
    public function authorization(string $state): GoogleAuthorization;

    /**
     * Exchanges the code Google handed back, with the PKCE secret of the trip, and says who signed in.
     *
     * @throws GoogleSignInException
     */
    public function identify(string $code, string $pkceVerifier): GoogleIdentity;
}
