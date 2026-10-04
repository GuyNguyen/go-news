FROM rust:alpine AS builder

WORKDIR /app
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static build-base \
    && rustup target add x86_64-unknown-linux-musl

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release --target x86_64-unknown-linux-musl --bin go-news-backend

# Prepare data directory owned by distroless nonroot user (UID 65532)
RUN mkdir -p /app/data && chown -R 65532:65532 /app/data

FROM gcr.io/distroless/static-debian12:nonroot

WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/go-news-backend /app/go-news-backend
COPY --from=builder --chown=65532:65532 /app/data /app/data

USER nonroot:nonroot

CMD ["/app/go-news-backend"]
