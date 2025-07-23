use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use octocrab::Octocrab;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iat: i64,
    exp: i64,
    iss: String,
}

#[derive(Debug, Deserialize)]
struct InstallationToken {
    token: String,
}

pub struct GitHubApp {
    app_id: String,
    private_key: String,
    installation_id: u64,
}

impl GitHubApp {
    pub fn new(app_id: String, private_key_path: String, installation_id: u64) -> Result<Self> {
        let private_key = fs::read_to_string(&private_key_path)
            .with_context(|| format!("Failed to read private key from {private_key_path}"))?;

        Ok(Self {
            app_id,
            private_key,
            installation_id,
        })
    }

    fn generate_jwt(&self) -> Result<String> {
        let now = Utc::now();
        let claims = Claims {
            iat: (now - Duration::seconds(60)).timestamp(),
            exp: (now + Duration::minutes(10)).timestamp(),
            iss: self.app_id.clone(),
        };

        let header = Header::new(Algorithm::RS256);
        let encoding_key = EncodingKey::from_rsa_pem(self.private_key.as_bytes())?;

        encode(&header, &claims, &encoding_key).context("Failed to encode JWT")
    }

    pub async fn get_installation_token(&self) -> Result<String> {
        let jwt = self.generate_jwt()?;

        let client = reqwest::Client::new();
        let response = client
            .post(format!(
                "https://api.github.com/app/installations/{}/access_tokens",
                self.installation_id
            ))
            .header("Authorization", format!("Bearer {jwt}"))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "CardiBot")
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            anyhow::bail!("Failed to get installation token: {} - {}", status, text);
        }

        let token_response: InstallationToken = response.json().await?;
        Ok(token_response.token)
    }

    pub async fn create_octocrab_instance(&self) -> Result<Octocrab> {
        let token = self.get_installation_token().await?;

        Octocrab::builder()
            .personal_token(token)
            .build()
            .context("Failed to create Octocrab instance")
    }
}

// Helper function to create either GitHub App or PAT authenticated client
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
                "Using GitHub App authentication (App ID: {}, Installation: {})",
                app_id,
                installation_id
            );
            let app = GitHubApp::new(app_id, private_key_path, installation_id)?;
            return app.create_octocrab_instance().await;
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
