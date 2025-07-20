use serenity::{all::*, async_trait, model::gateway::Ready};
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
        let commands = vec![crate::commands::create_issue_command()];

        for guild in &ready.guilds {
            let commands_builder = guild.id.set_commands(&ctx.http, commands.clone()).await;

            if let Err(e) = commands_builder {
                tracing::error!("Failed to register commands for guild {}: {}", guild.id, e);
            } else {
                tracing::info!("Registered commands for guild {}", guild.id);
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            match command.data.name.as_str() {
                "issue" => {
                    if let Err(e) =
                        crate::commands::handle_issue_command(&ctx, &command, &self).await
                    {
                        tracing::error!("Error handling command: {}", e);
                    }
                }
                _ => {}
            }
        }
    }
}
