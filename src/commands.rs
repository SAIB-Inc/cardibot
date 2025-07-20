use crate::bot::Bot;
use serenity::all::*;

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
    bot: &Bot,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if in a forum thread
    let channel = command.channel_id.to_channel(&ctx).await?;
    let thread = match channel {
        Channel::Guild(ch) if ch.thread_metadata.is_some() => ch,
        _ => {
            command
                .create_response(
                    &ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("This command only works in forum threads!")
                            .ephemeral(true),
                    ),
                )
                .await?;
            return Ok(());
        }
    };

    // Find project config
    let guild_id = command.guild_id.unwrap();
    let parent_id = thread.parent_id.unwrap();

    let project = match bot.config.find_project(guild_id.get(), parent_id.get()) {
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
                .create_response(
                    &ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("This forum is not configured for issue tracking")
                            .ephemeral(true),
                    ),
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
                .create_response(
                    &ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("You don't have permission to create issues")
                            .ephemeral(true),
                    ),
                )
                .await?;
            return Ok(());
        }
    }

    // Extract thread content
    let content = crate::github::extract_thread_content(&ctx, &thread).await?;

    // Get thread owner's username
    let thread_owner_name = if let Some(owner_id) = thread.owner_id {
        match owner_id.to_user(&ctx).await {
            Ok(user) => user.name,
            Err(_) => "Unknown".to_string(),
        }
    } else {
        "Unknown".to_string()
    };

    // Create or update GitHub issue
    tracing::info!(
        "Creating/updating GitHub issue for thread '{}' in project '{}'",
        thread.name,
        project.name.as_deref().unwrap_or(&project.github_repo)
    );
    let result = crate::github::create_or_update_issue(
        &bot.github,
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
        "GitHub Issue Updated"
    } else {
        "GitHub Issue Created"
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
                    .color(0x238636),
            ),
        )
        .await?;

    // Respond to interaction
    command
        .create_response(
            &ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(format!("✅ {} issue #{}", action, result.issue.number))
                    .ephemeral(true),
            ),
        )
        .await?;

    Ok(())
}
