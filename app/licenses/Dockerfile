# cargo-about, to list the licences of the Rust crates that ship in the installers (npm run licenses).
# The version is set in scripts/licenses.mjs, the only place it is written.
FROM rust:1.98.1
ARG CARGO_ABOUT_VERSION
RUN test -n "$CARGO_ABOUT_VERSION" && cargo install cargo-about --features cli --version "$CARGO_ABOUT_VERSION" --locked
