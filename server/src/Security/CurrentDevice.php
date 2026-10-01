<?php

declare(strict_types=1);

namespace App\Security;

use App\Entity\Device;
use Symfony\Component\HttpFoundation\RequestStack;

/**
 * Gives controllers the Device that App\Security\DeviceTokenAuthenticator
 * authenticated the current request with. The authenticator stores it as a
 * request attribute rather than a stateful singleton service, which would be
 * unsafe to reuse across the requests a single PHP process may serve.
 */
final readonly class CurrentDevice
{
    public const string ATTRIBUTE = 'device';

    public function __construct(
        private RequestStack $requestStack,
    ) {
    }

    public function get(): Device
    {
        $device = $this->requestStack->getCurrentRequest()?->attributes->get(self::ATTRIBUTE);
        if (!$device instanceof Device) {
            throw new \LogicException('CurrentDevice::get() called outside a device-authenticated request.');
        }

        return $device;
    }
}
