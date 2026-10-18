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

   If the first non-zero number changes (from `0.2.x` to `0.3.0`, for example), also change the version the app asks of the domain crate, `kosmotheka-domain = { version = "…" }` in `app/src-tauri/Cargo.toml`: with a leading `0.`, Cargo does not take a `0.3` as compatible with a `0.2`, and the build fails.

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

## With the workflow

`.github/workflows/release.yml` builds and publishes **only the app** (D74): the server, the website and the documentation are deployed by hand with `deploy/deploy.sh`.

1. Get the version ready (the "Before each release" section) and push it to `main`.
2. Create and push the tag, which must be `v` plus the version in `tauri.conf.json` (for example `v0.3.0`); if it does not match, the workflow fails before building.
3. The workflow does, in this order:
   - **Check the version:** the versions of the three files and the entry in `whats-new.json`.
   - **Linux** (Ubuntu 22.04): `.deb`, `.rpm` and AppImage. **Windows:** the installer. The names carry no version: `Kosmotheka-linux-x86_64.AppImage`, `Kosmotheka-linux-amd64.deb`, `Kosmotheka-linux-x86_64.rpm`, `Kosmotheka-windows-x64-setup.exe`.
   - **The draft release,** with the four files, a `SHA256SUMS` and, as its body, the English lines of the version in `whats-new.json`.
   - **Android:** builds the `.aab`, checks that it is signed and uploads it to Google Play as a **draft** release, to the track named by the `PLAY_TRACK` variable of the `release` environment (`internal` if it does not exist; `alpha` is the closed test).
4. **Review and publish the draft** on GitHub (*Releases*). Until then nobody sees it, and the `…/releases/latest/download/…` links still give the previous version.
5. **In Google Play,** open the track, review the draft release, add the notes (they come from `whats-new.json`) and send it for review.

To rehearse without publishing, run the workflow by hand (*Actions › Release › Run workflow*): it builds everything and keeps the files as workflow artifacts (14 days), but creates no release and uploads nothing to Google Play. Without the Android secrets, the `.aab` is built unsigned and the summary says so.

The secrets and the environment are set up once, in the public repository, by whoever maintains it. Everything below, done by hand, still works if the workflow is not available.

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
