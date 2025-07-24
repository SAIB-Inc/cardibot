use crate::config::Config;
use serenity::all::*;
use std::sync::Arc;

pub fn create_issue_command() -> CreateCommand {
    CreateCommand::new("issue")
        .description("Manage GitHub issues")
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "create",
            "Create a GitHub issue from this thread",
        ))
}

pub async fn handle_issue_command(
    ctx: &Context,
    command: &CommandInteraction,
    config: &Arc<Config>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Defer the response immediately to avoid timeout
    command
        .create_response(
            &ctx,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await?;

    // Check if in a forum thread
    let channel = command.channel_id.to_channel(&ctx).await?;
    let thread = match channel {
        Channel::Guild(ch) if ch.thread_metadata.is_some() => ch,
        _ => {
            command
                .edit_response(
                    &ctx,
                    EditInteractionResponse::new()
                        .content("This command only works in forum threads!"),
                )
                .await?;
            return Ok(());
        }
    };

    // Find project config
    let guild_id = command.guild_id.unwrap();
    let parent_id = thread.parent_id.unwrap();

    let project = match config.find_project(guild_id.get(), parent_id.get()) {
        Some(p) => {
            tracing::info!(
                "Found project '{}' for guild {} forum {}",
                p.name.as_deref().unwrap_or("unnamed"),
                guild_id,
                parent_id
            );
            p
        }
        None => {
            command
                .edit_response(
                    &ctx,
                    EditInteractionResponse::new()
                        .content("This forum is not configured for issue tracking"),
                )
                .await?;
            return Ok(());
        }
    };

    // Check permissions
    if let Some(required_role_id) = &project.allowed_role_id {
        let member = &command.member.as_ref().unwrap();
        let required_role_id = required_role_id
            .parse::<u64>()
            .map_err(|_| "Invalid role ID in configuration")?;

        let has_role = member
            .roles
            .iter()
            .any(|role_id| role_id.get() == required_role_id);

        if !has_role {
            command
                .edit_response(
                    &ctx,
                    EditInteractionResponse::new()
                        .content("You don't have permission to create issues"),
                )
                .await?;
            return Ok(());
        }
    }

    // Extract thread content
    let content = crate::github::extract_thread_content(ctx, &thread).await?;

    // Get thread owner's username
    let thread_owner_name = if let Some(owner_id) = thread.owner_id {
        match owner_id.to_user(&ctx).await {
            Ok(user) => user.name,
            Err(_) => "Unknown".to_string(),
        }
    } else {
        "Unknown".to_string()
    };

    // Create a fresh GitHub client
    let github = crate::github_app::create_github_client().await?;

    // Create or update GitHub issue
    tracing::info!(
        "Creating/updating GitHub issue for thread '{}' in project '{}'",
        thread.name,
        project.name.as_deref().unwrap_or(&project.github_repo)
    );
    let result = crate::github::create_or_update_issue(
        &github,
        project,
        &thread,
        content,
        thread_owner_name,
    )
    .await?;

    let action = if result.was_updated {
        "Updated"
    } else {
        "Created"
    };
    tracing::info!(
        "{} GitHub issue #{} for project '{}'",
        action,
        result.issue.number,
        project.name.as_deref().unwrap_or(&project.github_repo)
    );

    // Post GitHub link in thread
    let embed_title = if result.was_updated {
        crate::constants::MSG_ISSUE_UPDATED
    } else {
        crate::constants::MSG_ISSUE_CREATED
    };

    thread
        .send_message(
            &ctx,
            CreateMessage::new().embed(
                CreateEmbed::new()
                    .title(embed_title)
                    .description(format!("**Issue**: {}", result.issue.html_url))
                    .field("Number", format!("#{}", result.issue.number), true)
                    .field("Status", "Open", true)
                    .color(crate::constants::COLOR_SUCCESS),
            ),
        )
        .await?;

    // Update the deferred response
    command
        .edit_response(
            &ctx,
            EditInteractionResponse::new()
                .content(format!("✅ {} issue #{}", action, result.issue.number)),
        )
        .await?;

    Ok(())
}
