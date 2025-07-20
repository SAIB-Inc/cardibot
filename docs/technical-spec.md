# CardiBot - Technical Specification

## Document Information
- **Version**: 1.0.0
- **Date**: 2025-01-20
- **Status**: Draft

## Overview

CardiBot is a lightweight Discord bot that connects Discord Forums with GitHub Issues. It can handle multiple projects, routing different forum channels to different GitHub repositories. It treats Discord and GitHub as the sources of truth, using embedded links to maintain relationships without requiring a database.

## How It Works

### Core Concept
CardiBot watches a specific Discord forum channel. When someone runs `/issue create` in a thread, it:
1. Creates a GitHub issue with the thread content
2. Embeds the Discord thread URL in the issue body
3. Posts the GitHub issue URL back in the Discord thread
4. No database needed - the links maintain the relationship

### Data Flow
```
User creates thread in forum channel
         ↓
User runs /issue create
         ↓
CardiBot creates GitHub issue (with Discord link)
         ↓
CardiBot posts GitHub link in thread
         ↓
Both platforms now cross-linked
```

## Architecture

```
┌─────────────────────────────────────────┐
│            Discord Forum                 │
│  ┌─────────────────────────────────┐    │
│  │ Thread: "Bug with login"        │    │
│  │ Pinned: GitHub #123 ←─────────┐ │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
                                     │
                               Cross-linked
                                     │
┌─────────────────────────────────────────┐
│            GitHub Issues                 │
│  ┌─────────────────────────────────┐    │
│  │ Issue #123: Bug with login      │    │
│  │ Body: Discord Thread: [...] ←───┘    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
```

## Technical Implementation

### Required Components

1. **Discord Bot** - Handles commands in the forum channel
2. **GitHub Client** - Creates issues
3. **Config Loader** - Reads TOML configuration

### No Need For
- Database
- Config files
- Multiple server support
- Complex state management
- External dependencies beyond Discord/GitHub APIs

## Core Features

### 1. Issue Creation
```
/issue create
```
- Creates GitHub issue from current Discord thread
- Embeds Discord URL in issue body
- Posts GitHub URL as pinned message in thread

### 2. Issue Linking
```
/issue link <github-url>
```
- Links existing GitHub issue to current thread
- Posts GitHub URL as pinned message


### 3. Issue Close
```
/issue close [comment]
```
- Closes the linked GitHub issue
- Posts closing message in Discord

## Data Format

### GitHub Issue Body
```markdown
## Description
[Original Discord content]

## Steps to Reproduce
[If provided]

---
**Discord Thread**: https://discord.com/channels/[guild]/[thread]
**Reported by**: @discorduser
```

### Discord Pinned Message
```markdown
**GitHub Issue**: https://github.com/owner/repo/issues/123
**Status**: Open
**Labels**: bug, priority-high
**Last Updated**: 2025-01-20 10:30 UTC
```

## Configuration

CardiBot uses a simple configuration approach:

### Environment Variables (.env)
```bash
# Only tokens go here for security
DISCORD_TOKEN=your_bot_token
GITHUB_TOKEN=your_github_token
```

### Configuration File (config.toml)
```toml
# Global settings (optional)
log_level = "info"

# Project 1
[[projects]]
name = "Main Game"
discord_guild_id = "123456789012345678"
discord_forum_id = "987654321098765432"
github_owner = "myorg"
github_repo = "game-bugs"
allowed_role = "Beta Tester"

# Project 2
[[projects]]
name = "Documentation"
discord_guild_id = "123456789012345678"  # Same server
discord_forum_id = "876543210987654321"  # Different forum
github_owner = "myorg"
github_repo = "game-docs"
allowed_role = "Contributor"

# Project 3
[[projects]]
name = "Community Tools"
discord_guild_id = "234567890123456789"  # Different server
discord_forum_id = "765432109876543210"
github_owner = "community"
github_repo = "tools"
# No allowed_role = anyone can create issues
```

This allows:
- Multiple forums in the same Discord server → different repos
- Multiple Discord servers → different repos
- Different permissions per project

## Implementation Steps

### Phase 1: Basic Bot (Week 1)
- [ ] Discord bot with slash commands
- [ ] GitHub issue creation
- [ ] Basic link embedding
- [ ] Environment-based config

### Phase 2: Bidirectional Sync (Week 2)
- [ ] GitHub webhook handler
- [ ] Update Discord on issue changes
- [ ] Status synchronization
- [ ] Error handling

### Phase 3: Polish (Week 3)
- [ ] GitHub-based configuration
- [ ] Permission system
- [ ] Better error messages
- [ ] Documentation

## Key Benefits

1. **No State to Manage** - Data lives where it belongs
2. **Always Consistent** - No sync issues possible
3. **Simple Deployment** - Just run the bot
4. **Easy Recovery** - Just restart if something breaks
5. **Low Maintenance** - No database, no migrations

## Limitations

1. **Search** - Can't search across all linked issues easily
2. **Analytics** - Limited to what Discord/GitHub APIs provide
3. **Performance** - Each operation requires API calls
4. **Rate Limits** - Must respect Discord/GitHub limits

## Security

1. **No Data Storage** - Nothing sensitive to leak
2. **Token Security** - Only bot tokens need protection
3. **Permission Checks** - Discord role-based access
4. **Webhook Validation** - Verify GitHub signatures

## Error Handling

### Common Scenarios
1. **Thread not found** → Clear error message
2. **No permissions** → Explain required role
3. **Already linked** → Show existing issue
4. **Rate limited** → Retry with backoff
5. **API down** → Graceful degradation

## Success Metrics

1. **Response Time** < 3 seconds
2. **Uptime** > 99%
3. **Zero Data Loss** (links always preserved)
4. **User Satisfaction** via feedback

## Getting Started (Simple Mode)

### 1. Setup Discord
- Create a Discord application at https://discord.com/developers
- Get your bot token
- Enable Developer Mode in Discord (Settings → Advanced)
- Right-click your server → Copy ID (this is `DISCORD_GUILD_ID`)
- Right-click your forum channel → Copy ID (this is `DISCORD_FORUM_ID`)

### 2. Setup GitHub
- Create a Personal Access Token with `repo` scope
- Note your GitHub username/org and repository name

### 3. Configure CardiBot
Create a `.env` file for tokens:
```bash
DISCORD_TOKEN=your_bot_token_here
GITHUB_TOKEN=ghp_your_token_here
```

Create a `config.toml` file:

For a single project:
```toml
[[projects]]
discord_guild_id = "123456789012345678"
discord_forum_id = "987654321098765432"
github_owner = "your-github-username"
github_repo = "your-repo-name"
```

For multiple projects, just add more:
```toml
[[projects]]
name = "Bug Reports"
discord_guild_id = "123456789012345678"
discord_forum_id = "987654321098765432"
github_owner = "myorg"
github_repo = "bugs"

[[projects]]
name = "Feature Requests"
discord_guild_id = "123456789012345678"
discord_forum_id = "876543210987654321"
github_owner = "myorg"
github_repo = "features"
```

### 4. Run CardiBot
```bash
cargo run
```

That's it! CardiBot will now watch your forum channel and create GitHub issues when users run `/issue create`.

## Summary

CardiBot provides 90% of the value with 10% of the complexity. By using Discord and GitHub as our databases, we eliminate entire categories of problems while maintaining a simple, reliable system that's easy to understand and operate.