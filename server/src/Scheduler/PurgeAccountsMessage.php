<?php

declare(strict_types=1);

namespace App\Scheduler;

/** Dispatched hourly by App\Scheduler\PurgeSchedule; carries nothing, the purge needs no input. */
final class PurgeAccountsMessage
{
}
