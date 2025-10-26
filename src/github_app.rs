use anyhow::{Context, Result};
use jsonwebtoken::EncodingKey;
use octocrab::models::InstallationId;
use octocrab::Octocrab;
use std::fs;

pub struct GitHubApp {
    app_id: u64,
    private_key: EncodingKey,
    installation_id: InstallationId,
}

impl GitHubApp {
    pub fn new(app_id: String, private_key_path: String, installation_id: u64) -> Result<Self> {
        let private_key_pem = fs::read_to_string(&private_key_path)
            .with_context(|| format!("Failed to read private key from {private_key_path}"))?;

        let encoding_key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
            .context("Failed to parse RSA private key")?;

        let app_id = app_id
            .parse::<u64>()
            .context("Failed to parse app_id as u64")?;

        Ok(Self {
            app_id,
            private_key: encoding_key,
            installation_id: installation_id.into(),
        })
    }

    /// Creates an Octocrab instance with GitHub App authentication and automatic token caching
    pub async fn create_installation_client(&self) -> Result<Octocrab> {
        // Create app-authenticated client
        let app_client = Octocrab::builder()
            .app(self.app_id.into(), self.private_key.clone())
            .build()
            .context("Failed to create GitHub App client")?;

        // Get installation client with automatic token caching
        let (installation_client, _token) = app_client
            .installation_and_token(self.installation_id)
            .await
            .context("Failed to get installation client with token")?;

        Ok(installation_client)
    }
}

// Helper function to create either GitHub App or PAT authenticated client
// Returns a client with automatic token caching (for GitHub Apps) built into octocrab
pub async fn create_github_client() -> Result<Octocrab> {
    // Check if GitHub App credentials are available
    if let (Ok(app_id), Ok(installation_id)) = (
        std::env::var("GITHUB_APP_ID"),
        std::env::var("GITHUB_APP_INSTALLATION_ID"),
    ) {
        if let Ok(private_key_path) = std::env::var("GITHUB_APP_PRIVATE_KEY_PATH") {
            let installation_id = installation_id
                .parse()
                .context("Invalid GITHUB_APP_INSTALLATION_ID")?;

            tracing::info!(
                "Using GitHub App authentication with automatic token caching (App ID: {}, Installation: {})",
                app_id,
                installation_id
            );
            let app = GitHubApp::new(app_id, private_key_path, installation_id)?;
            return app.create_installation_client().await;
        }
    }

    // Fall back to PAT authentication
    let github_token = std::env::var("GITHUB_TOKEN")
        .context("GITHUB_TOKEN not set and GitHub App credentials not configured")?;

    tracing::info!("Using GitHub PAT authentication");
    Octocrab::builder()
        .personal_token(github_token)
        .build()
        .context("Failed to create Octocrab instance with PAT")
}
