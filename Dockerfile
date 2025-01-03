FROM rust:1.83.0-slim-bookworm AS builder


RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    musl-tools \
    musl-dev \
    cmake \
    gcc \
    gcc-multilib \
    g++-multilib \
    binutils \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-musl
# Set AR explicitly
ENV AR=/usr/bin/ar
ENV CC=/usr/bin/musl-gcc

WORKDIR /app

COPY . .

RUN cargo build --release --locked --bin proxy --target x86_64-unknown-linux-musl

FROM alpine

RUN addgroup -S ferrix && adduser -S ferrix -G ferrix

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/proxy /usr/local/bin/
RUN chmod +x /usr/local/bin/proxy

USER ferrix

ENTRYPOINT ["/usr/local/bin/proxy"]
