<?php

declare(strict_types=1);

namespace App\Dto;

use App\Metadata\ComicVineSearch;
use Symfony\Component\Validator\Constraints as Assert;
use Symfony\Component\Validator\Context\ExecutionContextInterface;

/** Body of `POST /v1/metadata/comic-vine/search` (7.8): a POST so that what a person looks for is never in an address. */
final readonly class ComicVineSearchRequest
{
    private const int MIN_QUERY = 2;
    private const int MAX_QUERY = 100;

    public function __construct(
        /** Far above what fits, so a huge text is refused before it is looked at; the real bounds are in {@see self::checkQuery()}. */
        #[Assert\Length(max: 400)]
        #[Assert\Regex('/^[^\p{Cc}]*$/u', message: 'The text must not hold control characters.')]
        public string $query,
    ) {
    }

    #[Assert\Callback]
    public function checkQuery(ExecutionContextInterface $context): void
    {
        $length = mb_strlen(ComicVineSearch::normalise($this->query));
        if ($length < self::MIN_QUERY || $length > self::MAX_QUERY) {
            $context->buildViolation('The text must be between {{ min }} and {{ max }} characters.')
                ->setParameter('{{ min }}', (string) self::MIN_QUERY)
                ->setParameter('{{ max }}', (string) self::MAX_QUERY)
                ->atPath('query')
                ->addViolation();
        }
    }
}
