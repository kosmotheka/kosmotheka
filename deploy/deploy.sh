#!/usr/bin/env bash
# Updates a Kosmotheka server from its git clone: the API server, and optionally the website and the documentation
# site (D90). It runs on the server itself and only pulls from the public repository: nothing is pushed to the
# machine. The steps are those of the self-hosting guide, «Actualizar» (site/src/content/docs/*/run-your-server/updating.md).
#
# Usage, from anywhere on the server:
#   deploy/deploy.sh            # the latest tag (vX.Y.Z)
#   deploy/deploy.sh main       # the tip of main
#   deploy/deploy.sh v0.3.0     # a given tag
#
# Settings come from the environment, or from deploy/deploy.env next to this script (not versioned):
#   KOSMOTHEKA_DIR   the clone (default: the repository this script is in)
#   APP_USER         the user that owns the clone and runs PHP, when the script is run as root (default: the current user)
#   PHP_BIN          default php8.5
#   COMPOSER_BIN     default /usr/bin/composer
#   BACKUP_UNIT      systemd unit run before migrating (default kosmotheka-backup.service; empty to skip)
#   RESTART_UNITS    restarted at the end (default "php8.5-fpm kosmotheka-worker")
#   BUILD_DOCS       1 to build the documentation site into site/dist (needs Node and npm; default 0)
#   NODE_DIR         the folder with the node and npm to build it with, e.g. an nvm version's bin/ (default: the PATH)
#   HEALTH_URL       checked at the end, e.g. https://api.example.org/v1/server (empty to skip)
#
# The website needs no step: nginx serves web/ from the clone, so the checkout updates it.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=/dev/null
[[ -f "$here/deploy.env" ]] && source "$here/deploy.env"

repo="${KOSMOTHEKA_DIR:-$(cd "$here/.." && pwd)}"
php="${PHP_BIN:-php8.5}"
composer="${COMPOSER_BIN:-/usr/bin/composer}"
backup_unit="${BACKUP_UNIT-kosmotheka-backup.service}"
restart_units="${RESTART_UNITS:-php8.5-fpm kosmotheka-worker}"
build_docs="${BUILD_DOCS:-0}"
node_dir="${NODE_DIR:-}"
health_url="${HEALTH_URL:-}"
app_user="${APP_USER:-}"

step() { printf '\n== %s\n' "$*"; }
# Everything that writes in the clone runs as its owner, so a run as root leaves no root-owned files behind.
as_app() { if [[ -n "$app_user" ]]; then sudo -u "$app_user" -H "$@"; else "$@"; fi; }

cd "$repo"

# Local changes would be overwritten or mixed into the release: stop instead. .env.local, var/ and site/dist are
# ignored by git, so they do not count.
if ! as_app git diff --quiet || ! as_app git diff --cached --quiet; then
  echo "The clone at $repo has local changes; deploy refuses to mix them in. See: git status" >&2
  exit 1
fi

step "Fetching"
as_app git fetch --tags --prune --quiet origin
target="${1:-$(as_app git tag --list 'v*' --sort=-v:refname | head -n 1)}"
[[ -n "$target" ]] || { echo "No vX.Y.Z tag yet: pass a ref, e.g. deploy/deploy.sh main" >&2; exit 1; }
[[ "$target" == "main" ]] && target="origin/main"
previous="$(as_app git rev-parse --short HEAD)"
next="$(as_app git rev-parse --short "$target^{commit}")"
echo "From $previous to $next ($target)"

if [[ -n "$backup_unit" ]]; then
  step "Backup ($backup_unit)"
  sudo systemctl start "$backup_unit"
fi

step "Code"
as_app git checkout --quiet --detach "$target"

step "Server: dependencies, migrations, cache"
cd "$repo/server"
as_app "$php" "$composer" install --no-dev --optimize-autoloader --no-interaction --quiet
as_app "$php" bin/console doctrine:migrations:migrate --no-interaction --allow-no-migration
as_app "$php" bin/console cache:clear --no-warmup
as_app "$php" bin/console cache:warmup

if [[ "$build_docs" == "1" ]]; then
  step "Documentation site"
  cd "$repo/site"
  # sudo resets PATH, so a Node from nvm is passed in explicitly.
  node_path="${node_dir:+$node_dir:}$PATH"
  as_app env PATH="$node_path" node --version
  as_app env PATH="$node_path" npm ci --no-audit --no-fund --silent
  # Built aside and swapped in one move, so nginx never serves a half-written site.
  as_app rm -rf dist-next dist-previous
  as_app env PATH="$node_path" ASTRO_TELEMETRY_DISABLED=1 npx astro build --outDir dist-next
  [[ -d dist ]] && as_app mv dist dist-previous
  as_app mv dist-next dist
  as_app rm -rf dist-previous
fi

step "Restarting: $restart_units"
# shellcheck disable=SC2086
sudo systemctl restart $restart_units

if [[ -n "$health_url" ]]; then
  step "Checking $health_url"
  curl --fail --silent --show-error --max-time 10 "$health_url" > /dev/null && echo "It answers."
fi

step "Done: $previous → $next. To go back: deploy/deploy.sh $previous (if no migration ran; otherwise restore the backup too)"
