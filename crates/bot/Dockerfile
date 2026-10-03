FROM rust:alpine AS builder

WORKDIR /app
RUN USER=root \
    && apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static build-base

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release --bin go-news-bot

FROM alpine:latest

WORKDIR /app
RUN apk update && apk add --no-cache openssl ca-certificates \
    && addgroup -S -g 10001 appgroup \
    && adduser -S -u 10001 -G appgroup appuser

COPY --from=builder /app/target/release/go-news-bot ./go-news
RUN chown appuser:appgroup ./go-news

USER appuser:appgroup

CMD ["./go-news"]
