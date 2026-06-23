# syntax=docker/dockerfile:1.7

# Base image with cargo-chef installed. cargo-chef computes a "recipe" of the
# dependency graph so we can cache the (slow) dependency build separately from
# the (fast) workspace build — incremental image rebuilds become seconds
# instead of minutes when only application code changes.
#
# Keep the Rust version in sync with rust-toolchain.toml.
FROM rust:1.94-alpine3.20 AS chef
WORKDIR /usr/src/app
RUN apk add --no-cache build-base openssl-dev pkgconfig curl
RUN cargo install cargo-chef --locked

# Stage 1: planner — produce recipe.json describing the dep graph only.
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: builder — cook deps from the recipe (cached layer), then build app.
FROM chef AS builder
# SQLx compile-time query checks read from `.sqlx/` instead of a live DB.
# Run `cargo sqlx prepare --workspace` against a real DB whenever queries
# change, then commit the updated `.sqlx/` directory.
ENV SQLX_OFFLINE=true
COPY --from=planner /usr/src/app/recipe.json recipe.json
# This layer is cached as long as Cargo.toml/Cargo.lock don't change.
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

# Stage 3: runtime image.
FROM alpine:3.20
WORKDIR /usr/src/app
RUN apk add --no-cache openssl ca-certificates
COPY --from=builder /usr/src/app/target/release/customer-profile-service .

EXPOSE 8000

CMD ["./customer-profile-service"]
