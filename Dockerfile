FROM lukemathwalker/cargo-chef:latest-rust-bookworm AS chef
WORKDIR /bszet-mind

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY --from=planner /bszet-mind/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin bszet-mind

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime

ARG ENVIRONMENT

ENV USER=bszet-mind
ENV UID=10001

ENV BSZET_MIND_LISTEN_ADDR=0.0.0.0:8080
ENV BSZET_MIND_INTERNAL_LISTEN_ADDR=0.0.0.0:8081
ENV BSZET_MIND_ENVIRONMENT=${ENVIRONMENT}

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

WORKDIR /bszet-mind
COPY --from=builder /bszet-mind/target/release/bszet-mind /usr/local/bin

USER ${USER}:${USER}
ENTRYPOINT ["/usr/local/bin/bszet-mind"]
