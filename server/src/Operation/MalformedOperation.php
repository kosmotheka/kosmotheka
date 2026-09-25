<?php

declare(strict_types=1);

namespace App\Operation;

/**
 * The payload does not even have the shape of an operation (missing keys,
 * unknown entity or action, wrong JSON types). The Rust client cannot produce
 * these: its deserializer rejects them before validation.
 */
final class MalformedOperation extends \InvalidArgumentException
{
}
