use anyhow::Result;
use octocrab::Octocrab;
use serenity::http::Http;
use std::sync::Arc;

/// Shared client management for Discord and GitHub
pub struct Clients {
    pub github: Arc<Octocrab>,
    pub discord_http: Arc<Http>,
}

impl Clients {
    /// Create clients for use outside of the main bot (e.g., CLI commands)
    pub async fn new_standalone() -> Result<Self> {
        // Ensure environment variables are loaded
        dotenv::dotenv().ok();

        let github = Arc::new(crate::github_app::create_github_client().await?);

        let discord_token = std::env::var("DISCORD_TOKEN")?;
        let discord_http = Arc::new(Http::new(&discord_token));

        Ok(Self {
            github,
            discord_http,
        })
    }
}
