# Multi-stage Native Rust build for 9Router
FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev sqlite-dev

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY tests ./tests

RUN cargo build --release

FROM alpine:3.21 AS runner

RUN apk add --no-cache ca-certificates

WORKDIR /app
COPY --from=builder /app/target/release/9router /usr/local/bin/9router

ENV PORT=20128
ENV HOST=0.0.0.0
ENV DATA_DIR=/app/data

RUN mkdir -p /app/data

EXPOSE 20128

ENTRYPOINT ["9router"]
CMD ["start"]
