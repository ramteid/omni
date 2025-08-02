# Multi-stage build for Google connector service
FROM lukemathwalker/cargo-chef:latest AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release --bin clio-google-connector

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y \
    ca-certificates \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Download and install pdfium binary (architecture-aware)
ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
        "linux/amd64") PDFIUM_ARCH="x64" ;; \
        "linux/arm64" | "linux/arm64/v8") PDFIUM_ARCH="arm64" ;; \
        *) echo "Unsupported platform: $TARGETPLATFORM" && exit 1 ;; \
    esac && \
    mkdir /tmp/pdfium && cd /tmp/pdfium && \
    wget https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/7242/pdfium-linux-${PDFIUM_ARCH}.tgz && \
    tar -xzf pdfium-linux-${PDFIUM_ARCH}.tgz && \
    mv lib/libpdfium.so /usr/local/lib/ && \
    ldconfig && \
    rm -rf /tmp/pdfium

WORKDIR /app
COPY --from=builder /app/target/release/clio-google-connector /usr/local/bin/clio-google-connector

EXPOSE 4001
CMD ["clio-google-connector"]
