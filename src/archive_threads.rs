use anyhow::Result;
use serenity::http::Http;
use serenity::model::id::{ChannelId, GuildId};
use std::sync::Arc;
use tracing::info;

use crate::config::Config;

pub async fn archive_locked_threads() -> Result<()> {
    println!("🧹 Archiving locked threads with configured prefixes...\n");

    // Load configuration
    let config = Config::load()?;
    
    println!("Thread prefixes to check: {:?}", crate::constants::THREAD_PREFIXES);
    println!();

    // Initialize Discord client
    let token = std::env::var("DISCORD_TOKEN")?;
    let discord = Arc::new(Http::new(&token));

    // Process each project
    for (idx, project) in config.projects.iter().enumerate() {
        println!("Project {}: {}", idx + 1, project.name.as_deref().unwrap_or("unnamed"));
        println!("  - Discord Guild: {}", project.discord_guild_id);
        println!("  - Discord Forum: {}", project.discord_forum_id);
        
        match archive_project_threads(&discord, project).await {
            Ok(count) => {
                println!("  ✅ Archived {} locked threads", count);
            }
            Err(e) => {
                eprintln!("  ❌ Error: {}", e);
            }
        }
        println!();
    }

    Ok(())
}

async fn archive_project_threads(
    discord: &Http,
    project: &crate::config::Project,
) -> Result<usize> {
    let guild_id = GuildId::new(project.discord_guild_id.parse()?);
    let forum_id = ChannelId::new(project.discord_forum_id.parse()?);

    // Get all active threads
    let threads = guild_id.get_active_threads(discord).await?;
    let mut archived_count = 0;

    for thread in threads.threads {
        // Only process threads in our forum
        if thread.parent_id != Some(forum_id) {
            continue;
        }

        // Check if thread has valid prefix
        let thread_name = &thread.name;
        let has_valid_prefix = crate::constants::THREAD_PREFIXES.iter()
            .any(|prefix| thread_name.starts_with(prefix));
        
        if !has_valid_prefix {
            continue;
        }

        // Check if thread is locked but not archived
        let metadata = thread.thread_metadata.as_ref();
        let is_locked = metadata.map(|m| m.locked).unwrap_or(false);
        let is_archived = metadata.map(|m| m.archived).unwrap_or(false);

        if is_locked && !is_archived {
            println!("  - Archiving locked thread: {} ({})", thread_name, thread.id);
            
            // Archive the thread
            thread.id
                .edit_thread(discord, serenity::builder::EditThread::new()
                    .archived(true))
                .await?;

            archived_count += 1;
            info!("Archived locked thread {} ({})", thread.id, thread_name);
        }
    }

    Ok(archived_count)
}