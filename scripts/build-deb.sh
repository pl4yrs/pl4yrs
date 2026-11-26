#!/usr/bin/env bash
# Build pl4yrs .deb packages into dist/.
#
# The .debs target Debian trixie arm64. On any host that is not linux/arm64 this
# script re-executes itself inside the pl4yrs-deb-builder container image
# (packaging/Dockerfile), built on demand.
#
# Usage:
#   scripts/build-deb.sh [--debrev N] [PACKAGE ...]
#
# Packages default to "pl4yrs-backend pl4yrs-tui". The Debian revision defaults
# to 1+<utc commit date>.g<short git sha>.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILDER_IMAGE=pl4yrs-deb-builder
PACKAGES=(pl4yrs-backend pl4yrs-tui)

usage() {
    echo -ne "Usage: \n"
    echo -ne "\t--help\t\t\tPrint help.\n"
    echo -ne "\t--debrev N\t\tDebian revision (default: 1+<utc commit date>.g<short git sha>).\n"
    echo -ne "\tPACKAGE ...\t\tWorkspace packages to build (default: ${PACKAGES[*]}).\n\n"
}

DEBREV=""
POSITIONAL=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        --debrev) DEBREV="$2"; shift 2 ;;
        --help) usage; exit 0 ;;
        -*) echo "build-deb: unknown argument '$1'" >&2; usage; exit 1 ;;
        *) POSITIONAL+=("$1"); shift ;;
    esac
done
[ "${#POSITIONAL[@]}" -gt 0 ] && PACKAGES=("${POSITIONAL[@]}")

if [ -z "$DEBREV" ]; then
    CTS="$(git -C "$REPO_ROOT" show -s --format=%ct HEAD)"
    STAMP="$(date -u -r "$CTS" +%Y%m%d%H%M%S 2>/dev/null || date -u -d "@$CTS" +%Y%m%d%H%M%S)"
    DEBREV="1+${STAMP}.g$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
fi

container_runtime() {
    if command -v podman >/dev/null 2>&1 && [ -z "${DOCKER:-}" ]; then
        echo "podman"
    else
        echo "${DOCKER:-docker}"
    fi
}

if [ "$(uname -s)" != "Linux" ] || [ "$(uname -m)" != "aarch64" ]; then
    DOCKER="$(container_runtime)"
    if ! $DOCKER image inspect "$BUILDER_IMAGE" >/dev/null 2>&1; then
        echo "build-deb: building $BUILDER_IMAGE image"
        $DOCKER build --platform linux/arm64 \
            -f "$REPO_ROOT/packaging/Dockerfile" -t "$BUILDER_IMAGE" "$REPO_ROOT"
    fi
    VOL_SUFFIX=""
    if [ "$DOCKER" = "podman" ] && command -v selinuxenabled >/dev/null 2>&1 && selinuxenabled; then
        VOL_SUFFIX=":z"
    fi
    echo "build-deb: re-executing inside $BUILDER_IMAGE (linux/arm64)"
    # Persist crate downloads
    exec $DOCKER run --rm --platform linux/arm64 \
        -v "$REPO_ROOT:/src${VOL_SUFFIX}" -w /src \
        -v pl4yrs-cargo-registry:/usr/local/cargo/registry \
        -e CARGO_TARGET_DIR=/src/target/deb \
        "$BUILDER_IMAGE" scripts/build-deb.sh --debrev "$DEBREV" "${PACKAGES[@]}"
fi

TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"

cd "$REPO_ROOT"
for pkg in "${PACKAGES[@]}"; do
    echo "build-deb: cargo deb -p $pkg (revision $DEBREV)"
    cargo deb --locked -p "$pkg" --deb-revision "$DEBREV"
done

mkdir -p dist
for pkg in "${PACKAGES[@]}"; do
    cp "$TARGET_DIR"/debian/"${pkg}"_*.deb dist/
done

echo "build-deb: wrote dist/:"
ls -l dist/*.deb
