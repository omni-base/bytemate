
use poise::{command};
use poise::serenity_prelude::{EditChannel, GuildChannel, PermissionOverwrite, PermissionOverwriteType, Permissions};
use crate::{BotError, Context};

use crate::modules::moderation::logs::{log_action, LogData, LogType};

#[command(slash_command, default_member_permissions="MANAGE_CHANNELS", subcommands("lock", "unlock"), guild_only)]
pub async fn channel(_: Context<'_>) -> Result<(), BotError> { Ok(()) }

/// Lock a channel
#[command(slash_command, default_member_permissions="MANAGE_CHANNELS", guild_only)]
pub async fn lock(
    ctx: Context<'_>,
    #[description = "Channel to lock"]
    #[channel_types("Text")]
    channel: Option<GuildChannel>,
    #[description = "Reason for locking the channel"]
    reason: Option<String>
) -> Result<(), BotError> {
    let current_channel = ctx.guild_channel().await.unwrap();

    let db = ctx.data().db.clone();
    let locales = ctx.data().localization_manager.clone();
    let guild_lang = locales
        .get_guild_language(db, ctx.guild_id().unwrap()).await.unwrap();
    
    

    let mut channel = channel.unwrap_or(current_channel);

    let everyone = ctx.guild().unwrap().id.everyone_role().get();

    let mut permissions: Vec<PermissionOverwrite> = channel.permission_overwrites.clone().into();

    if let Some(perms) = permissions.iter_mut().find(|p| p.kind == PermissionOverwriteType::Role(everyone.into())) {
        if perms.deny.contains(Permissions::SEND_MESSAGES) {
            let error_msg = locales.get("commands.moderation.channel.error_already_locked", guild_lang, &[]).await;
            ctx.say(error_msg).await?;
            return Ok(());
        } else {
            let msg = locales.get("commands.moderation.channel.reply_locked", guild_lang, &[]).await;
            ctx.say(msg).await?;
            perms.deny.insert(Permissions::SEND_MESSAGES);
            perms.deny.insert(Permissions::SEND_MESSAGES_IN_THREADS);
            perms.allow.remove(Permissions::SEND_MESSAGES);
            perms.allow.remove(Permissions::SEND_MESSAGES_IN_THREADS);
        }
    } else {
        permissions.push(PermissionOverwrite {
            kind: PermissionOverwriteType::Role(everyone.into()),
            allow: Permissions::empty(),
            deny: Permissions::SEND_MESSAGES | Permissions::SEND_MESSAGES_IN_THREADS,
         });
        let msg = locales.get("commands.moderation.channel.reply_locked", guild_lang, &[]).await;
        ctx.say(msg).await?;
    }

    channel.edit(ctx.http(), EditChannel::new().permissions(permissions)).await?;
    
    let data = ctx.data().clone();
    
    let no_reason = locales.get("commands.moderation.channel.no_reason", guild_lang, &[]).await;
    
    let log_data = LogData {
        data: Some(&*data),
        ctx: Some(ctx.serenity_context()),
        guild_id: Some(ctx.guild_id().unwrap().get()),
        channel_id: Some(channel.id.get()),
        moderator_id: Some(ctx.author().id),
        reason: reason.or(Option::from(no_reason)),
        ..LogData::default()
    };
    
    log_action(LogType::Lock, log_data).await?;
    
    Ok(())
}

/// Unlock a channel
#[command(slash_command, default_member_permissions="MANAGE_CHANNELS", guild_only)]
pub async fn unlock(
    ctx: Context<'_>,
    #[description = "Channel to unlock"]
    #[channel_types("Text")]
    channel: Option<GuildChannel>,
    #[description = "Reason for unlocking the channel"]
    reason: Option<String>
) -> Result<(), BotError> {
    let db = ctx.data().db.clone();
    let locales = ctx.data().localization_manager.clone();
    let guild_lang = locales
        .get_guild_language(db, ctx.guild_id().unwrap()).await.unwrap();
    
    let current_channel = ctx.guild_channel().await.unwrap();

    let mut channel = channel.unwrap_or(current_channel);

    let everyone = ctx.guild().unwrap().id.everyone_role().get();

    let mut permissions: Vec<PermissionOverwrite> = channel.permission_overwrites.clone().into();

    if let Some(perms) = permissions.iter_mut().find(|p| p.kind == PermissionOverwriteType::Role(everyone.into())) {
        if perms.deny.contains(Permissions::SEND_MESSAGES) {
            let msg = locales.get("commands.moderation.channel.reply_unlocked", guild_lang, &[]).await;
            ctx.say(msg).await?;
            perms.deny.remove(Permissions::SEND_MESSAGES);
            perms.allow.insert(Permissions::SEND_MESSAGES);
            perms.deny.remove(Permissions::SEND_MESSAGES_IN_THREADS);
            perms.allow.insert(Permissions::SEND_MESSAGES_IN_THREADS);
        } else {
            let error_msg = locales.get("commands.moderation.channel.error_already_unlocked", guild_lang, &[]).await;
            ctx.say(error_msg).await?;
            return Ok(());
        }
    } else {
        let error_msg = locales.get("commands.moderation.channel.error_already_unlocked", guild_lang, &[]).await;
        ctx.say(error_msg).await?;
        return Ok(());
    }

    channel.edit(ctx.http(), EditChannel::new().permissions(permissions)).await?;

    let data = ctx.data().clone();
  
    let no_reason = locales.get("commands.moderation.channel.no_reason", guild_lang, &[]).await;
    
    let log_data = LogData {
        data: Some(&*data),
        ctx: Some(ctx.serenity_context()),
        guild_id: Some(ctx.guild_id().unwrap().get()),
        channel_id: Some(channel.id.get()),
        moderator_id: Some(ctx.author().id),
        reason: reason.or(Option::from(no_reason)),
        ..LogData::default()
    };
    
    log_action(LogType::Unlock, log_data).await?;

    Ok(())
}