# FROM rust:slim AS builder
FROM rustlang/rust:nightly-slim AS builder

# Use sparse registry.
ARG CARGO_UNSTABLE_SPARSE_REGISTRY=true

# Update OS.
RUN apt update && apt upgrade -y
# RUN apt install -y pkg-config

# Create a new empty shell project.
RUN USER=root cargo new --bin app

WORKDIR /app

# Copy manifests.
# COPY ./.cargo ./.cargo
COPY ./Cargo.lock ./Cargo.toml ./

# Build only the dependencies to cache them.
RUN cargo build --release

# Remove default code from deps build.
RUN rm ./src/*.rs && rm ./target/release/deps/riveting_bot*

# Copy the source code.
COPY ./src ./src

# Build for release.
RUN cargo build --release && strip --strip-all ./target/release/riveting-bot

# Final.
FROM gcr.io/distroless/cc
# FROM ubuntu:latest
# RUN apt update && apt upgrade -y && apt install ca-certificates -y && update-ca-certificates

# Copy the build artifact from the build stage.
COPY --from=builder /app/target/release/riveting-bot /app/riveting-bot

# Run as non-root.
# USER 1000:1000

# Set the startup command.
ENTRYPOINT ["/app/riveting-bot"]
