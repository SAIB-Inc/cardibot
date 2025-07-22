use anyhow::Result;
use octocrab::Octocrab;
use crate::config::Config;
use crate::sync::extract_thread_id;

pub async fn debug_sync_status() -> Result<()> {
    println!("🔍 Debugging sync status...\n");

    // Load configuration
    let config = Config::load()?;
    let sync_config = config.sync_config();
    
    println!("Sync Configuration:");
    println!("  - Enabled: {}", sync_config.enabled);
    println!("  - Interval: {} seconds", sync_config.interval_seconds);
    println!("  - Thread prefixes: {:?}", sync_config.thread_prefixes);
    println!();

    // Initialize GitHub client
    let github = crate::github_app::create_github_client().await?;

    // Check each project
    for (idx, project) in config.projects.iter().enumerate() {
        println!("Project {}: {}", idx + 1, project.name.as_deref().unwrap_or("unnamed"));
        println!("  - GitHub: {}/{}", project.github_owner, project.github_repo);
        println!("  - Discord Guild: {}", project.discord_guild_id);
        println!("  - Discord Forum: {}", project.discord_forum_id);
        
        // Search for issues with thread IDs
        match debug_project_sync(&github, project).await {
            Ok(()) => {},
            Err(e) => {
                eprintln!("  ❌ Error checking project: {}", e);
            }
        }
        println!();
    }

    Ok(())
}

async fn debug_project_sync(github: &Octocrab, project: &crate::config::Project) -> Result<()> {
    // Search for all issues with thread IDs (both open and closed)
    // Search for all issues to check which ones have thread IDs
    let query = format!(
        "repo:{}/{} in:title",
        project.github_owner, project.github_repo
    );
    
    println!("  - Search query: {}", query);
    
    let search_result = github
        .search()
        .issues_and_pull_requests(&query)
        .send()
        .await?;
    
    println!("  - Total issues found: {}", search_result.items.len());
    
    // Filter issues with thread IDs
    let issues_with_thread_ids: Vec<_> = search_result.items
        .iter()
        .filter(|issue| extract_thread_id(&issue.title).is_some())
        .collect();
    
    println!("  - Issues with thread IDs: {}", issues_with_thread_ids.len());
    
    if issues_with_thread_ids.is_empty() {
        // Show first few issues to help debug
        if !search_result.items.is_empty() {
            println!("  - Sample issue titles (first 5):");
            for (i, issue) in search_result.items.iter().take(5).enumerate() {
                println!("    {}. \"{}\"", i + 1, issue.title);
            }
        }
        return Ok(());
    }
    
    // List issues with their states
    for issue in issues_with_thread_ids.iter().take(10) {
        if let Some(thread_id) = extract_thread_id(&issue.title) {
            println!(
                "    • Issue #{} [{}] - Thread ID: {} - State: {}",
                issue.number,
                issue.title,
                thread_id,
                format!("{:?}", issue.state)
            );
        }
    }
    
    if search_result.items.len() > 10 {
        println!("    ... and {} more", search_result.items.len() - 10);
    }
    
    // Count open issues with thread IDs
    let open_count = search_result.items
        .into_iter()
        .filter(|issue| extract_thread_id(&issue.title).is_some())
        .filter(|i| matches!(i.state, octocrab::models::IssueState::Open))
        .count();
    
    println!("  - Open issues with thread IDs: {}", open_count);
    println!("  - (Sync only tracks open issues for efficiency)");
    
    Ok(())
}