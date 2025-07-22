use anyhow::Result;
use octocrab::Octocrab;
use regex::Regex;
use serenity::http::Http;
use serenity::model::channel::ChannelType;
use serenity::model::id::{ChannelId, GuildId};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info, warn};

use crate::config::{Config, Project};

pub struct IssueSyncer {
    config: Arc<Config>,
    github: Arc<Octocrab>,
    discord: Arc<Http>,
}

impl IssueSyncer {
    pub fn new(config: Arc<Config>, github: Arc<Octocrab>, discord: Arc<Http>) -> Self {
        Self {
            config,
            github,
            discord,
        }
    }

    pub async fn start(self) {
        let sync_config = self.config.sync_config();
        
        if !sync_config.enabled {
            info!("Issue sync is disabled in configuration");
            return;
        }

        info!(
            "Starting issue sync with interval: {} seconds",
            sync_config.interval_seconds
        );

        let mut interval = interval(Duration::from_secs(sync_config.interval_seconds));
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.sync_all_projects().await {
                error!("Error during sync cycle: {}", e);
            }
        }
    }

    async fn sync_all_projects(&self) -> Result<()> {
        info!("Starting sync cycle for {} projects", self.config.projects.len());
        
        for project in &self.config.projects {
            if let Err(e) = self.sync_project(project).await {
                error!(
                    "Error syncing project {}: {}",
                    project.name.as_deref().unwrap_or("unnamed"),
                    e
                );
            }
        }
        Ok(())
    }

    async fn sync_project(&self, project: &Project) -> Result<()> {
        info!(
            "Syncing project: {}",
            project.name.as_deref().unwrap_or("unnamed")
        );

        // Search for all open issues with thread IDs
        let open_issues = self
            .search_issues(&project.github_owner, &project.github_repo, "open")
            .await?;
        
        info!("Found {} open issues with thread IDs", open_issues.len());

        // Build a set of open issue thread IDs for quick lookup
        let open_thread_ids: HashSet<u64> = open_issues
            .iter()
            .filter_map(|issue| extract_thread_id(&issue.title))
            .collect();

        // Count how many threads exist
        let mut existing_threads = 0;
        let mut missing_threads = 0;

        // Sync open issues (ensure threads are unlocked)
        for issue in &open_issues {
            if let Some(thread_id) = extract_thread_id(&issue.title) {
                match self.sync_open_issue(project, thread_id, issue).await {
                    Ok(true) => existing_threads += 1,
                    Ok(false) => missing_threads += 1,
                    Err(e) => {
                        warn!("Failed to sync open issue #{}: {}", issue.number, e);
                        missing_threads += 1;
                    }
                }
            }
        }
        
        info!(
            "Discord thread status: {}/{} exist ({} missing)", 
            existing_threads, 
            open_issues.len(),
            missing_threads
        );

        // Check all Discord threads in the forum
        if let Err(e) = self.sync_discord_threads(project, &open_thread_ids).await {
            warn!("Failed to sync Discord threads: {}", e);
        }

        Ok(())
    }

    async fn search_issues(
        &self,
        owner: &str,
        repo: &str,
        state: &str,
    ) -> Result<Vec<octocrab::models::issues::Issue>> {
        // Search for issues with thread IDs in square brackets like [1234567890]
        // We need to search for all issues and filter client-side since GitHub search
        // doesn't support regex patterns for numbers in brackets
        let query = format!("repo:{}/{} is:{} in:title", owner, repo, state);
        
        let page = self
            .github
            .search()
            .issues_and_pull_requests(&query)
            .send()
            .await?;

        // Filter to only issues with thread IDs
        let issues_with_thread_ids: Vec<_> = page.items
            .into_iter()
            .filter(|issue| extract_thread_id(&issue.title).is_some())
            .collect();

        Ok(issues_with_thread_ids)
    }

    async fn sync_open_issue(
        &self,
        project: &Project,
        thread_id: u64,
        issue: &octocrab::models::issues::Issue,
    ) -> Result<bool> {
        let channel_id = ChannelId::new(thread_id);
        let _guild_id = GuildId::new(project.discord_guild_id.parse()?);

        // Get thread info
        match self.discord.get_channel(channel_id).await {
            Ok(channel) => {
                if let Some(thread) = channel.guild() {
                    if thread.kind == ChannelType::PublicThread {
                        // Check if thread is locked or archived
                        let metadata = thread.thread_metadata.as_ref();
                        let is_locked = metadata.map(|m| m.locked).unwrap_or(false);
                        let is_archived = metadata.map(|m| m.archived).unwrap_or(false);
                        
                        if is_locked || is_archived {
                            // Post update message first (before unlocking)
                            channel_id
                                .send_message(&self.discord, serenity::builder::CreateMessage::new()
                                    .content(crate::constants::MSG_ISSUE_REOPENED))
                                .await?;
                            
                            // Unlock and unarchive the thread
                            channel_id
                                .edit_thread(&self.discord, serenity::builder::EditThread::new()
                                    .locked(false)
                                    .archived(false))
                                .await?;

                            info!("Unlocked and unarchived thread {} for reopened issue #{}", thread_id, issue.number);
                        }
                    }
                }
                return Ok(true); // Thread exists
            }
            Err(e) => {
                warn!(
                    "Thread {} not found: {} - GitHub issue: https://github.com/{}/{}/issues/{}", 
                    thread_id, 
                    e,
                    project.github_owner,
                    project.github_repo,
                    issue.number
                );
                Ok(false) // Thread doesn't exist
            }
        }
    }

    async fn sync_discord_threads(
        &self,
        project: &Project,
        open_thread_ids: &HashSet<u64>,
    ) -> Result<()> {
        let guild_id = GuildId::new(project.discord_guild_id.parse()?);
        let forum_id = ChannelId::new(project.discord_forum_id.parse()?);

        // Get all active threads in the guild
        let active_threads = guild_id
            .get_active_threads(&self.discord)
            .await?;

        // Process active threads to find ones that might need to be locked
        for thread in active_threads.threads {
            // Only process threads in our forum
            if thread.parent_id != Some(forum_id) {
                continue;
            }
            
            // Only check threads with valid prefixes
            let thread_name = &thread.name;
            let has_valid_prefix = crate::constants::THREAD_PREFIXES.iter()
                .any(|prefix| thread_name.starts_with(prefix));
            
            if !has_valid_prefix {
                continue;
            }
            
            // Skip already archived/locked threads
            let metadata = thread.thread_metadata.as_ref();
            let is_archived = metadata.map(|m| m.archived).unwrap_or(false);
            let is_locked = metadata.map(|m| m.locked).unwrap_or(false);
            
            if is_archived || is_locked {
                continue;
            }

            let thread_id = thread.id.get();
            
            // If this thread has an open issue, skip it (it should stay unlocked)
            if open_thread_ids.contains(&thread_id) {
                continue;
            }
            
            info!("Checking thread {} ({}) for closure", thread_id, thread_name);

            // Check if CardiBot created an issue for this thread
            let messages = thread.id
                .messages(&self.discord, serenity::builder::GetMessages::new().limit(crate::constants::DISCORD_MESSAGE_FETCH_LIMIT))
                .await?;
            
            // Look for CardiBot's issue creation message (in embeds)
            let mut github_issue_url = None;
            for msg in &messages {
                if msg.author.bot {
                    for embed in &msg.embeds {
                        if embed.title.as_deref() == Some(crate::constants::MSG_ISSUE_CREATED) || 
                           embed.title.as_deref() == Some(crate::constants::MSG_ISSUE_UPDATED) {
                            // Extract issue URL from embed description
                            if let Some(desc) = &embed.description {
                                if let Some(url_start) = desc.find("https://github.com/") {
                                    let url_part = &desc[url_start..];
                                    if let Some(url_end) = url_part.find(|c: char| c.is_whitespace()) {
                                        github_issue_url = Some(url_part[..url_end].to_string());
                                    } else {
                                        github_issue_url = Some(url_part.to_string());
                                    }
                                    info!("Found GitHub issue URL in thread {}: {}", thread_id, github_issue_url.as_ref().unwrap());
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            
            if let Some(issue_url) = github_issue_url {
                // Extract issue number from URL
                if let Some(issue_num_str) = issue_url.split('/').last() {
                    if let Ok(issue_number) = issue_num_str.parse::<u64>() {
                        // Check if this issue is still open
                        match self.github
                            .issues(&project.github_owner, &project.github_repo)
                            .get(issue_number)
                            .await 
                        {
                            Ok(issue) => {
                                if matches!(issue.state, octocrab::models::IssueState::Closed) {
                                    info!("Thread {} has closed issue #{}, archiving", thread_id, issue_number);
                                    
                                    // Post closure message
                                    thread.id
                                        .send_message(&self.discord, serenity::builder::CreateMessage::new()
                                            .content(crate::constants::MSG_ISSUE_CLOSED))
                                        .await?;

                                    // Lock and archive the thread
                                    thread.id
                                        .edit_thread(&self.discord, serenity::builder::EditThread::new()
                                            .locked(true)
                                            .archived(true))
                                        .await?;

                                    info!("Locked and archived thread {} - issue #{} is closed", thread_id, issue_number);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to check issue status for thread {}: {}", thread_id, e);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

pub fn extract_thread_id(title: &str) -> Option<u64> {
    // Extract thread ID from title format: "Title [1234567890]"
    let re = Regex::new(r"\[(\d+)\]").ok()?;
    re.captures(title)?
        .get(1)?
        .as_str()
        .parse::<u64>()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_thread_id() {
        assert_eq!(
            extract_thread_id("Bug with login [1234567890]"),
            Some(1234567890)
        );
        assert_eq!(
            extract_thread_id("Feature request [9876543210]"),
            Some(9876543210)
        );
        assert_eq!(extract_thread_id("No thread ID here"), None);
        assert_eq!(extract_thread_id("[not-a-number]"), None);
    }
}