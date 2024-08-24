
use std::path::Path;
use std::time::Duration;
use futures::stream::{self, StreamExt};
use poise::{command, CreateReply, send_reply};
use poise::serenity_prelude::{CreateAttachment, CreateChannel, CreateMessage, GetMessages, GuildChannel, Member, Role};
use crate::{BotError, Context};
use crate::localization::manager::{TranslationParam, TranslationRef};
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
    let db = ctx.data().db.clone();
    let locales = ctx.data().localization_manager.clone();
    let lang = locales
        .get_guild_language(db, ctx.guild_id().unwrap()).await.unwrap();

    let channel_id = ctx.channel_id();
    let guild_id = ctx.guild_id().unwrap();



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


        let content = locales.get(
            "commands.moderation.clear.messages_success_reply",
            lang,
            &[
                TranslationParam::from(message_ids.len().to_string()),
                TranslationParam::from(if message_ids.len() == 1 {
                    TranslationRef::new(
                        "commands.moderation.clear.messages_ending_one",
                        vec![]
                    )
                } else {
                    TranslationRef::new(
                        "commands.moderation.clear.messages_ending_multiple",
                        vec![]
                    )
                }),
                TranslationParam::from_option(user.map(|user| TranslationRef::new(
                    "commands.moderation.clear.messages_filtered_by_user",
                    vec![user.user.name.to_string()]
                ))),
                TranslationParam::from_option(role.map(|role| TranslationRef::new(
                    "commands.moderation.clear.messages_filtered_by_role",
                    vec![role.name.to_string()]
                ))),
            ]
        ).await;
        
        send_reply(ctx,
                   CreateReply::new()
                       .content(content)
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
        let content = locales.get(
            "commands.moderation.clear.messages_no_messages_reply",
            lang,
            &[
                TranslationParam::from_option(user.map(|user| TranslationRef::new(
                    "commands.moderation.clear.messages_filtered_by_user",
                    vec![user.user.name.to_string()]
                ))),
                TranslationParam::from_option(role.map(|role| TranslationRef::new(
                    "commands.moderation.clear.messages_filtered_by_role",
                    vec![role.name.to_string()]
                ))),
            ]
        ).await;
        
        send_reply(ctx,
                   CreateReply::new()
                       .content(content)
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