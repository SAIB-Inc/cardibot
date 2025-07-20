use serenity::{
    async_trait,
    model::gateway::Ready,
    model::channel::ChannelType,
    all::*,
    http::Http,
    prelude::{Context, EventHandler, GatewayIntents},
};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct DebugHandler {
    pub completed: Arc<Mutex<bool>>,
}

#[async_trait]
impl EventHandler for DebugHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("\n=== DISCORD SERVER INFORMATION ===\n");
        println!("Bot connected as: {}", ready.user.name);
        println!("Bot ID: {}", ready.user.id);
        println!();
        
        for guild in &ready.guilds {
            if let Ok(partial_guild) = guild.id.to_partial_guild(&ctx).await {
                println!("Server: {}", partial_guild.name);
                println!("Server ID: {}", guild.id);
                println!();
                
                // Get all channels
                if let Ok(channels) = guild.id.channels(&ctx).await {
                    let mut forum_channels = Vec::new();
                    
                    for (channel_id, channel) in channels {
                        if channel.kind == ChannelType::Forum {
                            forum_channels.push((channel_id, channel.name.clone()));
                        }
                    }
                    
                    if !forum_channels.is_empty() {
                        println!("Forum Channels:");
                        for (id, name) in forum_channels {
                            println!("  - {} (ID: {})", name, id);
                            if name.to_lowercase().contains("levvy") {
                                println!("    ^ This looks like your Levvy forum!");
                            }
                        }
                        println!();
                    }
                }
                
                // Show roles
                println!("Server Roles:");
                for (role_id, role) in &partial_guild.roles {
                    if role.name != "@everyone" {
                        println!("  - {} (ID: {})", role.name, role_id);
                    }
                }
                println!();
                println!("------------------------");
            }
        }
        
        println!("\n=== CONFIGURATION EXAMPLE ===\n");
        println!("Add this to your config.toml:");
        println!();
        println!("[[projects]]");
        println!("name = \"Your Project Name\"");
        println!("discord_guild_id = \"{}\"", ready.guilds[0].id);
        println!("discord_forum_id = \"YOUR_FORUM_ID_FROM_ABOVE\"");
        println!("github_owner = \"your-github-org\"");
        println!("github_repo = \"your-repo-name\"");
        println!("# allowed_role_id = \"YOUR_ROLE_ID_FROM_ABOVE\"  # Optional: restrict who can create issues");
        println!();
        
        // Signal completion and shut down
        *self.completed.lock().await = true;
        ctx.shard.shutdown_clean();
    }
}

pub async fn check_discord() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    
    let discord_token = std::env::var("DISCORD_TOKEN")?;
    let intents = GatewayIntents::GUILDS;
    
    let completed = Arc::new(Mutex::new(false));
    let handler = DebugHandler {
        completed: completed.clone(),
    };
    
    let mut client = Client::builder(&discord_token, intents)
        .event_handler(handler)
        .await?;
    
    // Start the client
    tokio::spawn(async move {
        if let Err(e) = client.start().await {
            eprintln!("Client error: {:?}", e);
        }
    });
    
    // Wait for completion
    while !*completed.lock().await {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    Ok(())
}

pub async fn post_feedback_instructions(channel_id: &str) -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    
    let discord_token = std::env::var("DISCORD_TOKEN")?;
    let channel_id = ChannelId::new(channel_id.parse::<u64>()?);
    
    // Create a minimal bot to send the message
    let http = Http::new(&discord_token);
    
    // Check if it's a forum channel
    let channel = channel_id.to_channel(&http).await?;
    
    match channel {
        Channel::Guild(guild_channel) if guild_channel.kind == ChannelType::Forum => {
            // For forum channels, create a new thread
            println!("Creating a new thread in forum channel...");
            
            let embed = CreateEmbed::new()
                .title("🚀 Welcome to Levvy V3 Testnet!")
                .description("We need your feedback to make Levvy even better.")
                .color(0x00ADB5)
                .field(
                    "📋 How to Provide Feedback",
                    "1. **Test the Platform**: Visit https://v3.levvy.fi/\n\
                     2. **Try Different Features**: Loans, borrows, position management\n\
                     3. **Report Issues**: Create a new forum thread with these prefixes in your title:\n\
                        • `[BUG] Your title here` - Something isn't working\n\
                        • `[FEEDBACK] Your title` - General suggestions\n\
                        • `[FEATURE] Your title` - New feature requests\n\
                        • `[QUESTION] Your title` - Need help or clarification\n\
                     4. **Be Specific**: Include steps to reproduce, screenshots if applicable",
                    false
                )
                .field(
                    "✅ What's Ready to Test",
                    "• Classic loans / borrows\n\
                     • Perpetual loans / borrows\n\
                     • Manage positions (open/close, adjust collateral)\n\
                     • Simplified \"New Levvy\" loan flow",
                    false
                )
                .field(
                    "🚧 Coming Soon",
                    "• Position history (toggle off for now)\n\
                     • Levvy Reaper liquidator bot (public release soon)\n\
                     • NFT loan/borrow capability\n\
                     • Stats & analytics page",
                    false
                )
                .field(
                    "💰 Getting Test Funds",
                    "• **Test ADA**: [Cardano Faucet](https://docs.cardano.org/cardano-testnets/tools/faucet)\n\
                     • **Test Tokens**: Use the faucet button at bottom-right of Levvy UI",
                    false
                )
                .field(
                    "⚠️ Important",
                    "**MAKE SURE YOUR WALLET IS SET TO CARDANO PREVIEW TESTNET**\n\
                     Use Preview testnet settings, not Pre-prod",
                    false
                )
                .footer(CreateEmbedFooter::new("Thank you for helping us test Levvy V3!"));
            
            // Create a forum thread
            let message = CreateMessage::new()
                .content("Please read the instructions below to provide feedback on Levvy V3 Testnet:")
                .embed(embed);
                
            let thread = CreateForumPost::new("📢 Levvy V3 Testnet Feedback Instructions", message);
            
            channel_id.create_forum_post(&http, thread).await?;
            println!("✅ Feedback thread created in forum {}", channel_id);
        }
        _ => {
            // For regular channels, just send a message
            let embed = CreateEmbed::new()
                .title("🚀 Levvy V3 Testnet Feedback Instructions")
                .description("Welcome to the Levvy V3 Testnet! We need your feedback to make Levvy even better.")
                .color(0x00ADB5)
                .field(
                    "📋 How to Provide Feedback",
                    "1. **Test the Platform**: Visit https://v3.levvy.fi/\n\
                     2. **Try Different Features**: Loans, borrows, position management\n\
                     3. **Report Issues**: Create a new forum thread with these prefixes in your title:\n\
                        • `[BUG] Your title here` - Something isn't working\n\
                        • `[FEEDBACK] Your title` - General suggestions\n\
                        • `[FEATURE] Your title` - New feature requests\n\
                        • `[QUESTION] Your title` - Need help or clarification\n\
                     4. **Be Specific**: Include steps to reproduce, screenshots if applicable",
                    false
                )
                .field(
                    "✅ What's Ready to Test",
                    "• Classic loans / borrows\n\
                     • Perpetual loans / borrows\n\
                     • Manage positions (open/close, adjust collateral)\n\
                     • Simplified \"New Levvy\" loan flow",
                    false
                )
                .field(
                    "🚧 Coming Soon",
                    "• Position history (toggle off for now)\n\
                     • Levvy Reaper liquidator bot (public release soon)\n\
                     • NFT loan/borrow capability\n\
                     • Stats & analytics page",
                    false
                )
                .field(
                    "💰 Getting Test Funds",
                    "• **Test ADA**: [Cardano Faucet](https://docs.cardano.org/cardano-testnets/tools/faucet)\n\
                     • **Test Tokens**: Use the faucet button at bottom-right of Levvy UI",
                    false
                )
                .field(
                    "⚠️ Important",
                    "**MAKE SURE YOUR WALLET IS SET TO CARDANO PREVIEW TESTNET**\n\
                     Use Preview testnet settings, not Pre-prod",
                    false
                )
                .footer(CreateEmbedFooter::new("Thank you for helping us test Levvy V3!"));
            
            let message = CreateMessage::new().embed(embed);
            channel_id.send_message(&http, message).await?;
            println!("✅ Feedback instructions posted to channel {}", channel_id);
        }
    }
    
    Ok(())
}