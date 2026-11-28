# The session image of this repository (#106): riff-base with what
# `just ci` needs, so a worker can format, lint and test in its
# container. riff-base has claude, git, gh, curl and CA certificates.
# The image holds no riff and no secret: riff binds its own binary into
# each session.
#
# The workflow "Session image" (.github/workflows/session-image.yml)
# builds it on each change of this folder, and writes the riff.toml line
# with its digest in the summary of the run.
#
# The FROM is the riff-base that riff's deploy/rust/Dockerfile names.
# Not riff-rust: its Rust is older than Bevy 0.20.1 needs (1.97.1).
FROM ghcr.io/como-technologies/riff-base@sha256:2799cd2e7c6e23ab4b9b576ad1331e7315044378a490317624c93d3d984ad5b1

# CI uses stable, so a newer stable can bring a lint that this Rust does
# not report: raise RUST_VERSION to the new stable. 1.100.1 is the
# stable of 2026-11-27.
ARG RUST_VERSION=1.100.1
ARG JUST_VERSION=1.59.0
ARG JUST_SHA256=409044617b6f06517c1035e55ca38c8c856ff8b650e41bc2aa1ef82d5903a23e

# gcc, libc6-dev, make and pkg-config: the linker of rustc, and the build
# of a crate with C code. libasound2-dev, libudev-dev and libwayland-dev:
# the system libraries of Bevy, the same as the apt line of CI.
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      gcc libc6-dev make pkg-config \
      libasound2-dev libudev-dev libwayland-dev \
 && rm -rf /var/lib/apt/lists/*

# The toolchain is in the image, with no write for the session: the
# session gives cargo its own CARGO_HOME. wasm32-unknown-unknown is the
# target of the web check of CI.
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl -fsSL https://sh.rustup.rs \
    | sh -s -- -y --no-modify-path --profile minimal \
        --default-toolchain "$RUST_VERSION" \
        --component rustfmt --component clippy \
        --target wasm32-unknown-unknown \
 && chmod -R a+rX "$RUSTUP_HOME" "$CARGO_HOME"

# just, from its release, checked against the SHA256SUMS of the release.
RUN curl -fsSLo /tmp/just.tar.gz \
      "https://github.com/casey/just/releases/download/${JUST_VERSION}/just-${JUST_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
 && echo "${JUST_SHA256}  /tmp/just.tar.gz" | sha256sum -c - \
 && tar -xzf /tmp/just.tar.gz -C /tmp just \
 && install -m 0755 /tmp/just /usr/local/bin/just \
 && rm /tmp/just.tar.gz /tmp/just

CMD ["bash"]
