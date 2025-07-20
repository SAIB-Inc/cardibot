use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cardibot")]
#[command(about = "Discord-GitHub feedback bridge bot", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the bot normally
    Run,

    /// Check Discord server information and exit
    CheckDiscord,

    /// Validate configuration file
    ValidateConfig,

    /// Post feedback instructions to a Discord channel
    PostFeedback {
        /// Channel ID where to post the feedback instructions
        #[arg(long)]
        channel: String,
    },
}
