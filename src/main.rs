mod config;
mod bot;
mod commands;
mod github;
mod cli;
mod debug;

use std::sync::Arc;
use anyhow::Result;
use clap::Parser;
use serenity::prelude::*;

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
            println!("Posting feedback instructions to channel {}...", channel);
            debug::post_feedback_instructions(&channel).await?;
        }
        cli::Commands::ValidateConfig => {
            println!("Validating configuration...");
            match config::Config::load() {
                Ok(config) => {
                    println!("✓ Configuration is valid!");
                    println!("  - Log level: {}", config.log_level.as_deref().unwrap_or("info"));
                    println!("  - Projects configured: {}", config.projects.len());
                    for (i, project) in config.projects.iter().enumerate() {
                        println!("\n  Project {}:", i + 1);
                        println!("    - Name: {}", project.name.as_deref().unwrap_or("(unnamed)"));
                        println!("    - Discord Guild: {}", project.discord_guild_id);
                        println!("    - Discord Forum: {}", project.discord_forum_id);
                        println!("    - GitHub: {}/{}", project.github_owner, project.github_repo);
                        if let Some(role_id) = &project.allowed_role_id {
                            println!("    - Required Role ID: {}", role_id);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("✗ Configuration error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        cli::Commands::Run => {
            // Load configuration first to get log level
            let config = Arc::new(config::Config::load()?);
            
            // Initialize logging with configured level
            let log_level = config.log_level.as_deref().unwrap_or("info");
            use tracing_subscriber::EnvFilter;
            tracing_subscriber::fmt()
                .with_env_filter(EnvFilter::new(log_level))
                .init();
            
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
        }
    }
    
    Ok(())
}