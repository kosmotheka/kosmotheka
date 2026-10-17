<?php

declare(strict_types=1);

namespace App\Dto;

use App\Enum\Locale;
use App\Enum\MediaKind;
use App\Metadata\TmdbSearch;
use Symfony\Component\Validator\Constraints as Assert;
use Symfony\Component\Validator\Context\ExecutionContextInterface;

/** Body of `POST /v1/metadata/tmdb/search` (7.8): a POST so that what a person looks for is never in an address. */
final readonly class TmdbSearchRequest
{
    private const int MIN_QUERY = 2;
    private const int MAX_QUERY = 100;

    public function __construct(
        #[Assert\Choice(choices: ['film', 'tv_series'])]
        public string $kind,
        /** Far above what fits, so a huge text is refused before it is looked at; the real bounds are in {@see self::checkQuery()}. */
        #[Assert\Length(max: 400)]
        #[Assert\Regex('/^[^\p{Cc}]*$/u', message: 'The text must not hold control characters.')]
        public string $query,
        /** A code of {@see Locale}; anything else means its fallback. */
        public string $language = 'en',
    ) {
    }

    #[Assert\Callback]
    public function checkQuery(ExecutionContextInterface $context): void
    {
        $length = mb_strlen(TmdbSearch::normalise($this->query));
        if ($length < self::MIN_QUERY || $length > self::MAX_QUERY) {
            $context->buildViolation('The text must be between {{ min }} and {{ max }} characters.')
                ->setParameter('{{ min }}', (string) self::MIN_QUERY)
                ->setParameter('{{ max }}', (string) self::MAX_QUERY)
                ->atPath('query')
                ->addViolation();
        }
    }

    public function mediaKind(): MediaKind
    {
        return MediaKind::from($this->kind);
    }

    public function askedLanguage(): string
    {
        return Locale::fromAny($this->language)->value;
    }
}
