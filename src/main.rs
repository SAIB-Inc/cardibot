mod archive_threads;
mod audit_sync;
mod bot;
mod cli;
mod clients;
mod commands;
mod config;
mod constants;
mod debug;
mod debug_sync;
mod github;
mod github_app;
mod sync;

use anyhow::Result;
use clap::Parser;
use serenity::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Parse CLI arguments
    let cli = cli::Cli::parse();

    match cli.command {
        cli::Commands::CheckDiscord => {
            println!("Checking Discord configuration...");
            debug::check_discord().await?;
        }
        cli::Commands::PostFeedback { channel } => {
            println!("Posting feedback instructions to channel {channel}...");
            debug::post_feedback_instructions(&channel).await?;
        }
        cli::Commands::ValidateConfig => {
            println!("Validating configuration...");
            match config::Config::load() {
                Ok(config) => {
                    println!("✓ Configuration is valid!");
                    println!(
                        "  - Log level: {}",
                        config.log_level.as_deref().unwrap_or("info")
                    );
                    println!("  - Projects configured: {}", config.projects.len());
                    for (i, project) in config.projects.iter().enumerate() {
                        println!("\n  Project {}:", i + 1);
                        println!(
                            "    - Name: {}",
                            project.name.as_deref().unwrap_or("(unnamed)")
                        );
                        println!("    - Discord Guild: {}", project.discord_guild_id);
                        println!("    - Discord Forum: {}", project.discord_forum_id);
                        println!(
                            "    - GitHub: {}/{}",
                            project.github_owner, project.github_repo
                        );
                        if let Some(role_id) = &project.allowed_role_id {
                            println!("    - Required Role ID: {role_id}");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("✗ Configuration error: {e}");
                    std::process::exit(1);
                }
            }
        }
        cli::Commands::DebugSync => {
            debug_sync::debug_sync_status().await?;
        }
        cli::Commands::ArchiveLockedThreads => {
            archive_threads::archive_locked_threads().await?;
        }
        cli::Commands::AuditSync => {
            audit_sync::audit_sync_status().await?;
        }
        cli::Commands::Run => {
            // Load configuration first to get log level
            let config = Arc::new(config::Config::load()?);

            // Initialize logging with configured level
            let log_level = config.log_level.as_deref().unwrap_or("info");
            use tracing_subscriber::EnvFilter;

            // Build filter to exclude octocrab and HTTP client deprecation warnings
            let filter = EnvFilter::new(format!(
                "{log_level},octocrab=warn,reqwest=warn,hyper=warn"
            ));

            tracing_subscriber::fmt().with_env_filter(filter).init();

            tracing::info!("Loaded {} projects", config.projects.len());

            // Initialize GitHub client (supports both App and PAT auth)
            let github = Arc::new(github_app::create_github_client().await?);

            // Initialize Discord bot
            let discord_token = std::env::var("DISCORD_TOKEN")?;
            let intents = GatewayIntents::GUILDS
                | GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::MESSAGE_CONTENT;

            let bot = bot::Bot {
                config: config.clone(),
                github: github.clone(),
            };

            let mut client = Client::builder(&discord_token, intents)
                .event_handler(bot)
                .await?;

            // Create shared clients for sync task
            let shared_clients =
                clients::Clients::from_existing(github.clone(), client.http.clone());

            // Spawn sync task if enabled
            let sync_config_clone = config.clone();
            tokio::spawn(async move {
                let syncer = sync::IssueSyncer::new(
                    sync_config_clone,
                    shared_clients.github,
                    shared_clients.discord_http,
                );
                syncer.start().await;
            });

            // Start the bot
            tracing::info!("Starting CardiBot...");
            client.start().await?;
        }
    }

    Ok(())
}
