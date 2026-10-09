# Source of truth for the server's commands. Everything runs inside the
# container: the host's PHP version does not matter and there are no paths or
# flags to remember. Run from server/ (the app and crates have their own tools).

# Defaults of `make stress`.
BOOKS ?= 20000
LIMIT ?= 1200

DOCKER_COMPOSE := docker compose -f docker/docker-compose.yaml
PHP            := docker exec php_kosmotheka sh -c 'cd /var/www/kosmotheka.local &&
END            := '
CONSOLE        := $(PHP) bin/console

.DEFAULT_GOAL := help

# ------------------------------------------------------------ environment ---

.PHONY: up
up: ## Starts the containers
	$(DOCKER_COMPOSE) up -d

.PHONY: down
down: ## Stops the containers
	$(DOCKER_COMPOSE) down

.PHONY: build
build: ## Rebuilds the images
	$(DOCKER_COMPOSE) build

.PHONY: ps
ps: ## Container status
	$(DOCKER_COMPOSE) ps

.PHONY: sh
sh: ## Opens a shell in the PHP container
	docker exec -it php_kosmotheka bash

.PHONY: logs
logs: ## Follows the nginx and php logs
	$(DOCKER_COMPOSE) logs -f nginx_kosmotheka php_kosmotheka

.PHONY: install
install: ## Installs the Composer dependencies
	$(PHP) composer install $(END)

# ---------------------------------------------------------------- symfony ---

.PHONY: cc
cc: ## Clears the cache
	$(CONSOLE) cache:clear $(END)

.PHONY: console
console: ## Runs bin/console (usage: make console C="debug:router")
	$(PHP) bin/console $(C) $(END)

.PHONY: migrate
migrate: ## Applies pending migrations
	$(CONSOLE) doctrine:migrations:migrate --no-interaction --allow-no-migration $(END)

.PHONY: migration
migration: ## Generates a migration from the entity diff
	$(CONSOLE) doctrine:migrations:diff --no-interaction $(END)

.PHONY: worker
worker: ## Runs the scheduler worker (hourly account purge, D49); keep it running in dev
	$(CONSOLE) messenger:consume scheduler_default -vv $(END)

# ------------------------------------------------------------------ tests ---

.PHONY: db-test
db-test: ## Migrates the test database (idempotent)
	@# kosmotheka_test and its `test` role are created by docker/postgres/init the
	@# first time the volume starts, so this only has to migrate.
	$(CONSOLE) doctrine:migrations:migrate --env=test --no-interaction --allow-no-migration $(END)

.PHONY: stress
stress: ## Load test (not in make check). make stress BOOKS=5000 LIMIT=600 [NO_TRIGGERS=1] [FILTER=testWhatOnePushAsks]
	$(PHP) STRESS_BOOKS=$(BOOKS) STRESS_NO_TRIGGERS=$(NO_TRIGGERS) timeout -k 10 $(LIMIT) php -d memory_limit=-1 bin/phpunit --group stress $(if $(FILTER),--filter $(FILTER)) $(END)

.PHONY: test
test: ## Runs the suite (usage: make test ARGS="--filter testAppliesAnOperation")
	$(PHP) php bin/phpunit $(ARGS) $(END)

# --------------------------------------------------------------- fixtures ---

.PHONY: fixtures
fixtures: migrate ## Vacía la base de DESARROLLO y la llena con los escenarios (docs/development/fixtures.md)
	$(CONSOLE) doctrine:fixtures:load --no-interaction $(END)

# ---------------------------------------------------------------- quality ---

.PHONY: phpstan
phpstan: ## Static analysis (reads the test container, warmed first)
	$(CONSOLE) cache:warmup --env=test --quiet $(END)
	$(PHP) ./vendor/bin/phpstan analyse --memory-limit=1G $(END)

.PHONY: cs
cs: ## Fixes code style
	$(PHP) ./vendor/bin/php-cs-fixer fix $(END)

.PHONY: cs-check
cs-check: ## Checks code style without touching anything
	$(PHP) ./vendor/bin/php-cs-fixer fix --dry-run --diff $(END)

.PHONY: rector
rector: ## Applies the Rector rules
	$(PHP) ./vendor/bin/rector process $(END)

.PHONY: rector-check
rector-check: ## Checks Rector without touching anything
	$(PHP) ./vendor/bin/rector process --dry-run $(END)

.PHONY: check
check: cs-check rector-check phpstan test ## Style, Rector, PHPStan and tests. Changes nothing.

.PHONY: fix
fix: cs rector ## Applies style and Rector, then verifies
	$(MAKE) phpstan
	$(MAKE) test

# --------------------------------------------------------------- utilities ---

.PHONY: help
help: ## Shows this help
	@grep -hE '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2}'

# Escribe las bases de datos de la app para cada personaje: el servidor lo da
# todo (server/var/fixtures/cast.json) y el ejemplo de Rust lo vuelca en carpetas
# de datos que la app abre tal cual (docs/development/fixtures.md).
FIXTURE_DEVICES := /tmp/kosmotheka-fixtures

.PHONY: app-fixtures
app-fixtures: fixtures ## Carga las fixtures y crea las carpetas de datos de la app de cada personaje (en /tmp/kosmotheka-fixtures)
	rm -rf $(FIXTURE_DEVICES)
	distrobox enter library-dev -- bash -ic 'cd $(CURDIR)/.. && cargo run -p kosmotheka --example fixture_devices -- $(CURDIR)/var/fixtures/cast.json $(FIXTURE_DEVICES)'
