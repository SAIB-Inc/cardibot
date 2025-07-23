use anyhow::Result;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub log_level: Option<String>,
    pub projects: Vec<Project>,
    pub sync: Option<SyncConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SyncConfig {
    #[serde(default = "default_sync_enabled")]
    pub enabled: bool,
    #[serde(default = "default_sync_interval")]
    pub interval_seconds: u64,
}

fn default_sync_enabled() -> bool {
    true
}

fn default_sync_interval() -> u64 {
    60 // 1 minute instead of 10 seconds to avoid rate limits
}

#[derive(Debug, Deserialize, Clone)]
pub struct Project {
    pub name: Option<String>,
    pub discord_guild_id: String,
    pub discord_forum_id: String,
    pub github_owner: String,
    pub github_repo: String,
    pub allowed_role_id: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let contents = fs::read_to_string(crate::constants::DEFAULT_CONFIG_PATH)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn find_project(&self, guild_id: u64, channel_id: u64) -> Option<&Project> {
        self.projects.iter().find(|p| {
            p.discord_guild_id == guild_id.to_string()
                && p.discord_forum_id == channel_id.to_string()
        })
    }

    pub fn sync_config(&self) -> SyncConfig {
        self.sync.clone().unwrap_or(SyncConfig {
            enabled: default_sync_enabled(),
            interval_seconds: default_sync_interval(),
        })
    }
}
