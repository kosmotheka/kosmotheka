#!/usr/bin/env bash
# Ensayo de la reescritura del historial para abrir el repositorio (D90).
# Trabaja SIEMPRE sobre una copia nueva: nunca toca el repositorio original ni sube nada.
# Uso: reescribir.sh <ruta del repositorio original> <carpeta de trabajo>
set -euo pipefail

ORIGEN="$(realpath "$1")"
TRABAJO="$(realpath -m "$2")"
COPIA="$TRABAJO/kosmotheka"
NOREPLY="6338830+Tersoal@users.noreply.github.com"

rm -rf "$COPIA"
mkdir -p "$TRABAJO"
# Clon local y completo (todas las ramas); sin remoto, para que nada pueda subirse por error.
git clone --quiet --no-local "$ORIGEN" "$COPIA"
git -C "$COPIA" remote remove origin

# Lo que sale del historial (clasificacion-docs.md, «Para el día de abrirlo»).
cat > "$TRABAJO/quitar.txt" <<'EOF'
docs/research/tmview/
docs/research/tmview-check.mjs
docs/research/tmview-image-check.mjs
docs/research/play-check.mjs
docs/publicacion/servidor-produccion.md
docs/publicacion/firma-android.md
docs/publicacion/provisional-pages/
server/.env.dev
glob:docs/design/screens/*/support.js
glob:docs/design/screens/*/ds-base.js
EOF

# El autor pasa a la dirección noreply de GitHub; dependabot se queda como está.
cat > "$TRABAJO/mailmap" <<EOF
Sergio Rebollo <$NOREPLY> Sergio Rebollo <6338830+Tersoal@users.noreply.github.com>
EOF

# git-filter-repo con versión fijada, en Docker (nada se instala en la máquina).
docker run --rm -v "$COPIA":/repo -v "$TRABAJO":/trabajo -w /repo python:3.13-slim sh -c '
  apt-get update -qq >/dev/null && apt-get install -y -qq git >/dev/null &&
  pip install -q git-filter-repo==2.47.0 &&
  git config --global --add safe.directory /repo &&
  git filter-repo --force \
    --invert-paths --paths-from-file /trabajo/quitar.txt \
    --path-rename "docs/historico/encargo-d99-dueño-de-la-nube.md:docs/historico/encargo-d99-dueno-de-la-nube.md" \
    --mailmap /trabajo/mailmap \
    --message-callback "
import re
return re.sub(rb\"(?m)^Claude-Session: .*\\n?\", b\"\", message)
"
'

echo "== Comprobaciones"
echo "commits: $(git -C "$COPIA" rev-list --all | wc -l)"
echo "autores:"; git -C "$COPIA" log --all --format='  %an <%ae>' | sort | uniq -c
echo "líneas Claude-Session que quedan: $(git -C "$COPIA" log --all --format=%B | grep -c '^Claude-Session:' || true)"
echo "rastros de lo quitado en el historial:"
git -C "$COPIA" log --all --name-only --format= | grep -E 'tmview|servidor-produccion|firma-android|provisional-pages|\.env\.dev$|/support\.js$|/ds-base\.js$|dueño' | sort -u | sed 's/^/  /' || echo "  ninguno"
git -C "$COPIA" gc --quiet --prune=now --aggressive
echo "tamaño de .git: $(du -sh "$COPIA/.git" | cut -f1)"
