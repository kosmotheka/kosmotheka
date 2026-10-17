# Production images for the Kosmotheka server (D48). Unlike ../php and ../nginx, which are
# for development only (they run as root-capable users and carry Xdebug), these:
#   - have no Xdebug, no sudo, no dev dependencies, and run as an unprivileged user;
#   - carry the code inside the image (nothing is mounted).
#
# The build context is the repository root, because the server reads ../spec at runtime:
#
#     docker build -f server/docker/prod/Dockerfile --target php -t kosmotheka-php .
#
# Targets: `php` (PHP-FPM; also runs the console and the worker) and `nginx`.

ARG PHP_VERSION=8.5
FROM php:${PHP_VERSION}-fpm-trixie AS base

RUN apt-get update && apt-get install -y --no-install-recommends \
        libicu-dev libpq-dev libzip-dev unzip \
 && rm -rf /var/lib/apt/lists/* \
 && docker-php-ext-install -j"$(nproc)" intl pdo_pgsql zip

# sodium, mbstring, ctype, iconv, xml and curl are already in the official image.
RUN cp "$PHP_INI_DIR/php.ini-production" "$PHP_INI_DIR/php.ini"
COPY server/docker/prod/php.ini /usr/local/etc/php/conf.d/zz-kosmotheka.ini

# ---- dependencies (this stage never reaches the final image) ----
FROM base AS vendor
COPY --from=composer:2 /usr/bin/composer /usr/local/bin/composer
WORKDIR /app/server
COPY server/composer.json server/composer.lock server/symfony.lock ./
RUN composer install --no-dev --no-scripts --no-autoloader --no-interaction --prefer-dist
COPY server/ ./
COPY spec/ /app/spec/
RUN composer dump-autoload --no-dev --classmap-authoritative --no-scripts \
 && APP_ENV=prod APP_SECRET=build-time-only php bin/console cache:warmup \
 && rm -rf var/log/* .env.local .env.local.php

# ---- PHP-FPM, console and worker ----
FROM base AS php
WORKDIR /app/server
COPY --from=vendor /app/server /app/server
COPY --from=vendor /app/spec /app/spec
# The cache was warmed with a throw-away secret: compiled containers do not hold it, but
# the directory must belong to the runtime user.
RUN chown -R www-data:www-data var
ENV APP_ENV=prod
USER www-data
EXPOSE 9000
CMD ["php-fpm", "-F"]

# ---- nginx: only the public folder and the site ----
FROM nginxinc/nginx-unprivileged:1.28-alpine AS nginx
COPY --from=vendor /app/server/public /app/server/public
COPY server/docker/prod/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 8080
