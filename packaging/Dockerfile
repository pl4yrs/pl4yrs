# Builder image for the pl4yrs .deb packages: rust toolchain on Debian trixie plus
# cargo-deb. buildpack-deps already ships dpkg, dpkg-dev, binutils and git.
#
#   docker build --platform linux/arm64 -f packaging/Dockerfile -t pl4yrs-deb-builder .
#
# scripts/build-deb.sh builds it automatically when missing.
FROM rust:1-trixie

RUN cargo install cargo-deb --locked --version =3.8.0
