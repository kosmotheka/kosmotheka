#!/bin/sh
# Prepares the inputs the design-sync converter expects, without touching the
# app: the components live in app/src/components/ui, not in a published
# package. Run from the repository root before package-build / resync.
set -e
CACHE=app/node_modules/.cache/design-sync
NM=.design-sync/.cache/view/node_modules

# 1. Stylesheet: the app's Tailwind entry plus the utilities a design agent
#    may use (see tailwind.css). It must live inside the package dir (app/).
mkdir -p "$CACHE"
[ -x .ds-sync/node_modules/.bin/tailwindcss ] && [ -d .ds-sync/node_modules/postcss ] \
  || (cd .ds-sync && npm i --silent @tailwindcss/cli@4 postcss)
.ds-sync/node_modules/.bin/tailwindcss -i .design-sync/tailwind.css -o "$CACHE/ds.css"
node .design-sync/postprocess-css.mjs "$CACHE/ds.css"
rm -rf "$CACHE/files"
cp -r app/node_modules/@fontsource-variable/geist/files "$CACHE/files"

# 2. A node_modules view where the app itself is the "library-app" package,
#    so the converter resolves it (and its dependencies) by name.
rm -rf "$NM"
mkdir -p "$NM"
for entry in app/node_modules/* app/node_modules/.bin; do
  ln -s "$(realpath "$entry")" "$NM/$(basename "$entry")"
done
ln -s "$(realpath app)" "$NM/library-app"
