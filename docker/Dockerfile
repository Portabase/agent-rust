## =========================
## Base image (shared)
## =========================
#FROM rust:1.92.0 AS base
#
#RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y \
#    pkg-config \
#    libssl-dev \
#    tzdata \
#    redis-server \
#    ca-certificates \
#    curl \
#    && apt-get clean \
#    && rm -rf /var/lib/apt/lists/*
#
#
#
## ========= Install PostgreSQL client binaries (versions 12-18) =========
## Pre-downloaded binaries from assets/tools/ - no network download needed
#ARG TARGETARCH
#RUN mkdir -p /usr/lib/postgresql/12/bin /usr/lib/postgresql/13/bin \
#  /usr/lib/postgresql/14/bin /usr/lib/postgresql/15/bin \
#  /usr/lib/postgresql/16/bin /usr/lib/postgresql/17/bin \
#  /usr/lib/postgresql/18/bin
#
## Copy pre-downloaded PostgreSQL binaries based on architecture
#COPY assets/tools/x64/postgresql/ /tmp/pg-x64/
#COPY assets/tools/arm/postgresql/ /tmp/pg-arm/
#RUN if [ "$TARGETARCH" = "amd64" ]; then \
#  cp -r /tmp/pg-x64/postgresql-12/bin/* /usr/lib/postgresql/12/bin/ && \
#  cp -r /tmp/pg-x64/postgresql-13/bin/* /usr/lib/postgresql/13/bin/ && \
#  cp -r /tmp/pg-x64/postgresql-14/bin/* /usr/lib/postgresql/14/bin/ && \
#  cp -r /tmp/pg-x64/postgresql-15/bin/* /usr/lib/postgresql/15/bin/ && \
#  cp -r /tmp/pg-x64/postgresql-16/bin/* /usr/lib/postgresql/16/bin/ && \
#  cp -r /tmp/pg-x64/postgresql-17/bin/* /usr/lib/postgresql/17/bin/ && \
#  cp -r /tmp/pg-x64/postgresql-18/bin/* /usr/lib/postgresql/18/bin/; \
#  elif [ "$TARGETARCH" = "arm64" ]; then \
#  cp -r /tmp/pg-arm/postgresql-12/bin/* /usr/lib/postgresql/12/bin/ && \
#  cp -r /tmp/pg-arm/postgresql-13/bin/* /usr/lib/postgresql/13/bin/ && \
#  cp -r /tmp/pg-arm/postgresql-14/bin/* /usr/lib/postgresql/14/bin/ && \
#  cp -r /tmp/pg-arm/postgresql-15/bin/* /usr/lib/postgresql/15/bin/ && \
#  cp -r /tmp/pg-arm/postgresql-16/bin/* /usr/lib/postgresql/16/bin/ && \
#  cp -r /tmp/pg-arm/postgresql-17/bin/* /usr/lib/postgresql/17/bin/ && \
#  cp -r /tmp/pg-arm/postgresql-18/bin/* /usr/lib/postgresql/18/bin/; \
#  fi && \
#  rm -rf /tmp/pg-x64 /tmp/pg-arm && \
#  chmod +x /usr/lib/postgresql/*/bin/*
#
#
#
#WORKDIR /app
#
## =========================
## Development image
## =========================
#FROM base AS dev
#
#RUN cargo install cargo-watch
#
## Pre-cache dependencies
#COPY Cargo.toml Cargo.lock ./
#RUN mkdir src && echo "fn main() {}" > src/main.rs
#RUN cargo build
#RUN rm -rf src
#
#COPY entrypoint.sh /entrypoint.sh
#RUN chmod +x /entrypoint.sh
#
#CMD ["/entrypoint.sh"]
#
## =========================
## Builder (production)
## =========================
#FROM base AS builder
#
#COPY . .
#RUN cargo build --release
#
#RUN echo "APP_VERSION=$(cargo pkgid | awk -F# '{print $2}')" > /app/version.env
#
## =========================
## Runtime (production)
## =========================
#FROM debian:bookworm-slim AS prod
##FROM ubuntu:24.04 AS prod
#
#
#RUN apt-get update && apt-get install -y \
#    redis-server \
#    ca-certificates \
#    tzdata \
#    postgresql-client  \
#    libpq5  \
#    mariadb-client \
#    && rm -rf /var/lib/apt/lists/*
#
#WORKDIR /app
#
#COPY --from=builder /app/target/release/app /usr/local/bin/app
#COPY --from=builder /app/version.env /app/version.env
#COPY entrypoint.sh /entrypoint.sh
#RUN chmod +x /entrypoint.sh
#
#ENV APP_ENV=production
#
#CMD ["/entrypoint.sh"]
# =========================
# Base image (shared)
# =========================
FROM rust:1.92.0 AS base

# Install common dependencies
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y \
    pkg-config \
    libssl-dev \
    tzdata \
    redis-server \
    ca-certificates \
    libpq5 \
    curl \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*


# =========================
# PostgreSQL client binaries (versions 12-18)
# =========================
ARG TARGETARCH

RUN for v in 12 13 14 15 16 17 18; do \
        mkdir -p /usr/lib/postgresql/$v/bin; \
    done

COPY assets/tools/x64/postgresql/ /tmp/pg-x64/
COPY assets/tools/arm/postgresql/ /tmp/pg-arm/

RUN if [ "$TARGETARCH" = "amd64" ]; then \
        for v in 12 13 14 15 16 17 18; do \
            cp -r /tmp/pg-x64/postgresql-$v/bin/* /usr/lib/postgresql/$v/bin/; \
        done; \
    elif [ "$TARGETARCH" = "arm64" ]; then \
        for v in 12 13 14 15 16 17 18; do \
            cp -r /tmp/pg-arm/postgresql-$v/bin/* /usr/lib/postgresql/$v/bin/; \
        done; \
    fi && \
    rm -rf /tmp/pg-x64 /tmp/pg-arm && \
    chmod +x /usr/lib/postgresql/*/bin/*


WORKDIR /app

# =========================
# Development image
# =========================
FROM base AS dev

RUN cargo install cargo-watch

# Pre-cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build
RUN rm -rf src

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

CMD ["/entrypoint.sh"]

# =========================
# Builder (production)
# =========================
FROM base AS builder

COPY . .
RUN cargo build --release

RUN echo "APP_VERSION=$(cargo pkgid | awk -F# '{print $2}')" > /app/version.env

# =========================
# Runtime (production)
# =========================
FROM ubuntu:24.04 AS prod

RUN apt-get update && apt-get install -y \
    redis-server \
    ca-certificates \
    tzdata \
    libpq5 \
    mariadb-client \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/app /usr/local/bin/app
COPY --from=builder /app/version.env /app/version.env
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

COPY --from=base /usr/lib/postgresql/ /usr/lib/postgresql/

ENV APP_ENV=production

CMD ["/entrypoint.sh"]
