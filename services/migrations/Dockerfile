FROM rust:1.75-slim-bookworm

# Install SQLx CLI with postgres features
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install sqlx-cli --no-default-features --features native-tls,postgres

# Create working directory
WORKDIR /migrations

# Copy migration files
COPY services/migrations/*.sql ./

# Run migrations and exit
CMD ["sqlx", "migrate", "run"]