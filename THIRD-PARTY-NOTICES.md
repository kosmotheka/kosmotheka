# Third-party notices

Kosmotheka is licensed under the AGPL-3.0-or-later (see `LICENSE`). It ships the following work by others, whose licences ask for this notice.

The libraries that go inside the app's installers (Rust crates and npm packages) are listed, with their licence texts, in `app/src/licenses.json`, which `npm run licenses` generates from `Cargo.lock` and the UI build. The app (About → Licences) shows a summary of it and saves the full list as a text file, made from the copy it carries, so the notice travels with the app. This file keeps what those lists do not cover: fonts, icons, SQLite and the Android libraries. The server's dependencies are in `server/composer.lock`.

## Figtree (font)

Used by the app (through `@fontsource-variable/figtree`) and by the website (`web/assets/fonts/`).

Copyright 2022 The Figtree Project Authors (https://github.com/erikdkennedy/figtree)

Licensed under the SIL Open Font License, Version 1.1. The full text is in [`web/assets/fonts/OFL.txt`](web/assets/fonts/OFL.txt) and at https://openfontlicense.org.

## Lucide (icons)

Used by the app through `lucide-react`.

ISC License

Copyright (c) 2026 Lucide Icons and Contributors

Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

Some Lucide icons derive from Feather:

The MIT License (MIT)

Copyright (c) 2013-present Cole Bemis

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## SQLite

The app stores its data with SQLite, compiled into the program through the `libsqlite3-sys` crate. SQLite is in the public domain (https://www.sqlite.org/copyright.html).

## Android libraries

The Android app also contains these libraries (the versions the Gradle files of the app and of the Tauri plugins ask for):

- AndroidX (`core-ktx`, `appcompat`, `activity-ktx`, `lifecycle-process`, `webkit`, `browser`, `biometric`, `camera-*`), Material Components for Android and Jackson (`jackson-databind`, `jackson-datatype-json-org`). All are under the Apache License, Version 2.0, whose text is in the list the app saves (About → Licences) and at https://www.apache.org/licenses/LICENSE-2.0. Copyright The Android Open Source Project, Google LLC and FasterXML, LLC, respectively.
- **ML Kit barcode scanning and Google Play services** (`com.google.android.gms:play-services-mlkit-barcode-scanning`, `play-services-base`, `-basement` and `-tasks`). They are not open source: they are services of Google with terms of their own, and the app needs Google Play services on the phone to scan barcodes. They are used only by the barcode scanner. Terms: [ML Kit Terms of Service](https://developers.google.com/ml-kit/terms), [Google APIs Terms of Service](https://developers.google.com/terms) and [Android Software Development Kit License Agreement](https://developer.android.com/studio/terms).

The list lives, written by hand, in `app/licenses/extra.json`.
