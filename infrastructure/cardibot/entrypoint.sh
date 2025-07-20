#!/bin/bash

# Substitute environment variables in config
envsubst < /app/production-config.toml > /app/config.toml

# Run the bot
exec ./cardibot run