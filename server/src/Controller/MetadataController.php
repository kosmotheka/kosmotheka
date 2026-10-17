<?php

declare(strict_types=1);

namespace App\Controller;

use App\Dto\BggSearchRequest;
use App\Dto\ComicVineSearchRequest;
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
use App\Metadata\Isbn;
use App\Metadata\RawgSearch;
use App\Metadata\RawgSource;
use App\Metadata\TmdbSearch;
use App\Metadata\TmdbSource;
use App\Security\AccountException;
use App\Security\RequestLimiter;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpKernel\Attribute\MapRequestPayload;
use Symfony\Component\Routing\Attribute\Route;
use Symfony\Component\Routing\Requirement\Requirement;
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
        private readonly RequestLimiter $limiter,
    ) {
    }

    // `.+`: whatever hangs from this address reaches here and is refused as an ISBN. An address the router does not
    // know is written whole to the application log, and this one holds what an account looks up.
    #[Route('/google-books/isbn/{isbn}', requirements: ['isbn' => Requirement::CATCH_ALL], methods: ['GET'])]
    public function googleBooksIsbn(string $isbn): JsonResponse
    {
        if (!$this->googleBooks->isConfigured()) {
            throw AccountException::metadataUnavailable();
        }

        try {
            $code = Isbn::parse($isbn);
        } catch (\InvalidArgumentException) {
            throw AccountException::invalidIsbn();
        }

        $user = $this->getUser();
        \assert($user instanceof User);
        $this->limiter->consumeMetadata($user->getId()->toRfc4122());

        $book = $this->lookup->lookup($this->googleBooks, $code, $user->getId()->toRfc4122());

        $response = new JsonResponse($book instanceof BookMetadata ? $book->toArray() : BookMetadata::notFoundPayload());
        $response->headers->set('Cache-Control', 'no-store');

        return $response;
    }

    /** A POST on purpose: the title a person looks for is never in an address (an address is written to logs). */
    #[Route('/tmdb/search', methods: ['POST'])]
    public function tmdbSearch(#[MapRequestPayload(serializationContext: [AbstractNormalizer::ALLOW_EXTRA_ATTRIBUTES => false])] TmdbSearchRequest $payload): JsonResponse
    {
        if (!$this->tmdbSource->isConfigured()) {
            throw AccountException::metadataUnavailable();
        }

        $user = $this->getUser();
        \assert($user instanceof User);
        $this->limiter->consumeMetadata($user->getId()->toRfc4122());

        $candidates = $this->tmdbSearch->search($payload->mediaKind(), $payload->askedLanguage(), TmdbSearch::normalise($payload->query), $user->getId()->toRfc4122());

        $response = new JsonResponse(['candidates' => $candidates]);
        $response->headers->set('Cache-Control', 'no-store');

        return $response;
    }

    /** A POST on purpose, as TMDB's: the title a person looks for is never in an address. */
    #[Route('/rawg/search', methods: ['POST'])]
    public function rawgSearch(#[MapRequestPayload(serializationContext: [AbstractNormalizer::ALLOW_EXTRA_ATTRIBUTES => false])] RawgSearchRequest $payload): JsonResponse
    {
        if (!$this->rawgSource->isConfigured()) {
            throw AccountException::metadataUnavailable();
        }

        $user = $this->getUser();
        \assert($user instanceof User);
        $this->limiter->consumeMetadata($user->getId()->toRfc4122());

        $response = new JsonResponse(['candidates' => $this->rawgSearch->search($payload->query, $user->getId()->toRfc4122())]);
        $response->headers->set('Cache-Control', 'no-store');

        return $response;
    }

    /** A POST on purpose, as the others': the title a person looks for is never in an address. */
    #[Route('/comic-vine/search', methods: ['POST'])]
    public function comicVineSearch(#[MapRequestPayload(serializationContext: [AbstractNormalizer::ALLOW_EXTRA_ATTRIBUTES => false])] ComicVineSearchRequest $payload): JsonResponse
    {
        if (!$this->comicVineSource->isConfigured()) {
            throw AccountException::metadataUnavailable();
        }

        $user = $this->getUser();
        \assert($user instanceof User);
        $this->limiter->consumeMetadata($user->getId()->toRfc4122());

        $response = new JsonResponse(['candidates' => $this->comicVineSearch->search($payload->query, $user->getId()->toRfc4122())]);
        $response->headers->set('Cache-Control', 'no-store');

        return $response;
    }

    /** A POST on purpose, as the others': the title a person looks for is never in an address. */
    #[Route('/bgg/search', methods: ['POST'])]
    public function bggSearch(#[MapRequestPayload(serializationContext: [AbstractNormalizer::ALLOW_EXTRA_ATTRIBUTES => false])] BggSearchRequest $payload): JsonResponse
    {
        if (!$this->bggSource->isConfigured()) {
            throw AccountException::metadataUnavailable();
        }

        $user = $this->getUser();
        \assert($user instanceof User);
        $this->limiter->consumeMetadata($user->getId()->toRfc4122());

        $response = new JsonResponse(['candidates' => $this->bggSearch->search($payload->searchScope(), $payload->query, $user->getId()->toRfc4122())]);
        $response->headers->set('Cache-Control', 'no-store');

        return $response;
    }
}
