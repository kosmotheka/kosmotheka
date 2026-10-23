<?php

declare(strict_types=1);

namespace App\Controller;

use App\Dto\BggSearchRequest;
use App\Dto\ComicVineSearchRequest;
use App\Dto\GameUpcChoiceRequest;
use App\Dto\GameUpcLookupRequest;
use App\Dto\RawgSearchRequest;
use App\Dto\TmdbSearchRequest;
use App\Entity\User;
use App\Metadata\BggSearch;
use App\Metadata\BggSource;
use App\Metadata\BookLookup;
use App\Metadata\BookMetadata;
use App\Metadata\BookProvider;
use App\Metadata\ComicVineSearch;
use App\Metadata\ComicVineSource;
use App\Metadata\GameUpcLookup;
use App\Metadata\GameUpcSource;
use App\Metadata\Isbn;
use App\Metadata\RawgSearch;
use App\Metadata\RawgSource;
use App\Metadata\TmdbSearch;
use App\Metadata\TmdbSource;
use App\Security\AccountException;
use App\Security\RateLimitKey;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Attribute\MapRequestPayload;
use Symfony\Component\HttpKernel\Attribute\RateLimit;
use Symfony\Component\Routing\Attribute\Route;
use Symfony\Component\Routing\Requirement\Requirement;
use Symfony\Component\Security\Http\Attribute\CurrentUser;
use Symfony\Component\Serializer\Normalizer\AbstractNormalizer;

/**
 * Books looked up through the server for accounts that have one (7.8, D83). Thin: the cache, the limits and the call
 * live in App\Metadata. What an account searches is never written anywhere: not to the activity log, not to a log.
 */
#[Route('/v1/metadata')]
final class MetadataController extends AbstractController
{
    public function __construct(
        private readonly BookProvider $googleBooks,
        private readonly BookLookup $lookup,
        private readonly TmdbSource $tmdbSource,
        private readonly TmdbSearch $tmdbSearch,
        private readonly RawgSource $rawgSource,
        private readonly RawgSearch $rawgSearch,
        private readonly BggSource $bggSource,
        private readonly BggSearch $bggSearch,
        private readonly ComicVineSource $comicVineSource,
        private readonly ComicVineSearch $comicVineSearch,
        private readonly GameUpcSource $gameUpcSource,
        private readonly GameUpcLookup $gameUpcLookup,
    ) {
    }

    // `.+`: whatever hangs from this address reaches here and is refused as an ISBN. An address the router does not
    // know is written whole to the application log, and this one holds what an account looks up.
    #[Route('/google-books/isbn/{isbn}', requirements: ['isbn' => Requirement::CATCH_ALL], methods: ['GET'])]
    #[RateLimit('metadata_account', key: RateLimitKey::account(...))]
    public function googleBooksIsbn(string $isbn, #[CurrentUser] User $user): JsonResponse
    {
        if (!$this->googleBooks->isConfigured()) {
            throw AccountException::metadataUnavailable();
        }

        try {
            $code = Isbn::parse($isbn);
        } catch (\InvalidArgumentException) {
            throw AccountException::invalidIsbn();
        }

        $book = $this->lookup->lookup($this->googleBooks, $code, $user->getId()->toRfc4122());

        $response = new JsonResponse($book instanceof BookMetadata ? $book->toArray() : BookMetadata::notFoundPayload());
        $response->headers->set('Cache-Control', 'no-store');

        return $response;
    }

    /** A POST on purpose: the title a person looks for is never in an address (an address is written to logs). */
    #[Route('/tmdb/search', methods: ['POST'])]
    #[RateLimit('metadata_account', key: RateLimitKey::account(...))]
    public function tmdbSearch(#[MapRequestPayload(serializationContext: [AbstractNormalizer::ALLOW_EXTRA_ATTRIBUTES => false])] TmdbSearchRequest $payload, #[CurrentUser] User $user): JsonResponse
    {
        if (!$this->tmdbSource->isConfigured()) {
            throw AccountException::metadataUnavailable();
        }

        $candidates = $this->tmdbSearch->search($payload->mediaKind(), $payload->askedLanguage(), TmdbSearch::normalise($payload->query), $user->getId()->toRfc4122());

        $response = new JsonResponse(['candidates' => $candidates]);
        $response->headers->set('Cache-Control', 'no-store');

        return $response;
    }

    /** A POST on purpose, as TMDB's: the title a person looks for is never in an address. */
    #[Route('/rawg/search', methods: ['POST'])]
    #[RateLimit('metadata_account', key: RateLimitKey::account(...))]
    public function rawgSearch(#[MapRequestPayload(serializationContext: [AbstractNormalizer::ALLOW_EXTRA_ATTRIBUTES => false])] RawgSearchRequest $payload, #[CurrentUser] User $user): JsonResponse
    {
        if (!$this->rawgSource->isConfigured()) {
            throw AccountException::metadataUnavailable();
        }

        $response = new JsonResponse(['candidates' => $this->rawgSearch->search($payload->query, $user->getId()->toRfc4122())]);
        $response->headers->set('Cache-Control', 'no-store');

        return $response;
    }

    /** A POST on purpose, as the others': the title a person looks for is never in an address. */
    #[Route('/comic-vine/search', methods: ['POST'])]
    #[RateLimit('metadata_account', key: RateLimitKey::account(...))]
    public function comicVineSearch(#[MapRequestPayload(serializationContext: [AbstractNormalizer::ALLOW_EXTRA_ATTRIBUTES => false])] ComicVineSearchRequest $payload, #[CurrentUser] User $user): JsonResponse
    {
        if (!$this->comicVineSource->isConfigured()) {
            throw AccountException::metadataUnavailable();
        }

        $response = new JsonResponse(['candidates' => $this->comicVineSearch->search($payload->query, $user->getId()->toRfc4122())]);
        $response->headers->set('Cache-Control', 'no-store');

        return $response;
    }

    /** A POST on purpose, as the others': the title a person looks for is never in an address. */
    #[Route('/bgg/search', methods: ['POST'])]
    #[RateLimit('metadata_account', key: RateLimitKey::account(...))]
    public function bggSearch(#[MapRequestPayload(serializationContext: [AbstractNormalizer::ALLOW_EXTRA_ATTRIBUTES => false])] BggSearchRequest $payload, #[CurrentUser] User $user): JsonResponse
    {
        if (!$this->bggSource->isConfigured()) {
            throw AccountException::metadataUnavailable();
        }

        $response = new JsonResponse(['candidates' => $this->bggSearch->search($payload->searchScope(), $payload->query, $user->getId()->toRfc4122())]);
        $response->headers->set('Cache-Control', 'no-store');

        return $response;
    }

    /**
     * Which board game a barcode is (D62): a POST, as the searches, so that what a person scans is never in an address.
     * The answer says whether GameUPC is sure (`verified`), has guesses for the person to choose from (`choose`) or
     * knows nothing of it (`unknown`).
     */
    #[Route('/gameupc/lookup', methods: ['POST'])]
    #[RateLimit('metadata_account', key: RateLimitKey::account(...))]
    public function gameUpcLookup(#[MapRequestPayload(serializationContext: [AbstractNormalizer::ALLOW_EXTRA_ATTRIBUTES => false])] GameUpcLookupRequest $payload, #[CurrentUser] User $user): JsonResponse
    {
        if (!$this->gameUpcSource->isConfigured()) {
            throw AccountException::metadataUnavailable();
        }

        $response = new JsonResponse($this->gameUpcLookup->lookup($payload->barcode(), $user->getId()->toRfc4122())->toArray());
        $response->headers->set('Cache-Control', 'no-store');

        return $response;
    }

    /**
     * The game a person chose for a barcode, among GameUPC's guesses or by searching BoardGameGeek, sent back to
     * GameUPC under the account's GameUPC id: that is how its data grows, and it asked us to do it.
     */
    #[Route('/gameupc/choice', methods: ['POST'])]
    #[RateLimit('metadata_account', key: RateLimitKey::account(...))]
    public function gameUpcChoice(#[MapRequestPayload(serializationContext: [AbstractNormalizer::ALLOW_EXTRA_ATTRIBUTES => false])] GameUpcChoiceRequest $payload, #[CurrentUser] User $user): JsonResponse
    {
        if (!$this->gameUpcSource->isConfigured()) {
            throw AccountException::metadataUnavailable();
        }

        $this->gameUpcLookup->choose($payload->barcode(), $payload->externalId, $user->getId()->toRfc4122());

        return new JsonResponse(null, Response::HTTP_NO_CONTENT);
    }
}
