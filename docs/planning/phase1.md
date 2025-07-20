# CardiBot Phase 1 Implementation Plan

## Goal
Build the core bot functionality:
- `/issue create` command in Discord forums
- Creates GitHub issue with Discord link
- Posts GitHub link back to Discord
- Multi-project config support
- No webhooks needed for Phase 1

## Technical Stack
- **Language**: Rust
- **Discord**: Serenity 0.12
- **GitHub**: Octocrab 0.32
- **Config**: TOML via `toml` crate
- **Async**: Tokio

## Implementation Steps

### Step 1: Project Setup
```bash
cargo new cardibot
cd cardibot
```

**Dependencies** (`Cargo.toml`):
```toml
[dependencies]
serenity = { version = "0.12", features = ["client", "gateway", "model", "rustls_backend"] }
octocrab = "0.44"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
toml = "0.8"
serde = { version = "1.0", features = ["derive"] }
dotenv = "0.15"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
```

**Project Structure**:
```
cardibot/
├── Cargo.toml
├── .env.example
├── config.toml.example
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── bot.rs
│   ├── commands.rs
│   └── github.rs
```

**.env.example**:
```
DISCORD_TOKEN=your_bot_token_here
GITHUB_TOKEN=ghp_your_token_here
```

**config.toml.example**:
```toml
[[projects]]
discord_guild_id = "YOUR_SERVER_ID"
discord_forum_id = "YOUR_FORUM_CHANNEL_ID"
github_owner = "your-github-username"
github_repo = "your-repo-name"
```

### Step 2: Configuration Module

**src/config.rs**:
```rust
use serde::Deserialize;
use std::fs;
use anyhow::Result;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub webhook_port: Option<u16>,
    pub log_level: Option<String>,
    pub projects: Vec<Project>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Project {
    pub name: Option<String>,
    pub discord_guild_id: String,
    pub discord_forum_id: String,
    pub github_owner: String,
    pub github_repo: String,
    pub allowed_role: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let contents = fs::read_to_string("config.toml")?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }
    
    pub fn find_project(&self, guild_id: u64, channel_id: u64) -> Option<&Project> {
        self.projects.iter().find(|p| {
            p.discord_guild_id == guild_id.to_string() && 
            p.discord_forum_id == channel_id.to_string()
        })
    }
}
```

### Step 3: Discord Bot Structure

**src/bot.rs**:
```rust
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready, application::interaction::Interaction},
    prelude::*,
};
use std::sync::Arc;

pub struct Bot {
    pub config: Arc<crate::config::Config>,
    pub github: Arc<octocrab::Octocrab>,
}

#[async_trait]
impl EventHandler for Bot {
    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!("Bot is ready as {}", ready.user.name);
        
        // Register slash commands
        let commands = vec![
            crate::commands::create_issue_command(),
        ];
        
        for guild in &ready.guilds {
            let _ = guild.id.set_application_commands(&ctx.http, |cmds| {
                for cmd in &commands {
                    cmds.create_application_command(|c| cmd(c));
                }
                cmds
            }).await;
        }
    }
    
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            match command.data.name.as_str() {
                "issue" => crate::commands::handle_issue_command(&ctx, &command, &self).await,
                _ => {}
            }
        }
    }
}
```

### Step 4: Issue Creation Command

**src/commands.rs**:
```rust
use serenity::{
    builder::CreateApplicationCommand,
    model::prelude::*,
    prelude::*,
};
use crate::bot::Bot;

pub fn create_issue_command() -> impl FnOnce(&mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    |cmd| cmd
        .name("issue")
        .description("Manage GitHub issues")
        .create_option(|opt| opt
            .name("create")
            .description("Create a GitHub issue from this thread")
            .kind(CommandOptionType::SubCommand)
        )
}

pub async fn handle_issue_command(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    bot: &Bot,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if in a forum thread
    let channel = command.channel_id.to_channel(&ctx).await?;
    let thread = match channel {
        Channel::Guild(ch) if ch.thread_metadata.is_some() => ch,
        _ => {
            command.create_interaction_response(&ctx, |r| {
                r.kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|d| {
                        d.content("This command only works in forum threads!")
                            .ephemeral(true)
                    })
            }).await?;
            return Ok(());
        }
    };
    
    // Find project config
    let guild_id = command.guild_id.unwrap();
    let parent_id = thread.parent_id.unwrap();
    
    let project = match bot.config.find_project(guild_id.0, parent_id.0) {
        Some(p) => p,
        None => {
            command.create_interaction_response(&ctx, |r| {
                r.kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|d| {
                        d.content("This forum is not configured for issue tracking")
                            .ephemeral(true)
                    })
            }).await?;
            return Ok(());
        }
    };
    
    // Check permissions
    if let Some(required_role) = &project.allowed_role {
        let member = &command.member.as_ref().unwrap();
        let has_role = member.roles.iter().any(|role_id| {
            guild_id.roles(&ctx).await
                .map(|roles| roles.values().any(|r| r.name == *required_role && r.id == *role_id))
                .unwrap_or(false)
        });
        
        if !has_role {
            command.create_interaction_response(&ctx, |r| {
                r.kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|d| {
                        d.content(format!("You need the {} role to create issues", required_role))
                            .ephemeral(true)
                    })
            }).await?;
            return Ok(());
        }
    }
    
    // Create GitHub issue
    let issue = create_github_issue(bot, project, &thread, &ctx).await?;
    
    // Post GitHub link in thread
    thread.send_message(&ctx, |m| {
        m.embed(|e| {
            e.title("GitHub Issue Created")
                .description(format!("**Issue**: {}", issue.html_url))
                .field("Number", format!("#{}", issue.number), true)
                .field("Status", "Open", true)
                .color(0x238636)
        })
    }).await?;
    
    // Respond to interaction
    command.create_interaction_response(&ctx, |r| {
        r.kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|d| {
                d.content(format!("✅ Created issue #{}", issue.number))
                    .ephemeral(true)
            })
    }).await?;
    
    Ok(())
}
```

### Step 5: GitHub Integration

**src/github.rs**:
```rust
use octocrab::models::issues::Issue;
use crate::config::Project;
use serenity::model::channel::GuildChannel;
use anyhow::Result;

pub async fn create_issue(
    github: &octocrab::Octocrab,
    project: &Project,
    thread: &GuildChannel,
    content: String,
) -> Result<Issue> {
    let discord_url = format!(
        "https://discord.com/channels/{}/{}",
        thread.guild_id,
        thread.id
    );
    
    let body = format!(
        "{}\n\n---\n**Discord Thread**: {}\n**Created by**: <@{}>",
        content,
        discord_url,
        thread.owner_id.unwrap_or_default()
    );
    
    let issue = github
        .issues(&project.github_owner, &project.github_repo)
        .create(thread.name.clone())
        .body(body)
        .send()
        .await?;
    
    Ok(issue)
}

pub async fn extract_thread_content(
    ctx: &serenity::prelude::Context,
    thread: &GuildChannel,
) -> Result<String> {
    let messages = thread.messages(&ctx, |m| m.limit(10)).await?;
    
    let content = messages
        .iter()
        .rev()
        .take(5)
        .map(|m| format!("**@{}**: {}", m.author.name, m.content))
        .collect::<Vec<_>>()
        .join("\n\n");
    
    Ok(content)
}
```

### Step 6: Main Application

**src/main.rs**:
```rust
mod config;
mod bot;
mod commands;
mod github;

use std::sync::Arc;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Load environment variables
    dotenv::dotenv().ok();
    
    // Load configuration
    let config = Arc::new(config::Config::load()?);
    tracing::info!("Loaded {} projects", config.projects.len());
    
    // Initialize GitHub client
    let github_token = std::env::var("GITHUB_TOKEN")?;
    let github = Arc::new(
        octocrab::OctocrabBuilder::new()
            .personal_token(github_token)
            .build()?
    );
    
    // Initialize Discord bot
    let discord_token = std::env::var("DISCORD_TOKEN")?;
    let intents = GatewayIntents::GUILDS 
        | GatewayIntents::GUILD_MESSAGES 
        | GatewayIntents::MESSAGE_CONTENT;
    
    let bot = bot::Bot { config, github };
    
    let mut client = Client::builder(&discord_token, intents)
        .event_handler(bot)
        .await?;
    
    // Start the bot
    tracing::info!("Starting CardiBot...");
    client.start().await?;
    
    Ok(())
}
```

### Step 7: Quick Testing

**Manual test flow**:
1. Create test Discord server with forum channel
2. Create test GitHub repo
3. Set up config.toml with test values
4. Run bot
5. Use `/issue create` in a forum thread
6. Verify issue appears in GitHub with Discord link
7. Verify GitHub link appears in Discord

## Deliverables

Phase 1 deliverables:

1. ✅ Working Discord bot that responds to `/issue create`
2. ✅ GitHub issue creation with Discord link
3. ✅ Discord message with GitHub link
4. ✅ Multi-project support via config
5. ✅ Basic permission system
6. ✅ Error handling and logging

## Next Steps (Phase 2 Preview)

- Webhook server for GitHub → Discord updates
- `/issue sync` command
- `/issue close` command
- Better formatting and templates

## Success Criteria

Phase 1 is complete when:
- Bot successfully creates GitHub issues from Discord threads
- Cross-platform links are properly embedded
- Multiple projects can be configured and work independently
- Basic error cases are handled gracefully
- Code is documented and tested