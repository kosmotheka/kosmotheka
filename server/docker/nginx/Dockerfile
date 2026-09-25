FROM debian:trixie-slim
ARG APP_NAME

LABEL maintainer="Sergio Rebollo <6338830+Tersoal@users.noreply.github.com>"

RUN apt-get update && apt-get install -y nginx ca-certificates && rm -rf /var/lib/apt/lists/*

COPY nginx.conf /etc/nginx/
COPY library.local.conf /etc/nginx/sites-available/

RUN ln -s /etc/nginx/sites-available/library.local.conf /etc/nginx/sites-enabled/library.local.conf \
 && rm /etc/nginx/sites-enabled/default

RUN echo "upstream php-upstream { server php_library:9000; }" > /etc/nginx/conf.d/upstream.conf

RUN usermod -u 1000 www-data

CMD ["nginx"]

EXPOSE 80
