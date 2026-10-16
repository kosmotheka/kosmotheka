// Checks that src/licenses.json (the «Licenses» screen) still covers what ships: the crates `cargo tree` says end in
// the binaries for the three targets, and the npm packages the UI build reads. It compares them with the lists the
// generator stored in the file, which it took with the same code (scripts/licenses-lib.mjs). Needs cargo.
// A dependency that is only for development does not matter here; one that ships and changed does: run
// `npm run licenses`.
import { readFileSync } from "node:fs";

import { aboutTargets, bundledPackages, rustCrates, rustTargets } from "./licenses-lib.mjs";

const file = JSON.parse(readFileSync(new URL("../src/licenses.json", import.meta.url), "utf8"));
const problems = [];

function compare(name, now, stored) {
  const had = new Set(stored);
  const has = new Set(now);
  for (const key of has)
    if (!had.has(key)) problems.push(`${name}: ${key} ships and is not in licenses.json`);
  for (const key of had)
    if (!has.has(key)) problems.push(`${name}: ${key} is in licenses.json and no longer ships`);
}

if (aboutTargets().join() !== rustTargets().join()) {
  problems.push(
    `about.toml targets (${aboutTargets()}) differ from licenses-lib.mjs (${rustTargets()})`,
  );
}

compare("rust", rustCrates(), file.compared.rust);
compare("npm", (await bundledPackages()).keys(), file.compared.npm);

// Every package has a text to show, or a place to read its terms.
for (const entry of file.packages) {
  if (entry.source !== "other" && entry.texts.length === 0) {
    problems.push(`${entry.name}@${entry.version} has no licence text`);
  }
  for (const index of entry.texts) {
    if (!file.texts[index]?.text)
      problems.push(`${entry.name}@${entry.version}: text ${index} is missing`);
  }
}

if (problems.length > 0) {
  for (const problem of problems) console.error(problem);
  console.error(
    "run `npm run licenses` (needs Docker and the network) and commit src/licenses.json",
  );
  process.exit(1);
}
console.log(
  `licenses: ${file.compared.rust.length} crates and ${file.compared.npm.length} npm packages are covered by ${file.packages.length} entries`,
);
