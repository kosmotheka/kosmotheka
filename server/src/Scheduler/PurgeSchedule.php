<?php

declare(strict_types=1);

namespace App\Scheduler;

use Symfony\Component\Scheduler\Attribute\AsSchedule;
use Symfony\Component\Scheduler\RecurringMessage;
use Symfony\Component\Scheduler\Schedule;
use Symfony\Component\Scheduler\ScheduleProviderInterface;

/**
 * D49's 24h purge, run every hour, and the cloud's pass over the trash (D105), every day. Only ticks while a
 * worker consumes the `scheduler_default` transport that this attribute creates automatically
 * (docs/development/server.md); nothing runs it by itself.
 */
#[AsSchedule]
final class PurgeSchedule implements ScheduleProviderInterface
{
    public function getSchedule(): Schedule
    {
        return new Schedule()->add(
            RecurringMessage::every('1 hour', new PurgeAccountsMessage()),
            RecurringMessage::every('1 day', new ExpireTrashMessage()),
        );
    }
}
