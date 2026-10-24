// Writes src/licenses.json: the notices of everything that ships in the installers, for the «Licenses» screen.
//   Rust crates: cargo-about, in Docker (needs Docker and the network: it downloads the crates).
//   npm packages: the modules Vite reads when it builds the UI.
//   Android libraries and SQLite: licenses/extra.json, by hand.
// `npm run check:licenses` then says when a dependency changed and this has to be run again.
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { appDir, bundledPackages, licenseTextOf, repoDir, rustCrates } from "./licenses-lib.mjs";

// The only place the version of cargo-about is written (the image and the output carry it from here).
const CARGO_ABOUT_VERSION = "0.9.2";
const IMAGE = `kosmotheka-cargo-about:${CARGO_ABOUT_VERSION}`;

// Crates `cargo tree` lists that cargo-about leaves out. Each is only reached through a proc-macro (it runs when
// compiling and is not linked into the app). Anything new here makes the generator stop for a decision.
const COMPILE_TIME_ONLY = {
  "serde_spanned@0.6.9": "dependency of toml_edit 0.20, used by glib-macros (proc-macro)",
  "indexmap@1.9.3": "dependency of toml_edit 0.20, used by glib-macros (proc-macro)",
  "hashbrown@0.12.3": "dependency of indexmap 1.9, used by glib-macros (proc-macro)",
};

function docker(args, options = {}) {
  return execFileSync("docker", args, {
    cwd: repoDir,
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024,
    ...options,
  });
}

function ensureImage() {
  try {
    docker(["image", "inspect", IMAGE], { stdio: "ignore" });
  } catch {
    console.error(`building ${IMAGE}…`);
    docker(
      [
        "build",
        "-q",
        "-t",
        IMAGE,
        "--build-arg",
        `CARGO_ABOUT_VERSION=${CARGO_ABOUT_VERSION}`,
        "app/licenses",
      ],
      { stdio: ["ignore", "ignore", "inherit"] },
    );
  }
}

// Cargo, in the same image, so nothing needs installing on the machine (the repository is mounted read-only).
function inImage(command) {
  ensureImage();
  return docker([
    "run",
    "--rm",
    "-v",
    `${repoDir}:/work:ro`,
    "-v",
    "kosmotheka-cargo-registry:/usr/local/cargo/registry",
    "-w",
    "/work",
    IMAGE,
    ...command,
  ]);
}

const aboutOutput = () =>
  JSON.parse(
    inImage([
      "cargo",
      "about",
      "generate",
      "--format",
      "json",
      "--manifest-path",
      "app/src-tauri/Cargo.toml",
      "-c",
      "about.toml",
    ]),
  );

const texts = []; // { id, text }, each distinct text once
function textIndex(id, text) {
  const found = texts.findIndex((entry) => entry.id === id && entry.text === text);
  if (found >= 0) return found;
  texts.push({ id, text });
  return texts.length - 1;
}

const problems = [];
const packages = [];

// Rust
const about = aboutOutput();
const byCrate = new Map(); // name@version -> text indices
for (const license of about.licenses) {
  for (const { crate } of license.used_by) {
    const key = `${crate.name}@${crate.version}`;
    const list = byCrate.get(key) ?? [];
    list.push(textIndex(license.id, license.text.trim()));
    byCrate.set(key, list);
  }
}
const rust = rustCrates((args) => inImage(["cargo", ...args]));
for (const { package: pkg, license } of about.crates) {
  if (pkg.source === null) continue; // our own crates
  const key = `${pkg.name}@${pkg.version}`;
  packages.push({
    name: pkg.name,
    version: pkg.version,
    source: "rust",
    license,
    repository: pkg.repository ?? `https://crates.io/crates/${pkg.name}`,
    texts: [...new Set(byCrate.get(key) ?? [])],
  });
}
const listed = new Set(packages.map((p) => `${p.name}@${p.version}`));
for (const key of rust) {
  if (!listed.has(key) && !COMPILE_TIME_ONLY[key])
    problems.push(`cargo tree has ${key} and cargo-about does not`);
}
for (const entry of packages) {
  if (entry.texts.length === 0) problems.push(`no licence text for ${entry.name}@${entry.version}`);
}

const extra = JSON.parse(readFileSync(join(appDir, "licenses", "extra.json"), "utf8"));

// npm
const npm = await bundledPackages();
for (const entry of npm.values()) {
  const text = licenseTextOf(entry.dir) ?? extra.clarify[entry.name]?.text ?? null;
  if (!entry.license || !text) {
    problems.push(
      `npm ${entry.name}@${entry.version}: ${entry.license ? "no licence file" : "no licence field"}`,
    );
    continue;
  }
  packages.push({
    name: entry.name,
    version: entry.version,
    source: "npm",
    license: entry.license,
    repository: entry.repository,
    texts: [textIndex(entry.license, text)],
  });
}

// By hand
const apache = texts.findIndex((entry) => entry.id === "Apache-2.0");
if (apache < 0) problems.push("no Apache-2.0 text to use for the Android libraries");
for (const entry of extra.notices) {
  packages.push({ ...entry, source: "android", texts: [apache] });
}
for (const { text, ...entry } of extra.bundled) {
  packages.push({ ...entry, source: "other", texts: [textIndex(entry.license, text)] });
}
for (const entry of extra.public_domain) {
  packages.push({ ...entry, source: "other", license: "Public domain", texts: [] });
}
const terms = extra.terms;

if (problems.length > 0) {
  for (const problem of problems) console.error(problem);
  process.exit(1);
}

packages.sort(
  (a, b) =>
    a.source.localeCompare(b.source) ||
    a.name.localeCompare(b.name) ||
    a.version.localeCompare(b.version),
);
const sorted = (set) => [...set].sort();
const lines = (list) => list.map((item) => `    ${JSON.stringify(item)}`).join(",\n");
const out = `{
  "generator": ${JSON.stringify({ cargoAbout: CARGO_ABOUT_VERSION })},
  "compared": {
    "rust": ${JSON.stringify(sorted(rust))},
    "npm": ${JSON.stringify(sorted(npm.keys()))}
  },
  "terms": ${JSON.stringify(terms)},
  "texts": [
${lines(texts)}
  ],
  "packages": [
${lines(packages)}
  ]
}
`;
writeFileSync(join(appDir, "src", "licenses.json"), out);
console.log(
  `licenses: ${packages.length} packages (${rust.size} crates compared, ${npm.size} npm), ${texts.length} texts, ${(out.length / 1024).toFixed(0)} KB`,
);
