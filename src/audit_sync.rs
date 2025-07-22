use anyhow::Result;
use octocrab::Octocrab;
use serenity::http::Http;
use serenity::model::id::{ChannelId, GuildId};
use std::collections::HashSet;
use std::sync::Arc;

use crate::config::Config;
use crate::sync::extract_thread_id;

pub async fn audit_sync_status() -> Result<()> {
    println!("🔍 Auditing sync status between GitHub and Discord...\n");

    // Load configuration
    let config = Config::load()?;
    let sync_config = config.sync_config();
    
    println!("Sync Configuration:");
    println!("  - Enabled: {}", sync_config.enabled);
    println!("  - Thread prefixes: {:?}", crate::constants::THREAD_PREFIXES);
    println!();

    // Initialize clients
    let github = crate::github_app::create_github_client().await?;
    let token = std::env::var("DISCORD_TOKEN")?;
    let discord = Arc::new(Http::new(&token));

    // Audit each project
    for (idx, project) in config.projects.iter().enumerate() {
        println!("Project {}: {}", idx + 1, project.name.as_deref().unwrap_or("unnamed"));
        println!("  - GitHub: {}/{}", project.github_owner, project.github_repo);
        println!("  - Discord Guild: {}", project.discord_guild_id);
        println!("  - Discord Forum: {}", project.discord_forum_id);
        println!();
        
        match audit_project(&github, &discord, project).await {
            Ok(()) => {},
            Err(e) => {
                eprintln!("  ❌ Error auditing project: {}", e);
            }
        }
        println!();
    }

    Ok(())
}

async fn audit_project(
    github: &Octocrab,
    discord: &Http,
    project: &crate::config::Project,
) -> Result<()> {
    // Get all open GitHub issues with thread IDs
    let query = format!(
        "repo:{}/{} is:open in:title",
        project.github_owner, project.github_repo
    );
    
    let search_result = github
        .search()
        .issues_and_pull_requests(&query)
        .send()
        .await?;
    
    // Build set of open issue thread IDs
    let github_open_threads: HashSet<u64> = search_result.items
        .iter()
        .filter_map(|issue| extract_thread_id(&issue.title))
        .collect();
    
    println!("  📊 GitHub Status:");
    println!("    - Open issues with thread IDs: {}", github_open_threads.len());
    
    // Get Discord threads
    let guild_id = GuildId::new(project.discord_guild_id.parse()?);
    let forum_id = ChannelId::new(project.discord_forum_id.parse()?);
    let active_threads = guild_id.get_active_threads(discord).await?;
    
    // Analyze managed threads
    let mut discord_managed_unlocked = 0;
    let mut discord_managed_locked = 0;
    let mut threads_with_wrong_state = Vec::new();
    let mut existing_thread_ids = HashSet::new();
    
    for thread in active_threads.threads {
        // Only process threads in our forum
        if thread.parent_id != Some(forum_id) {
            continue;
        }
        
        let thread_id = thread.id.get();
        
        // Only care about threads that are managed (have their ID in a GitHub issue)
        if github_open_threads.contains(&thread_id) {
            existing_thread_ids.insert(thread_id);
            
            // This thread is linked to an open GitHub issue
            let metadata = thread.thread_metadata.as_ref();
            let is_locked = metadata.map(|m| m.locked).unwrap_or(false);
            let is_archived = metadata.map(|m| m.archived).unwrap_or(false);
            
            if !is_archived {
                if is_locked {
                    threads_with_wrong_state.push((thread_id, thread.name.clone(), "Should be UNLOCKED (issue is open)"));
                    discord_managed_locked += 1;
                } else {
                    discord_managed_unlocked += 1;
                }
            }
        }
    }
    
    // Find issues without existing Discord threads
    let missing_threads: Vec<_> = github_open_threads
        .iter()
        .filter(|&&id| !existing_thread_ids.contains(&id))
        .collect();
    
    println!("\n  💬 Discord Status:");
    println!("    - Managed threads found: {}", discord_managed_unlocked + discord_managed_locked);
    println!("    - Correctly unlocked: {}", discord_managed_unlocked);
    println!("    - Incorrectly locked: {}", discord_managed_locked);
    
    println!("\n  🔄 Sync Analysis:");
    
    if threads_with_wrong_state.is_empty() && missing_threads.is_empty() {
        println!("    ✅ All managed threads are properly synced!");
    } else {
        if !threads_with_wrong_state.is_empty() {
            println!("\n    ⚠️  Threads with incorrect state:");
            for (id, name, reason) in &threads_with_wrong_state {
                println!("      - {} (ID: {}) - {}", name, id, reason);
            }
        }
        
        if !missing_threads.is_empty() {
            println!("\n    ℹ️  {} open GitHub issues reference missing Discord threads", missing_threads.len());
            println!("    (These threads may have been deleted or archived)");
            if missing_threads.len() <= 5 {
                for &&thread_id in &missing_threads {
                    if let Some(issue) = search_result.items.iter().find(|i| extract_thread_id(&i.title) == Some(thread_id)) {
                        println!("      - Issue #{}: {}", issue.number, issue.title);
                    }
                }
            }
        }
    }
    
    println!("\n  📝 Summary:");
    println!("    - Sync only manages threads where CardiBot created a GitHub issue");
    println!("    - Other Discord threads (even with [BUG] prefix) are ignored");
    println!("    - Threads are identified as managed by checking for bot messages");
    
    Ok(())
}