# Documentation site

The public documentation of Kosmotheka, published at docs.kosmotheka.com. It is built with [Starlight](https://starlight.astro.build) (Astro) from the Markdown in `src/content/docs/`, and ships as static HTML: nothing of it runs on the server. It is separate from `docs/`, which is the development diary.

```bash
cd site
npm ci
npm run dev       # http://localhost:4321, with reload
npm run build     # writes dist/ and the search index (Pagefind); this is what the CI runs
```

## Languages and folders

Both languages have a prefix, `/es/` and `/en/`, and Spanish is the default. `/` only redirects to one of them from the browser's language (`src/pages/index.astro`).

Pages live in `src/content/docs/es/` and `src/content/docs/en/`, in three parts: `using-the-app/`, `run-your-server/` and `contribute/`. **The folder and file names are the same in both languages** (English): the language picker links a page to the same path in the other language. Only titles and text are translated.

## Adding a page

1. Write it first in Spanish: `src/content/docs/es/<part>/<name>.md`, with `title` and `description` in the front matter.
2. The sidebar lists the pages of each folder by itself; use `sidebar: { order: N }` in the front matter to place one.
3. Translate it to `src/content/docs/en/<part>/<name>.md`, with the same path. Until then, a visitor in English sees the Spanish page with a notice that it is not translated, and the URL does not change when it is.
4. `npm run build` must pass: it fails on invalid front matter and on a broken page.

## Look and feel

Colours and font come from the website (`web/`) and the brand (`brand/`), mapped onto Starlight's variables in `src/styles/custom.css`. The font, Figtree, is served from `public/fonts/` with its licence (`OFL.txt`). The site loads nothing from third parties: no analytics, no web fonts, no CDN. Keep it that way. Astro's telemetry is switched off in the npm scripts.

Dependencies are pinned to exact versions and `package-lock.json` is in git. Dependabot proposes updates weekly.
