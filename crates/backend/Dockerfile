FROM rust:alpine AS builder

WORKDIR /app
RUN USER=root \
    && apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static build-base

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release --bin go-news-backend

FROM alpine:latest

WORKDIR /app
RUN apk update && apk add --no-cache openssl ca-certificates sqlite-libs \
    && addgroup -S -g 10001 appgroup \
    && adduser -S -u 10001 -G appgroup appuser \
    && mkdir -p /app/data \
    && chown -R appuser:appgroup /app

COPY --from=builder /app/target/release/go-news-backend .
RUN chown appuser:appgroup ./go-news-backend

USER appuser:appgroup

CMD ["./go-news-backend"]
