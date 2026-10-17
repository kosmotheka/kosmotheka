// Fails when a UI string exists in one language and not in the other: every key of every file in src/i18n/locales/
// must be in all of them (AGENTS.md: no user-facing string without both translations). Plural forms count as keys, so
// a language whose plural rules need other forms will have to be taught here when it arrives.
import { readFileSync, readdirSync } from "node:fs";

const dir = new URL("../src/i18n/locales/", import.meta.url);
const locales = readdirSync(dir).filter((name) => name.endsWith(".json"));

function keys(value, prefix = "") {
  if (value === null || typeof value !== "object" || Array.isArray(value)) return [prefix];
  return Object.entries(value).flatMap(([key, inner]) =>
    keys(inner, prefix ? `${prefix}.${key}` : key),
  );
}

// `genresByKind.<kind>.<key>` is an override a language may need (Spanish agrees the genre with the kind: «Policiaca»
// for a film) and another may not: the app falls back to `genres.<key>` (lib/kinds.ts).
function fallsBack(key, own) {
  const override = /^genresByKind\.[^.]+\.(.+)$/.exec(key);
  return override !== null && own.has(`genres.${override[1]}`);
}

const byLocale = new Map(
  locales.map((name) => [
    name,
    new Set(keys(JSON.parse(readFileSync(new URL(name, dir), "utf8")))),
  ]),
);
const problems = [];
for (const [name, own] of byLocale) {
  for (const [other, theirs] of byLocale) {
    if (other === name) continue;
    for (const key of theirs) {
      if (own.has(key) || fallsBack(key, own)) continue;
      problems.push(`${name}: missing ${key} (it is in ${other})`);
    }
  }
}

if (problems.length > 0) {
  for (const problem of problems) console.error(problem);
  process.exit(1);
}
console.log(`i18n: ${locales.join(", ")} have the same keys`);
