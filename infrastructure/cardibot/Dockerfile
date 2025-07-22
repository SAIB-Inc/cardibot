# Build stage
FROM rust:1.88-slim AS builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo files first for better caching
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build the application in release mode
RUN cargo build --release --bin cardibot

# Runtime stage
FROM debian:bookworm-slim
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    gettext-base \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/cardibot /app/cardibot

# Copy configuration and entrypoint
COPY infrastructure/cardibot/production-config.toml /app/production-config.toml
COPY infrastructure/cardibot/entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

# Create a non-root user
RUN useradd -m -u 1001 cardibot && \
    chown -R cardibot:cardibot /app

USER cardibot

# Environment variables (will be provided by Railway)
ENV DISCORD_TOKEN=""
# GitHub authentication - either PAT or App
ENV GITHUB_TOKEN=""
ENV GITHUB_APP_ID=""
ENV GITHUB_APP_INSTALLATION_ID=""
ENV GITHUB_APP_PRIVATE_KEY=""
# Project configuration
ENV PROJECT_NAME="Levvy V3"
ENV DISCORD_GUILD_ID=""
ENV DISCORD_FORUM_ID=""
ENV GITHUB_OWNER=""
ENV GITHUB_REPO=""
ENV ALLOWED_ROLE_ID=""

# Run the bot
ENTRYPOINT ["/app/entrypoint.sh"]