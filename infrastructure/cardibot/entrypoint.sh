#!/bin/bash

# Substitute environment variables in config
envsubst < /app/production-config.toml > /app/config.toml

# Handle GitHub App private key if provided directly
if [ ! -z "$GITHUB_APP_PRIVATE_KEY" ]; then
    echo "Writing GitHub App private key..."
    mkdir -p /app/secrets
    echo "$GITHUB_APP_PRIVATE_KEY" > /app/secrets/github-app.pem
    export GITHUB_APP_PRIVATE_KEY_PATH=/app/secrets/github-app.pem
fi

# Run the bot
exec ./cardibot run