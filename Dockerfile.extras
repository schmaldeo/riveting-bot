# Setup.
# FROM rust:slim AS rust
FROM rustlang/rust:nightly-bookworm-slim AS rust

# Update OS and setup deps.
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    apt update && apt upgrade -y \
    && apt install -y --no-install-recommends \
    pkg-config libopus-dev
# curl clang make cmake


# Compile.
FROM rust AS builder

# Use sparse registry.
ARG CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
ARG BUILD_FEATURES=voice

# Create a new empty shell project.
RUN cargo new --bin app

WORKDIR /app

# Copy manifests.
# COPY ./.cargo ./.cargo
COPY ./Cargo.lock ./Cargo.toml ./

# Build only the dependencies to cache them.
RUN --mount=type=cache,target=~/.cargo,sharing=locked \
    cargo build --release --features=$BUILD_FEATURES

# Remove default code from deps build.
RUN rm ./src/*.rs && rm ./target/release/deps/riveting_bot*

# Copy the source code.
COPY ./src ./src

# Build for release.
RUN --mount=type=cache,target=~/.cargo,sharing=locked \
    cargo build --release --features=$BUILD_FEATURES \
    && strip --strip-all ./target/release/riveting-bot


# Final image.
FROM debian:bookworm-slim as final

# Update OS and setup deps.
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    apt update && apt upgrade -y \
    # && apt install -y ca-certificates && update-ca-certificates \
    && apt install -y --no-install-recommends \
    libopus0 yt-dlp

# Copy the build artifact from the build stage.
COPY --from=builder /app/target/release/riveting-bot /app/riveting-bot

# Run as non-root.
# USER 1000:1000

# Set the startup command.
ENTRYPOINT ["/app/riveting-bot"]
