# CardiBot 🤖

A Discord-GitHub bridge bot that automatically creates GitHub issues from Discord forum posts. Built with Rust for performance and reliability.

## Features

- **Discord to GitHub**: Creates GitHub issues from Discord forum posts with `/issue create` command
- **Role-based permissions**: Restrict issue creation to specific Discord roles
- **Tag mapping**: Automatically maps Discord tags ([BUG], [FEATURE], etc.) to GitHub labels
- **Duplicate prevention**: Updates existing issues instead of creating duplicates
- **GitHub App authentication**: Creates issues as a bot user (cardibot[bot])
- **Multi-project support**: Configure multiple Discord servers and GitHub repositories

## Quick Start

### Prerequisites

- Discord bot token
- GitHub App or Personal Access Token
- Rust 1.88+ (for local development)
- Docker (for deployment)

### Local Development

1. Clone the repository:
```bash
git clone https://github.com/SAIB-Inc/cardibot.git
cd cardibot
```

2. Create a `.env` file:
```bash
cp .env.example .env
# Edit .env with your credentials
```

3. Create a `config.toml` file:
```bash
cp config.toml.example config.toml
# Edit config.toml with your project settings
```

4. Run the bot:
```bash
cargo run -- run
```

## Configuration

### Environment Variables

Create a `.env` file with:

```env
DISCORD_TOKEN=your_bot_token_here

# Option 1: Personal Access Token (PAT)
GITHUB_TOKEN=ghp_your_token_here

# Option 2: GitHub App authentication (recommended)
GITHUB_APP_ID=your_app_id
GITHUB_APP_INSTALLATION_ID=your_installation_id
GITHUB_APP_PRIVATE_KEY_PATH=/path/to/private-key.pem
```

### Project Configuration

Create a `config.toml` file:

```toml
log_level = "info"

[[projects]]
name = "Your Project Name"
discord_guild_id = "YOUR_SERVER_ID"
discord_forum_id = "YOUR_FORUM_CHANNEL_ID"
github_owner = "your-github-org"
github_repo = "your-repo-name"
allowed_role_id = "YOUR_ROLE_ID"  # Optional: restrict to role
```

## Discord Setup

1. Create a Discord application at https://discord.com/developers/applications
2. Create a bot and copy the token
3. Invite the bot to your server with these permissions:
   - Read Messages
   - Send Messages
   - Use Slash Commands
   - Read Message History
4. Get your forum channel ID (right-click → Copy ID with developer mode enabled)

## GitHub App Setup

For professional deployments, use a GitHub App instead of PAT:

1. Create a GitHub App in your organization settings
2. Set permissions: Issues (Read & Write), Metadata (Read)
3. Generate and download a private key
4. Install the app on your repository
5. Note the App ID and Installation ID

## Deployment

### Railway

1. Deploy using the Railway button or CLI
2. Set environment variables:
   ```
   DISCORD_TOKEN=...
   GITHUB_APP_ID=...
   GITHUB_APP_INSTALLATION_ID=...
   GITHUB_APP_PRIVATE_KEY=<paste entire PEM content>
   PROJECT_NAME=...
   DISCORD_GUILD_ID=...
   DISCORD_FORUM_ID=...
   GITHUB_OWNER=...
   GITHUB_REPO=...
   ALLOWED_ROLE_ID=...
   ```

### Docker

```bash
docker pull ghcr.io/saib-inc/cardibot:latest
docker run -e DISCORD_TOKEN=... ghcr.io/saib-inc/cardibot:latest
```

## Usage

1. Create a forum post in your designated Discord forum channel
2. Use tags like [BUG], [FEATURE], [FEEDBACK], or [QUESTION]
3. Run `/issue create` in the forum post
4. CardiBot creates a GitHub issue with:
   - Thread title as issue title
   - Thread content and messages
   - Appropriate labels based on tags
   - Link back to Discord thread
   - Discord username attribution

## CLI Commands

```bash
# Run the bot
cargo run -- run

# Validate configuration
cargo run -- validate-config

# Check Discord connection
cargo run -- check-discord

# Post feedback instructions
cargo run -- post-feedback --channel CHANNEL_ID
```

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/amazing-feature`
3. Commit changes: `git commit -m 'feat: add amazing feature'`
4. Push to branch: `git push origin feat/amazing-feature`
5. Open a Pull Request

## License

MIT License - see LICENSE file for details

## Roadmap

- [ ] Two-way sync: Update Discord threads when GitHub issues change
- [ ] Close Discord threads when GitHub issues are closed
- [ ] Add reactions to show issue status
- [ ] Support for multiple forums per project
- [ ] Custom label mappings
- [ ] Issue templates