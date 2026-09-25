# Modelled on Riddle's: PHP-FPM with the extensions Symfony and Doctrine need,
# PostgreSQL instead of MySQL, and no image or PDF tooling.
FROM php:8.5-fpm
ARG TIMEZONE
ARG APP_NAME

LABEL maintainer="Sergio Rebollo <6338830+Tersoal@users.noreply.github.com>"

RUN mkdir -p /usr/share/man/man1

RUN apt-get update && apt-get install -y \
    libicu-dev \
    libzip-dev \
    libpq-dev \
    zip \
    unzip \
    git \
 && rm -rf /var/lib/apt/lists/*

RUN docker-php-ext-configure intl --enable-intl \
 && docker-php-ext-install -j"$(nproc)" intl pdo_pgsql zip

# Without a php.ini only compiled defaults apply (128M), too little for
# composer and cache:clear.
RUN printf '[PHP]\nmemory_limit = 1G\n' > /usr/local/etc/php/conf.d/memory-limit.ini

RUN curl -sS https://getcomposer.org/installer | php -- --install-dir=/usr/local/bin --filename=composer \
 && composer --version

RUN ln -snf "/usr/share/zoneinfo/${TIMEZONE}" /etc/localtime && echo "${TIMEZONE}" > /etc/timezone \
 && printf '[PHP]\ndate.timezone = "%s"\n' "${TIMEZONE}" > /usr/local/etc/php/conf.d/tzone.ini

# xdebug on demand, PhpStorm on 9001 (as in Riddle).
RUN pecl install xdebug \
 && docker-php-ext-enable xdebug \
 && { \
      echo "error_reporting = E_ALL"; \
      echo "display_startup_errors = On"; \
      echo "display_errors = On"; \
      echo "xdebug.mode=debug"; \
      echo "xdebug.start_with_request=trigger"; \
      echo "xdebug.discover_client_host=1"; \
      echo "xdebug.idekey=PHPSTORM"; \
      echo "xdebug.client_port=9001"; \
    } >> /usr/local/etc/php/conf.d/docker-php-ext-xdebug.ini

WORKDIR /var/www/${APP_NAME}

# uid 1000, the host's: files written from the container keep the right owner.
RUN apt-get update && apt-get install -y sudo && rm -rf /var/lib/apt/lists/* \
 && useradd -ms /bin/bash -u 1000 sergio \
 && echo "sergio ALL=(root) NOPASSWD:ALL" > /etc/sudoers.d/sergio \
 && chmod 0440 /etc/sudoers.d/sergio

USER sergio
