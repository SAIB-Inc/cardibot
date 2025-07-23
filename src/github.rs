use crate::config::Project;
use anyhow::Result;
use octocrab::models::issues::Issue;
use serenity::builder::GetMessages;
use serenity::model::channel::GuildChannel;

pub struct IssueResult {
    pub issue: Issue,
    pub was_updated: bool,
}

pub async fn create_or_update_issue(
    github: &octocrab::Octocrab,
    project: &Project,
    thread: &GuildChannel,
    content: String,
    thread_owner_name: String,
) -> Result<IssueResult> {
    let discord_url = format!(
        "https://discord.com/channels/{}/{}",
        thread.guild_id, thread.id
    );

    // Extract tag from thread title if present
    let original_title = thread.name.clone();
    let mut labels = Vec::new();

    // Check for thread prefixes and map to GitHub labels
    if original_title.contains(crate::constants::PREFIX_BUG) {
        labels.push(crate::constants::LABEL_BUG.to_string());
    }
    if original_title.contains(crate::constants::PREFIX_FEATURE) {
        labels.push(crate::constants::LABEL_FEATURE.to_string());
    }
    if original_title.contains(crate::constants::PREFIX_QUESTION) {
        labels.push(crate::constants::LABEL_QUESTION.to_string());
    }
    if original_title.contains(crate::constants::PREFIX_FEEDBACK) {
        labels.push(crate::constants::LABEL_FEEDBACK.to_string());
    }

    // Add thread ID to title to make it unique
    let title = format!("{} [{}]", original_title, thread.id);

    let body = format!(
        "{content}\n\n---\n**Discord Thread**: {discord_url}\n**Created by**: {thread_owner_name}"
    );

    // Search for existing issue with this thread ID
    let search_query = format!(
        "[{}] in:title repo:{}/{} is:issue",
        thread.id, project.github_owner, project.github_repo
    );

    let existing_issues = github
        .search()
        .issues_and_pull_requests(&search_query)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(
                "GitHub API search failed for query '{}': {:?}",
                search_query,
                e
            );
            e
        })?;

    // Check if we found an existing issue
    if let Some(existing_issue) = existing_issues.items.first() {
        // Update the existing issue
        let issue_number = existing_issue.number;

        let updated_issue = github
            .issues(&project.github_owner, &project.github_repo)
            .update(issue_number)
            .body(&body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("GitHub API update issue #{} failed: {:?}", issue_number, e);
                e
            })?;

        Ok(IssueResult {
            issue: updated_issue,
            was_updated: true,
        })
    } else {
        // Create new issue with or without labels
        let issue = if labels.is_empty() {
            github
                .issues(&project.github_owner, &project.github_repo)
                .create(title)
                .body(body)
                .send()
                .await
                .map_err(|e| {
                    tracing::error!(
                        "GitHub API create issue failed for repo {}/{}: {:?}",
                        project.github_owner,
                        project.github_repo,
                        e
                    );
                    e
                })?
        } else {
            github
                .issues(&project.github_owner, &project.github_repo)
                .create(title)
                .body(body)
                .labels(labels)
                .send()
                .await
                .map_err(|e| {
                    tracing::error!(
                        "GitHub API create issue with labels failed for repo {}/{}: {:?}",
                        project.github_owner,
                        project.github_repo,
                        e
                    );
                    e
                })?
        };

        Ok(IssueResult {
            issue,
            was_updated: false,
        })
    }
}

pub async fn extract_thread_content(
    ctx: &serenity::prelude::Context,
    thread: &GuildChannel,
) -> Result<String> {
    let messages = thread
        .messages(
            &ctx,
            GetMessages::new().limit(crate::constants::GITHUB_THREAD_CONTENT_LIMIT),
        )
        .await?;

    let content = messages
        .iter()
        .rev()
        .take(5)
        .map(|m| format!("**@{}**: {}", m.author.name, m.content))
        .collect::<Vec<_>>()
        .join("\n\n");

    Ok(content)
}
