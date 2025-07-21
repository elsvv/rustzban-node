FROM rust:1.86-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .
ENV OPENSSL_NO_VENDOR=1
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates curl unzip && \
    curl -L https://github.com/Gozargah/Marzban-scripts/raw/master/install_latest_xray.sh | bash && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rustzban-node /usr/local/bin/
EXPOSE 62050 62051
CMD ["rustzban-node"] 