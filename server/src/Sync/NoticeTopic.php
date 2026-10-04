<?php

declare(strict_types=1);

namespace App\Sync;

use App\Entity\CloudLibrary;
use App\Entity\User;

/**
 * The Mercure topics (D75). They name the cloud library's own id and the account's id, never
 * the local library id (D84).
 */
final readonly class NoticeTopic
{
    public static function library(CloudLibrary $library): string
    {
        return 'urn:kosmotheka:cloud-library:'.$library->getId()->toRfc4122();
    }

    /** «The list of your libraries changed». */
    public static function account(User $user): string
    {
        return 'urn:kosmotheka:account:'.$user->getId()->toRfc4122();
    }
}
