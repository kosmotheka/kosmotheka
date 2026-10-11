<?php

declare(strict_types=1);

namespace App\Controller;

use App\Entity\User;
use App\Metadata\BookLookup;
use App\Metadata\BookMetadata;
use App\Metadata\BookProvider;
use App\Metadata\Isbn;
use App\Security\AccountException;
use App\Security\RequestLimiter;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\Routing\Attribute\Route;

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
}
