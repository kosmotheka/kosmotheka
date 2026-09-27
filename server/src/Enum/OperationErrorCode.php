<?php

declare(strict_types=1);

namespace App\Enum;

/**
 * The error codes of spec/README.md. They are the violation codes of
 * App\Validator\ValidOperation, so clients get the same code from the server as
 * from their own validation.
 */
enum OperationErrorCode: string
{
    case InvalidId = 'invalid_id';
    case InvalidLibrary = 'invalid_library';
    case InvalidIntent = 'invalid_intent';
    case NoChanges = 'no_changes';
    case OtherLibrary = 'other_library';
    case OtherDevice = 'other_device';
    case UnknownField = 'unknown_field';
    case MissingField = 'missing_field';
    case NullRequired = 'null_required';
    case WrongType = 'wrong_type';
    case EmptyUpdate = 'empty_update';
    case FieldsOnDelete = 'fields_on_delete';
    case WrongId = 'wrong_id';
}
