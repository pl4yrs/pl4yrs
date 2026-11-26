# Pl4yrs workspace

[![Coverage Status](https://coveralls.io/repos/github/pl4yrs/pl4yrs/badge.svg?branch=test/improve-test-coverage)](https://coveralls.io/github/pl4yrs/pl4yrs?branch=test/improve-test-coverage)

This is the monorepo containing the different components for the Pl4yrs stack:

* `pl4yrs-backend` - The backend API for controlling the device configuration and status.
* `pl4yrsd` - The media playing daemon.
* `pl4yrs-tui` - The TUI application to control pl4yrs.
* `pl4yrs-api` - The public API crate to be shared across backend and clients.

## Debian packages

`pl4yrs-backend` and `pl4yrs-tui` are packaged as `.deb`s for Debian trixie arm64
via [cargo-deb](https://github.com/kornelski/cargo-deb).

```bash
scripts/build-deb.sh               # builds both packages
scripts/build-deb.sh --debrev 1    # override tagged-release revision (default is 1+<utc commit date>.g<sha>)
```

On macOS the script builds the `pl4yrs-deb-builder` image (`packaging/Dockerfile`,
rust:1-trixie + cargo-deb) once and re-executes itself inside it. Output lands in `dist/`.

Install on a device:

```bash
apt install ./pl4yrs-backend_*.deb ./pl4yrs-tui_*.deb
```
