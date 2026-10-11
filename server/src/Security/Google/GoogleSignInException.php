<?php

declare(strict_types=1);

namespace App\Security\Google;

/** Google refused, could not be reached or said something that does not hold; the message never carries what it sent. */
final class GoogleSignInException extends \RuntimeException
{
}
