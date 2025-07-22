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
    #[serde(default = "default_sync_prefixes")]
    pub thread_prefixes: Vec<String>,
}

fn default_sync_enabled() -> bool {
    true
}

fn default_sync_interval() -> u64 {
    10
}

fn default_sync_prefixes() -> Vec<String> {
    vec!["[BUG]".to_string(), "[FEATURE]".to_string(), "[QUESTION]".to_string()]
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
        let contents = fs::read_to_string("config.toml")?;
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
            thread_prefixes: default_sync_prefixes(),
        })
    }
}
