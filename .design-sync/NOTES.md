# Design sync notes

- **The components are not a published package.** They live in `app/src/components/ui` (shadcn/ui). `.design-sync/prepare.sh` (the `buildCmd`) builds a `node_modules` view in `.design-sync/.cache/view/node_modules` where `library-app` is a symlink to `app/`, so the converter synthesizes its entry from `src/components/ui`. Always pass `--node-modules ./.design-sync/.cache/view/node_modules`.
- **The stylesheet is compiled for the sync**, not taken from `app/dist`: `.design-sync/tailwind.css` imports `app/src/index.css` and safelists the utilities a design agent is likely to use (`@source inline(...)`). It is written to `app/node_modules/.cache/design-sync/ds.css` because `cssEntry` must stay inside the package dir. The Geist font files are copied next to it.
- **The stylesheet is post-processed** (`postprocess-css.mjs`, run by `prepare.sh`) because Claude Design flags theme variables whose type it cannot infer. Each one (`--ease-*`, `--animate-*`, `--aspect-*`, `--default-transition-*`, `--tw-*` defaults) gets `/* @kind other */` **on the same line, after the `;`**: Design ignores the annotation on its own line (the first attempt, 2026-09-25). Wrapping utility selectors in `:is()` did not stop Design counting custom properties inside utilities, so it was dropped.
- **Custom properties inside utility rules** (Tailwind's `--tw-*` internals) are reported by Design as noise; the sync cannot filter them. They are kept low by leaving variable-heavy utilities out of the safelist in `tailwind.css` (space-x/y, ring colours, divide, filters, scale): from 333 to 72 on 2026-09-25, of which 53 come from the app's own code.
- **Card layouts:** `Alert` uses `cardMode: column` and `Select`/`SelectGroup` `cardMode: single` (their open lists are portals and escape a grid cell), after `[GRID_OVERFLOW]` warnings on the 2026-09-25 re-sync.
- **Re-sync anchor:** fetch `_ds_sync.json` with `DesignSync get_file` into `.design-sync/.cache/remote-sync.json` before running `resync.mjs --remote`.
- **Paths in config** (`docsDir`, `docsMap`, `guidelinesGlob`) are relative to the package *symlink path* `.design-sync/.cache/view/node_modules/library-app`, hence the `../../../../` prefix to reach `.design-sync/`.
- **Groups** come from `.design-sync/docs/<Name>.md` frontmatter (the 12 main components) and from the stubs in `.design-sync/docs/groups/` via `docsMap` (their sub-components).
- **Render check runs inside the `library-dev` Distrobox container** (Ubuntu 24.04), with Playwright's `chromium-headless-shell` in `~/.cache/ms-playwright`. The host (Ubuntu 20.04) was not tried. Run validate and capture as `distrobox enter library-dev -- bash -c 'cd ~/Proyectos/personales/library && node .ds-sync/package-validate.mjs ./ds-bundle'`.
- **Overlay previews** need `onOpenAutoFocus={(e) => e.preventDefault()}` on `DialogContent`/`AlertDialogContent` (otherwise the first input's text is selected or a focus ring shows), and `SelectContent position="popper"` for an open Select (item-aligned mode covers the trigger and scrolls the list).
- `lucide-react` is merged into `window.Library` (`extraEntries`). Its `Badge` and `Table` icons collide with the components; the components win, and the icons are available as `BadgeIcon` and `TableIcon`.

## Known render warns

- The host (Ubuntu 20.04) render check also works; its capture screenshots sometimes show a fallback font because the shot is taken before Geist finishes loading. A Playwright check confirmed `document.fonts` loads Geist Variable. Not a styling bug.

- None after authoring. The 40 sub-components without their own preview show the floor card (by design; their usage is in the parent's preview and prompt).

## Re-sync risks

- The safelist in `tailwind.css` decides which utilities designs can use; classes outside it and outside the app's own code render unstyled. Extend it when the design agent needs more.
- `prepare.sh` symlinks every entry of `app/node_modules`; after `npm install` in `app/`, re-run it.
- The brief and error catalogue are copied as guidelines from `docs/design/`; when they change, re-sync to update the project.
- Previews use realistic app data (book titles, device names); they do not import app code outside `components/ui`.
