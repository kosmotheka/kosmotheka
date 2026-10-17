---
title: Publishing a release
description: What to do before each release of the Kosmotheka Android app, and how the package that is uploaded to Google Play is built and signed.
sidebar:
  order: 14
---

This page is for whoever publishes a release of the app: the number, the what's-new notes, the licences list and the signed Android package. You do not need it to contribute code.

## Before each release

1. **The version number, in four places that have to match:**
   - `version` in `Cargo.toml` (the `[workspace.package]` section, at the root);
   - `version` in `app/package.json`;
   - `version` in `app/src-tauri/tauri.conf.json`;
   - the key for that version in `app/src/whats-new.json` (see point 2).

   The lock files (`Cargo.lock`, `app/package-lock.json`) also carry the version: they are updated when you build or with `npm install`. `npm run check:whats-new` compares the versions in `Cargo.toml`, `package.json` and `tauri.conf.json`.

   **Every release uploaded to Google Play has to have a higher number:** Google rejects a repeated `versionCode`. Tauri calculates it from `version` (`0.1.0` → `1000`).
2. **The what's-new notes.** The lines about what changes for people live in `app/src/whats-new.json`, under the key `next` while the version has no number. When you choose it, rename `next` to the number (`"0.2.1"`, for example). Then `cd app && npm run check:whats-new` has to pass: it checks that the app's version has its entry, not that the entry is complete; read it.
3. **The licences list, with the exact versions.**
   ```bash
   cd app
   npm run licenses              # needs Docker and the network
   npm run check:licenses:exact
   ```
   In continuous integration the check only looks at package names, so that automatic dependency updates pass on their own. Before publishing, the «Licences» screen has to carry the versions that are published. If `app/src/licenses.json` changes, it goes in the release commit.
4. **The usual checks:** `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and in `app/`: `npm run typecheck`, `npm run lint` and `npm run check:spec` (see [Setting up the environment: the app](../setup-app/)).

## The upload key

Google Play uses **Play App Signing**: the key that signs the app installed on phones is generated and kept by Google. Whoever publishes only has an **upload key**, which shows Google that the package is theirs. If it is lost or leaked, Google allows changing it (in Play Console, *App integrity*), but it has to be kept safe.

What is uploaded is an **Android App Bundle** (`.aab`), not an APK; Google generates the APK for each phone from it.

**The key lives outside the repository.** By default, in `~/.kosmotheka-release/`:

```
~/.kosmotheka-release/
  upload.jks               the upload key
  keystore.properties      where it is, and its passwords
```

`app/src-tauri/gen/android/app/build.gradle.kts` reads `keystore.properties` from that folder, or from the path given in the `KOSMOTHEKA_KEYSTORE_PROPERTIES` environment variable. If the file does not exist, the release build comes out **unsigned**. The repository's `.gitignore` ignores `*.jks`, `*.keystore` and `keystore.properties` anywhere, just in case. **Never** commit them.

### Creating it (once)

In your own terminal, and with a password you write yourself:

```bash
mkdir -p ~/.kosmotheka-release && chmod 700 ~/.kosmotheka-release
keytool -genkeypair -v \
  -keystore ~/.kosmotheka-release/upload.jks \
  -keyalg RSA -keysize 2048 -validity 10000 -alias upload
```

It asks for a password (twice) and some details (name, organisation, country). If at the end it asks for another password for the key, press Enter to use the same one. Then create `~/.kosmotheka-release/keystore.properties`:

```properties
storeFile=/full/path/to/upload.jks
storePassword=<the password>
keyAlias=upload
keyPassword=<the password>
```

and protect it: `chmod 600 ~/.kosmotheka-release/*`. Keep the password in a password manager and a copy of `upload.jks` off the computer.

## Building the `.aab`

```bash
cd app
npm run tauri android build -- --aab
```

It builds for the four architectures and takes a while. The result is in:

```
app/src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab
```

Check that it is signed with the upload key (it asks for no password):

```bash
keytool -printcert -jarfile app/src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab
```

It must show the certificate with the details you gave when creating the key. If there is no certificate, the package came out unsigned: `keystore.properties` is missing or wrong.

To build with the Android tools, follow [Setting up the environment: the app](../setup-app/).

## Uploading it

You upload it yourself, by hand, from the Google Play console: under *Testing* you choose the track, *Create release*, upload the `.aab`, write the release notes and publish. Google Play's notes (500 characters per language at most) come from the lines in `app/src/whats-new.json`.

A new personal Google Play account needs, before production, a **closed** test with at least 12 people for 14 days.

## Something that surprises people

The Google Play version and a debug APK have the same application identifier but a different signature: Android does not let you install one over the other. To go from the debug APK to the Play one you have to uninstall the first, and **that deletes its data**. Before that, make a backup (or have the libraries on another paired device) and restore it or pair again afterwards.
