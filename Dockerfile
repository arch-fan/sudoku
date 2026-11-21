# syntax=docker/dockerfile:1.7
#
# Web (PWA) build of the game, served by static-web-server v3.
#
#   docker build -t sudoku-web .
#   docker run --rm -p 8080:8080 sudoku-web        → http://localhost:8080
#
# The wasm output is platform-independent, so the build stage always runs on the
# build machine's native arch; `--platform linux/amd64` only changes the tiny
# final server image (no emulation needed when building on Apple Silicon).

ARG RUST_TOOLCHAIN=nightly-2026-11-21
ARG TRUNK_VERSION=0.22.2
ARG SWS_VERSION=3.0.0-beta.3

# ---------------------------------------------------------------------------
# 1. Build the wasm bundle with Trunk
# ---------------------------------------------------------------------------
FROM --platform=$BUILDPLATFORM rust:1-slim-trixie AS build
ARG RUST_TOOLCHAIN
ARG TRUNK_VERSION

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl git brotli \
    && rm -rf /var/lib/apt/lists/*

# Same pinned nightly as mise.toml (the web build doesn't need Cranelift).
RUN rustup toolchain install "${RUST_TOOLCHAIN}" --profile minimal --target wasm32-unknown-unknown \
    && rustup default "${RUST_TOOLCHAIN}"

RUN curl -fsSL "https://github.com/trunk-rs/trunk/releases/download/v${TRUNK_VERSION}/trunk-$(uname -m)-unknown-linux-gnu.tar.gz" \
    | tar -xz -C /usr/local/bin trunk \
    && trunk --version

WORKDIR /src
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    cd apps/web \
    && trunk build --release --cargo-profile web-release --dist /dist

# Pre-compress text assets; SWS serves the .br/.gz siblings directly.
RUN find /dist -type f \( -name '*.wasm' -o -name '*.js' -o -name '*.html' -o -name '*.webmanifest' \) \
    -exec brotli --best --keep {} \; \
    -exec gzip --best --keep {} \;

# ---------------------------------------------------------------------------
# 2. Serve it (scratch-based image, a few MB)
# ---------------------------------------------------------------------------
FROM joseluisq/static-web-server:${SWS_VERSION}

COPY --from=build /dist /public
COPY sws.toml /etc/sws.toml

ENV SERVER_CONFIG_FILE=/etc/sws.toml
EXPOSE 8080
