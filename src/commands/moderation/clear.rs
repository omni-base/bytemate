
use std::path::Path;
use std::time::Duration;
use futures::stream::{self, StreamExt};
use poise::{command, CreateReply, send_reply};
use poise::serenity_prelude::{CreateAttachment, CreateChannel, CreateMessage, GetMessages, GuildChannel, Member, Role};
use crate::{BotError, Context};
use crate::modules::moderation::logs::{log_action, LogData, LogType};

#[command(slash_command, default_member_permissions = "MANAGE_MESSAGES", subcommands("channel", "messages"), guild_only)]
pub async fn clear(_: Context<'_>) -> Result<(), BotError> {
    Ok(())
}




/// Clear a specified number of messages
#[command(slash_command, default_member_permissions = "MANAGE_MESSAGES", guild_only)]
pub async fn messages(
    ctx: Context<'_>,
    #[description = "Number of messages to delete (1-100)"]
    #[min = 1] #[max = 100]
    amount: u8,
    #[description = "Filter by user messages"]
    user: Option<Member>,
    #[description = "Filter by role messages"]
    role: Option<Role>,
) -> Result<(), BotError> {
    let channel_id = ctx.channel_id();
    let guild_id = ctx.guild_id().ok_or_else(|| BotError::from("Command must be used in a guild"))?;

    let messages = channel_id.messages(&ctx.http(), GetMessages::new().limit(amount)).await?;

    let filtered_messages = stream::iter(messages)
        .filter_map(|msg| async {
            let user_match = user
                .as_ref()
                .map_or(true, |member| member.user.id == msg.author.id);

            let role_match = if let Some(role) = &role {
                guild_id.member(&ctx.http(), msg.author.id).await
                    .map(|member| member.roles.contains(&role.id))
                    .unwrap_or(false)
            } else {
                true
            };

            if user_match && role_match {
                Some(msg)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .await;

    let mut messages_for_logs: Vec<String> = Vec::new();
    let mut total_length = 0;

    for message in filtered_messages.iter() {
        let message_content = format!("[{}] {}", message.author.global_name.as_deref().unwrap_or(message.author.name.as_str()), message.content);
        if total_length + message_content.len() > 1800 {
            break;
        }
        messages_for_logs.push(message_content.clone());
        total_length += message_content.len();
    }

    if total_length > 1800 && !messages_for_logs.is_empty() {
        messages_for_logs.last_mut().unwrap().push_str(" [...]");
    }

    let message_ids: Vec<_> = filtered_messages.iter().map(|m| m.id).collect();

    if !message_ids.is_empty() {
        channel_id.delete_messages(ctx.http(), &message_ids, None).await?;

        let reply_content = format!(
            "Successfully deleted {} message(s).{}{}",
            message_ids.len(),
            user.map_or(String::new(), |m| format!(" Filtered by user: {}.", m.user.name)),
            role.map_or(String::new(), |r| format!(" Filtered by role: {}.", r.name))
        );

        send_reply(ctx,
                   CreateReply::new()
                       .content(reply_content)
                       .ephemeral(true)
        ).await?;

        let data = ctx.data().clone();
        
        let log_data = LogData {
            data: Some(&*data),
            ctx: Some(ctx.serenity_context()),
            guild_id: Some(guild_id.get()),
            channel_id: Some(channel_id.get()),
            moderator_id: Some(ctx.author().id),
            messages_deleted: Some(message_ids.len() as u32),
            messages: Some(messages_for_logs),
            ..LogData::default()
        };
        
        log_action(LogType::ClearMessages, log_data).await?;
    } else {
        let reply_content = format!(
            "No messages matched the filter criteria.{}{}",
            user.map_or(String::new(), |m| format!(" User filter: {}.", m.user.name)),
            role.map_or(String::new(), |r| format!(" Role filter: {}.", r.name))
        );

        send_reply(ctx,
                   CreateReply::new()
                       .content(reply_content)
                       .ephemeral(true)
        ).await?;
    }

    Ok(())
}
/// Delete all messages by cloning the channel
#[command(slash_command, default_member_permissions = "MANAGE_MESSAGES", guild_only)]
pub async fn channel(
    ctx: Context<'_>,
    #[description = "Channel to clear"]
    #[channel_types("Text")]
    channel: Option<GuildChannel>,
) -> Result<(), BotError> {
    let channel = if let Some(channel) = channel {
        channel
    } else {
        ctx.guild_channel().await.unwrap()
    };
    
    let guild = ctx.guild().unwrap().clone();
    let new_channel = guild.create_channel(ctx.http(), CreateChannel::new(channel.name)
        .permissions(channel.permission_overwrites)
        .position(channel.position)
        .nsfw(channel.nsfw)
        .topic(channel.topic.unwrap_or_default())
        .category(channel.parent_id.unwrap_or_default())
        .kind(channel.kind)).await;
    ctx.guild_channel().await.unwrap().delete(ctx.http(), None).await?;

    if let Ok(ref channel) = new_channel {
        let message = channel.send_message(ctx.http(), CreateMessage::new().content(format!("<@{}>", ctx.author().id)).add_file(CreateAttachment::file(&tokio::fs::File::open(Path::new("./src/images/clearall.gif")).await.unwrap(), "clearall.gif").await.unwrap()))
            .await?;
        
        let http = ctx.serenity_context().clone().http;

        // Spawn a new task to delete the message after 5 seconds
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            message.delete(&http, None).await.unwrap();
        });
    }


    if let Ok(channel) = new_channel {
        let data = ctx.data().clone();
        
        let log_data = LogData {
            data: Some(&*data),
            moderator_id: Some(ctx.author().id),
            ctx: Some(ctx.serenity_context()),
            guild_id: Some(guild.id.get()),
            channel_id: Some(channel.id.get()),
            ..LogData::default()
        };
        
        log_action(LogType::ClearChannel, log_data).await?;
    }

    Ok(())
}