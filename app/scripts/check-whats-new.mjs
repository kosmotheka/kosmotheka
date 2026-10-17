// Checks the news file (src/whats-new.json): the app's version must have its own entry, every entry has text in every language the app has a file for,
// and from one to five lines, and the three places that carry the version agree. It ensures there is an
// entry, not that it is complete. The lines for the version that has no number yet live under "next".
import { readFileSync, readdirSync } from "node:fs";

const read = (path) => readFileSync(new URL(path, import.meta.url), "utf8");
const problems = [];

// The languages every line must have text in: those the app has a file for in src/i18n/locales/.
const languages = readdirSync(new URL("../src/i18n/locales/", import.meta.url))
  .filter((name) => name.endsWith(".json"))
  .map((name) => name.replace(/\.json$/, ""));

const file = JSON.parse(read("../src/whats-new.json"));
const appVersion = JSON.parse(read("../package.json")).version;
const tauriVersion = JSON.parse(read("../src-tauri/tauri.conf.json")).version;
const workspace = read("../../Cargo.toml").match(
  /\[workspace\.package\][^[]*?^version\s*=\s*"([^"]+)"/ms,
)?.[1];

if (appVersion !== tauriVersion || appVersion !== workspace) {
  problems.push(
    `versions differ: package.json ${appVersion}, tauri.conf.json ${tauriVersion}, Cargo.toml ${workspace}`,
  );
}
if (!Array.isArray(file[appVersion])) {
  problems.push(
    `no entry for version ${appVersion}: move the lines under "next" to "${appVersion}" in src/whats-new.json`,
  );
}

// Keep in step with `Switch::parse` in src-tauri/src/whats_new.rs, which also checks the real keys exist.
const switchKey = /^(source:[a-z_]+|google_books_via_server)$/;
for (const [version, lines] of Object.entries(file)) {
  if (version !== "next" && !/^\d+\.\d+\.\d+$/.test(version)) {
    problems.push(`"${version}" is not a version`);
  }
  if (!Array.isArray(lines) || lines.length < 1 || lines.length > 5) {
    problems.push(`${version}: from one to five lines are needed`);
    continue;
  }
  lines.forEach((line, index) => {
    for (const language of languages) {
      if (typeof line[language] !== "string" || line[language].trim() === "") {
        problems.push(`${version}, line ${index + 1}: missing "${language}"`);
      }
    }
    if (line.switch !== undefined && !switchKey.test(line.switch)) {
      problems.push(`${version}, line ${index + 1}: unknown switch "${line.switch}"`);
    }
  });
}

if (problems.length > 0) {
  for (const problem of problems) console.error(problem);
  process.exit(1);
}
console.log(
  `whats-new: version ${appVersion} has its entry; ${Object.keys(file).length} entries checked`,
);
