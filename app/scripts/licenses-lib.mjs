// What ships in the installers, as the generator (licenses.mjs) and the check (check-licenses.mjs) both read it.
// Sharing this file is what keeps «what goes in» the same on both sides.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { build } from "vite";

export const appDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
export const repoDir = resolve(appDir, "..");

/** The targets the installers are built for: desktop Linux and Windows, and the Android phones. Also in about.toml. */
const TARGETS = ["x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc", "aarch64-linux-android"];

/** The targets as about.toml lists them, which must be the same ones. */
export function aboutTargets() {
  const toml = readFileSync(join(repoDir, "about.toml"), "utf8");
  const list = toml.match(/^targets = \[([^\]]*)\]/m)?.[1] ?? "";
  return [...list.matchAll(/"([^"]+)"/g)].map((found) => found[1]);
}

export const rustTargets = () => [...TARGETS];

/**
 * The npm packages whose files the UI build reads: every module Vite processes that lives in node_modules, so
 * the CSS and fonts of a package count too. Nothing is written to disk. Keyed by `name@version`.
 */
export async function bundledPackages() {
  const seen = new Map();
  const collect = {
    name: "collect-node-modules",
    transform(_code, id) {
      const path = id.split("?")[0];
      if (!path.includes("/node_modules/") || seen.has(path)) return null;
      seen.set(path, packageDir(path));
      return null;
    },
  };
  await build({
    root: appDir,
    logLevel: "silent",
    plugins: [collect],
    build: { write: false, reportCompressedSize: false },
  });
  const packages = new Map();
  for (const dir of new Set(seen.values())) {
    const manifest = JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
    packages.set(`${manifest.name}@${manifest.version}`, {
      name: manifest.name,
      version: manifest.version,
      license: licenseOf(manifest),
      repository: repositoryOf(manifest),
      dir,
    });
  }
  return packages;
}

function packageDir(path) {
  let dir = dirname(path);
  while (dir.includes("/node_modules/")) {
    const manifest = join(dir, "package.json");
    if (existsSync(manifest) && JSON.parse(readFileSync(manifest, "utf8")).name) return dir;
    dir = dirname(dir);
  }
  throw new Error(`no package.json above ${path}`);
}

function licenseOf(manifest) {
  if (typeof manifest.license === "string") return manifest.license;
  if (manifest.license?.type) return manifest.license.type;
  if (Array.isArray(manifest.licenses)) return manifest.licenses.map((l) => l.type).join(" OR ");
  return null;
}

function repositoryOf(manifest) {
  const repository =
    typeof manifest.repository === "string" ? manifest.repository : manifest.repository?.url;
  if (!repository) return manifest.homepage ?? null;
  return repository
    .replace(/^git\+/, "")
    .replace(/^git:\/\//, "https://")
    .replace(/^github:/, "https://github.com/")
    .replace(/\.git$/, "");
}

/** The licence file of a package, with its NOTICE file when it has one (Apache-2.0 asks for it). */
export function licenseTextOf(dir) {
  const files = readdirSync(dir);
  const pick = (pattern) => files.filter((file) => pattern.test(file)).sort();
  const texts = [...pick(/^(licen[sc]e|copying)(\.|$)/i), ...pick(/^notice(\.|$)/i)].map((file) =>
    readFileSync(join(dir, file), "utf8").trim(),
  );
  return texts.length > 0 ? texts.join("\n\n") : null;
}

/** Runs cargo where the check runs (the CI, or the development container). The generator runs it in Docker instead. */
function localCargo(args) {
  return execFileSync("cargo", args, {
    cwd: repoDir,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
}

/**
 * The registry crates that end in the binaries, from the Cargo lockfile: what `cargo tree` says is a normal
 * dependency of the app on any of the three targets. Workspace crates (ours) are left out. Keyed by `name@version`.
 */
export function rustCrates(cargo = localCargo) {
  const crates = new Set();
  for (const target of TARGETS) {
    const out = cargo([
      "tree",
      "--locked",
      "-p",
      "kosmotheka",
      "--edges",
      "normal",
      "--target",
      target,
      "--prefix",
      "none",
      "--format",
      "{p}",
    ]);
    for (const line of out.split("\n")) {
      const found = line.match(/^(\S+) v(\S+)(?: \(proc-macro\))?(?: \((?!\/)[^)]*\))?/);
      if (found && !/\(\//.test(line)) crates.add(`${found[1]}@${found[2]}`);
    }
  }
  return crates;
}
