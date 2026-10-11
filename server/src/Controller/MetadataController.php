<?php

declare(strict_types=1);

namespace App\Controller;

use App\Dto\TmdbSearchRequest;
use App\Entity\User;
use App\Metadata\BookLookup;
use App\Metadata\BookMetadata;
use App\Metadata\BookProvider;
use App\Metadata\Isbn;
use App\Metadata\TmdbSearch;
use App\Metadata\TmdbSource;
use App\Security\AccountException;
use App\Security\RequestLimiter;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\HttpKernel\Attribute\MapRequestPayload;
use Symfony\Component\Routing\Attribute\Route;
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
        private readonly RequestLimiter $limiter,
    ) {
    }

    #[Route('/google-books/isbn/{isbn}', methods: ['GET'])]
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

        $book = $this->lookup->lookup($this->googleBooks, $code);

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

        $candidates = $this->tmdbSearch->search($payload->mediaKind(), $payload->askedLanguage(), TmdbSearch::normalise($payload->query));

        $response = new JsonResponse(['candidates' => $candidates]);
        $response->headers->set('Cache-Control', 'no-store');

        return $response;
    }
}
