// Prints the English lines of a version in src/whats-new.json as a Markdown list: the body of its GitHub release.
// Usage: node scripts/release-notes.mjs [version]  (default: the app's version)
import { readFileSync } from "node:fs";

const read = (path) => readFileSync(new URL(path, import.meta.url), "utf8");
const version = process.argv[2] ?? JSON.parse(read("../package.json")).version;
const lines = JSON.parse(read("../src/whats-new.json"))[version];

if (!Array.isArray(lines) || lines.length === 0) {
  console.error(`release-notes: no lines for version ${version} in src/whats-new.json`);
  process.exit(1);
}
console.log(lines.map((line) => `- ${line.en}`).join("\n"));
