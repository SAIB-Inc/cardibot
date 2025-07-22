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

Additionally, CardiBot can continuously sync GitHub issue status back to Discord:
1. Polls GitHub for issues created by CardiBot
2. Matches them to Discord threads via embedded IDs
3. Updates Discord thread status based on GitHub changes
4. Maintains synchronization without webhooks or databases

### Data Flow

#### Issue Creation Flow
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

#### Issue Sync Flow (Polling)
```
Every 10 seconds:
         ↓
Query GitHub for recent issues with thread IDs
         ↓
For each issue:
    - Extract thread ID from title
    - Check Discord thread status
    - Update if needed:
        * Closed → Lock thread
        * Reopened → Unlock thread
        * Assignee → Post update
         ↓
Sleep until next cycle
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        CardiBot                              │
│  ┌─────────────────┐        ┌──────────────────────────┐    │
│  │ Discord Handler │        │    Sync Task (Polling)   │    │
│  │                 │        │                          │    │
│  │ /issue create   │        │ Every 10s:             │    │
│  │ → Create Issue  │        │ 1. Query GitHub        │    │
│  └─────────────────┘        │ 2. Check Discord       │    │
│                             │ 3. Update Threads      │    │
│                             └──────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                    ↓                           ↓
┌─────────────────────────────────────────┐    │
│            Discord Forum                 │    │
│  ┌─────────────────────────────────┐    │    │
│  │ Thread: "Bug with login [12345]"│    │    │
│  │ Status: 🔒 Locked              │←───┼────┘
│  │ Pinned: GitHub #123 ←─────────┐ │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
                                     │
                               Cross-linked
                                     │
┌─────────────────────────────────────────┐
│            GitHub Issues                 │
│  ┌─────────────────────────────────┐    │
│  │ Issue #123: Bug with login [12345]   │
│  │ Status: Closed                  │    │
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
- Adds thread ID to issue title for tracking

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

### 4. Automatic Issue Sync (Polling-based)
The sync feature runs continuously in the background:

#### Sync Logic
1. **Simple Query**
   - Search query: `repo:owner/name is:open [1-9] in:title`
   - Fetches ALL open issues with thread IDs (created by CardiBot)
   - No date filtering - we sync all open issues
   - Single API call (assuming <100 open issues)

2. **State Synchronization**
   ```rust
   // Pseudo-code for sync logic
   // Step 1: Get all open issues
   let open_issues = github.search_issues(
       &format!("repo:{}/{} is:open [1-9] in:title", owner, repo)
   ).await?;
   
   // Step 2: Get all closed issues that might have open threads
   let closed_issues = github.search_issues(
       &format!("repo:{}/{} is:closed [1-9] in:title", owner, repo)
   ).await?;
   
   // Step 3: Sync each issue
   for issue in open_issues {
       let thread_id = extract_thread_id(issue.title);
       if let Ok(thread) = discord.get_thread(thread_id).await {
           if thread.locked {
               thread.unlock("Issue is open on GitHub").await?;
               thread.send("🔓 Issue reopened on GitHub").await?;
           }
       }
   }
   
   for issue in closed_issues {
       let thread_id = extract_thread_id(issue.title);
       if let Ok(thread) = discord.get_thread(thread_id).await {
           if !thread.locked {
               thread.lock("Issue closed on GitHub").await?;
               thread.add_reaction("✅").await?;
               thread.send("🔒 Issue closed on GitHub").await?;
           }
       }
   }
   ```

3. **Discord Updates**
   - **Issue Closed**: Lock thread, add ✅ reaction, post closure message
   - **Issue Open**: Unlock thread (if locked), ensure it's accessible
   - **Thread Not Found**: Log and skip (thread might be deleted)

4. **Why This Approach Works**
   - **Simple**: No complex state tracking or caching
   - **Complete**: Every sync sees the full picture
   - **Efficient**: 1-2 API calls per project per sync
   - **Reliable**: No missed updates or edge cases
   - **Scalable**: Works fine up to ~100 open issues per repo

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
# Discord bot token
DISCORD_TOKEN=your_bot_token

# GitHub authentication (choose one)
# Option 1: Personal Access Token
GITHUB_TOKEN=your_github_token

# Option 2: GitHub App (recommended - creates issues as bot user)
GITHUB_APP_ID=your_app_id
GITHUB_APP_INSTALLATION_ID=your_installation_id
GITHUB_APP_PRIVATE_KEY_PATH=/path/to/private-key.pem
# Or for production (Docker/Railway):
GITHUB_APP_PRIVATE_KEY=-----BEGIN RSA PRIVATE KEY-----...
```

### Configuration File (config.toml)
```toml
# Global settings (optional)
log_level = "info"

# Sync configuration (optional - defaults shown)
[sync]
enabled = true                    # Enable/disable sync globally
interval_seconds = 10             # Poll every 10 seconds

# Project 1
[[projects]]
name = "Main Game"
discord_guild_id = "123456789012345678"
discord_forum_id = "987654321098765432"
github_owner = "myorg"
github_repo = "game-bugs"
allowed_role_id = "123456789012345678"  # Role ID for permissions

# Project 2
[[projects]]
name = "Documentation"
discord_guild_id = "123456789012345678"  # Same server
discord_forum_id = "876543210987654321"  # Different forum
github_owner = "myorg"
github_repo = "game-docs"
allowed_role_id = "234567890123456789"

# Project 3
[[projects]]
name = "Community Tools"
discord_guild_id = "234567890123456789"  # Different server
discord_forum_id = "765432109876543210"
github_owner = "community"
github_repo = "tools"
# No allowed_role_id = anyone can create issues
```

This allows:
- Multiple forums in the same Discord server → different repos
- Multiple Discord servers → different repos
- Different permissions per project

## Implementation Status

### Phase 1: Core Bot ✅ (Completed)
- [x] Discord bot with slash commands
- [x] GitHub issue creation with thread ID tracking
- [x] Duplicate prevention (updates existing issues)
- [x] Role-based permissions
- [x] Multi-project support
- [x] GitHub App authentication (bot user)
- [x] Docker deployment with CI/CD
- [x] Production deployment on Railway

### Phase 2: Issue Sync 🚧 (Next)
- [ ] Background polling task (tokio interval)
- [ ] GitHub search for open/closed issues
- [ ] Discord thread state management
- [ ] Lock/unlock threads based on issue status
- [ ] Status messages in threads
- [ ] Sync configuration options
- [ ] Error handling and retry logic
- [ ] Metrics and logging

### Phase 3: Enhanced Sync (Future)
- [ ] Assignee notifications
- [ ] Label to Discord tag mapping
- [ ] Milestone tracking
- [ ] PR link notifications
- [ ] Custom status reactions
- [ ] Sync health dashboard
- [ ] Bulk sync command

## Implementation Plan for Sync Feature

### 1. Core Sync Module (`src/sync.rs`)
```rust
pub struct IssueSyncer {
    config: Arc<Config>,
    github: Arc<Octocrab>,
    discord: Arc<Http>,
}

impl IssueSyncer {
    pub async fn start(self) {
        let mut interval = tokio::time::interval(
            Duration::from_secs(self.config.sync.interval_seconds)
        );
        
        loop {
            interval.tick().await;
            self.sync_all_projects().await;
        }
    }
    
    async fn sync_project(&self, project: &Project) -> Result<()> {
        // 1. Search for all issues with thread IDs
        let issues = self.search_issues(project).await?;
        
        // 2. Group by state
        let (open, closed): (Vec<_>, Vec<_>) = issues
            .into_iter()
            .partition(|i| i.state == "open");
        
        // 3. Sync each group
        self.sync_open_issues(project, open).await?;
        self.sync_closed_issues(project, closed).await?;
        
        Ok(())
    }
}
```

### 2. Integration Points
- **Main**: Spawn sync task alongside bot
- **Config**: Add sync settings structure
- **GitHub**: Reuse existing client with search API
- **Discord**: Add thread management methods

### 3. Testing Strategy
- Unit tests for thread ID extraction
- Integration tests with mock APIs
- Manual testing with test Discord server
- Gradual rollout (start with sync disabled)

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

### Current State (v1.0)
- ✅ Discord to GitHub issue creation
- ✅ Role-based permissions
- ✅ Multi-project support
- ✅ GitHub App authentication
- ✅ Production-ready with Docker/CI/CD

### Next Steps (v2.0)
- 🚧 Polling-based sync (GitHub → Discord)
- 🚧 Automatic thread locking on issue closure
- 🚧 Status indicators in Discord

### Why Polling Over Webhooks
1. **No Infrastructure**: No HTTP server, no public URL, no reverse proxy
2. **No Database**: No state to corrupt or migrate
3. **Simple Deployment**: Just set environment variables and run
4. **Reliable**: Can recover from any failure by restarting
5. **Efficient**: 1-2 API calls per project every 10 seconds is negligible

### Performance Estimates
- **API Calls**: ~12 per project per minute (well within limits)
- **Response Time**: <1 second for sync operations
- **Memory Usage**: ~50MB for bot + sync
- **Scalability**: Handles hundreds of open issues per project