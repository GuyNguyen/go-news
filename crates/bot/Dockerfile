FROM rust:alpine AS builder

WORKDIR /app
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static build-base \
    && rustup target add x86_64-unknown-linux-musl

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release --target x86_64-unknown-linux-musl --bin go-news-bot

FROM gcr.io/distroless/static-debian12:nonroot

WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/go-news-bot /app/go-news

USER nonroot:nonroot

CMD ["/app/go-news"]
