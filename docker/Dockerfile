# =========================
# Base Rust image
# =========================
FROM rust:1.92 AS base

ARG TARGETARCH

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y \
    pkg-config \
    libssl-dev \
    tzdata \
    redis-server \
    ca-certificates \
    libpq5 \
    libreadline8 \
    libncurses6 \
    zlib1g \
    curl \
    mariadb-client \
    wget \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# =========================
# PostgreSQL client binaries (only target arch)
# =========================
RUN for v in 12 13 14 15 16 17 18; do mkdir -p /usr/lib/postgresql/$v/bin; done
COPY assets/tools/$TARGETARCH/postgresql/ /tmp/pg/
RUN for v in 12 13 14 15 16 17 18; do \
        cp -r /tmp/pg/postgresql-$v/bin/* /usr/lib/postgresql/$v/bin/; \
    done && rm -rf /tmp/pg && chmod +x /usr/lib/postgresql/*/bin/*

# =========================
# MongoDB client binaries (only target arch)
# =========================
RUN if [ "$TARGETARCH" = "amd64" ]; then \
        wget -q https://fastdl.mongodb.org/tools/db/mongodb-database-tools-debian12-x86_64-100.10.0.deb -O /tmp/mongodb-tools.deb; \
    elif [ "$TARGETARCH" = "arm64" ]; then \
        wget -q https://fastdl.mongodb.org/tools/db/mongodb-database-tools-ubuntu2204-arm64-100.10.0.deb -O /tmp/mongodb-tools.deb; \
    fi && dpkg -i /tmp/mongodb-tools.deb || apt-get install -f -y --no-install-recommends && \
    rm -f /tmp/mongodb-tools.deb && \
    mkdir -p /usr/local/mongodb/bin && \
    ln -sf /usr/bin/mongodump /usr/local/mongodb/bin/mongodump || true && \
    ln -sf /usr/bin/mongorestore /usr/local/mongodb/bin/mongorestore || true

WORKDIR /app

# =========================
# Development stage
# =========================
FROM base AS dev

RUN cargo install cargo-watch

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build
RUN rm -rf src

COPY . .

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

CMD ["/entrypoint.sh"]

# =========================
# Builder stage for production
# =========================
FROM base AS builder
WORKDIR /app

RUN cargo install cargo-chef

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo chef prepare --recipe-path recipe.json
RUN cargo chef cook --recipe-path recipe.json --release
RUN rm -rf src

COPY . .
RUN cargo build --release

RUN echo "APP_VERSION=$(cargo pkgid | awk -F# '{print $2}')" > /app/version.env

# =========================
# Production runtime
# =========================
FROM ubuntu:24.04 AS prod

RUN apt-get update && apt-get install -y \
    redis-server \
    ca-certificates \
    tzdata \
    libpq5 \
    libreadline8 \
    libncurses6 \
    zlib1g \
    mariadb-client \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/app /usr/local/bin/app
COPY --from=builder /app/version.env /app/version.env

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

COPY --from=base /usr/lib/postgresql/ /usr/lib/postgresql/
COPY --from=base /usr/local/mongodb/bin /usr/local/mongodb/bin

ENV APP_ENV=production

CMD ["/entrypoint.sh"]
