# FROM rust:slim AS builder
FROM rustlang/rust:nightly-slim AS builder

RUN apt update && apt upgrade -y

# Create a new empty shell project
RUN USER=root cargo new --bin app

WORKDIR /app

# Copy manifests
COPY ./Cargo.lock ./Cargo.toml ./

# Build only the dependencies to cache them
RUN cargo build --release

# Remove default code from deps build
RUN rm ./src/*.rs && rm ./target/release/deps/riveting_bot*

# Copy the source code
COPY ./src ./src

# Build for release
RUN cargo build --release && strip --strip-all ./target/release/riveting-bot

# Final
FROM gcr.io/distroless/cc

# Copy the build artifact from the build stage
COPY --from=builder /app/target/release/riveting-bot /

# Run as non-root
USER 1000:1000

# Set the startup command
CMD ["/riveting-bot"]