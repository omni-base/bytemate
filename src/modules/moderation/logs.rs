use std::ops::BitAnd;
use std::str::FromStr;
use chrono::Utc;
use diesel::{ExpressionMethods};
use poise::serenity_prelude::{CacheHttp, ChannelId, Context, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateMessage, GuildId, Timestamp, UserId};
use crate::{BotError, Data};
use crate::database::models::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use crate::localization::manager::{Language, LocalizationManager};
use crate::util::color::BotColors;
use strum_macros::EnumIter;
#[derive(Debug, Clone, Copy, EnumIter)]
#[repr(u32)]
pub enum LogType {
    ClearMessages = 1 << 0,      // 00000001
    ClearChannel = 1 << 1,       // 00000010
    Mute = 1 << 2,               // 00000100
    Unmute = 1 << 3,             // 00001000
    Kick = 1 << 4,               // 00010000
    Lock = 1 << 5,               // 00100000
    Unlock = 1 << 6,             // 01000000
    Ban = 1 << 7,                // 10000000
    Unban = 1 << 8,              // 00000001 00000000
    Warn = 1 << 9,               // 00000010 00000000
    RemoveWarn = 1 << 10,        // 00000100 00000000
    RemoveMultipleWarns = 1 << 11, // 00001000 00000000
}

impl LogType {
    pub fn to_string(&self, manager: &LocalizationManager, lang: Language) -> String {
        match self {
            LogType::ClearMessages => manager.get("commands.configuration.moderation.actions.clear_messages", lang, &[]),
            LogType::ClearChannel => manager.get("commands.configuration.moderation.actions.clear_channel", lang, &[]),
            LogType::Mute => manager.get("commands.configuration.moderation.actions.mute", lang, &[]),
            LogType::Unmute => manager.get("commands.configuration.moderation.actions.unmute", lang, &[]),
            LogType::Kick => manager.get("commands.configuration.moderation.actions.kick", lang, &[]),
            LogType::Lock => manager.get("commands.configuration.moderation.actions.lock", lang, &[]),
            LogType::Unlock => manager.get("commands.configuration.moderation.actions.unlock", lang, &[]),
            LogType::Ban => manager.get("commands.configuration.moderation.actions.ban", lang, &[]),
            LogType::Unban => manager.get("commands.configuration.moderation.actions.unban", lang, &[]),
            LogType::Warn => manager.get("commands.configuration.moderation.actions.warn", lang, &[]),
            LogType::RemoveWarn => manager.get("commands.configuration.moderation.actions.remove_warn", lang, &[]),
            LogType::RemoveMultipleWarns => manager.get("commands.configuration.moderation.actions.remove_multiple_warns", lang, &[]),
        }
    }
    pub fn as_bit(&self) -> u32 {
        *self as u32
    }
}

pub fn get_active_log_types(mask: u32, manager: &LocalizationManager, lang: Language) -> Vec<String> {
    let mut active_types: Vec<String> = Vec::new();
    
    for &log_type in &[
        LogType::ClearMessages,
        LogType::ClearChannel,
        LogType::Mute,
        LogType::Unmute,
        LogType::Kick,
        LogType::Lock,
        LogType::Unlock,
        LogType::Ban,
        LogType::Unban,
        LogType::Warn,
        LogType::RemoveWarn,
        LogType::RemoveMultipleWarns,
    ] {
        if mask & log_type.as_bit() != 0 {
            active_types.push(log_type.to_string(manager, lang));
        }
    }
    
    active_types
}

pub fn string_to_log_type(s: &str, manager: &LocalizationManager, lang: Language) -> Option<LogType> {
    LogType::from_str(s).ok().or_else(|| {
        [LogType::ClearMessages, LogType::ClearChannel, LogType::Mute, LogType::Unmute,
            LogType::Kick, LogType::Lock, LogType::Unlock, LogType::Ban, LogType::Unban,
            LogType::Warn, LogType::RemoveWarn, LogType::RemoveMultipleWarns]
            .iter()
            .find(|&log_type| log_type.to_string(manager, lang) == s)
            .copied()
    })
}

#[derive(Default)]
pub struct LogData<'a> {
    pub ctx: Option<&'a Context>,
    pub data: Option<&'a Data>,
    pub guild_id: Option<u64>,
    pub user_id: Option<u64>,
    pub channel_id: Option<u64>,
    pub moderator_id: Option<UserId>,
    pub reason: Option<String>,
    pub duration: Option<String>,
    pub case_id: Option<i32>,
    pub points: Option<i32>,
    pub messages_deleted: Option<u32>,
    pub messages: Option<Vec<String>>,
    pub removed_warns: Option<Vec<(UserId, i32, i32)>>,
}

impl<'a> LogData<'a> {
    pub fn new(ctx: &'a Context, data: &'a Data, guild_id: u64, moderator_id: UserId) -> Self {
        Self {
            ctx: Some(ctx),
            data: Some(data),
            guild_id: Some(guild_id),
            moderator_id: Some(moderator_id),
            user_id: None,
            channel_id: None,
            reason: None,
            duration: None,
            case_id: None,
            points: None,
            messages_deleted: None,
            messages: None,
            removed_warns: None,
        }
    }
}

impl FromStr for LogType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ClearMessages" => Ok(LogType::ClearMessages),
            "ClearChannel" => Ok(LogType::ClearChannel),
            "Mute" => Ok(LogType::Mute),
            "Unmute" => Ok(LogType::Unmute),
            "Kick" => Ok(LogType::Kick),
            "Lock" => Ok(LogType::Lock),
            "Unlock" => Ok(LogType::Unlock),
            "Ban" => Ok(LogType::Ban),
            "Unban" => Ok(LogType::Unban),
            "Warn" => Ok(LogType::Warn),
            "RemoveWarn" => Ok(LogType::RemoveWarn),
            "RemoveMultipleWarns" => Ok(LogType::RemoveMultipleWarns),
            _ => Err(()),
        }
    }
}

impl BitAnd<LogType> for i32 {
    type Output = bool;

    fn bitand(self, rhs: LogType) -> Self::Output {
        let mask: u32 = self as u32;
        mask & rhs.as_bit() != 0
    }
}

pub async fn log_action(log_type: LogType, log_data: LogData<'_>) -> Result<(), BotError> {
    use crate::database::schema::moderation_settings::dsl::*;
    let data = log_data.data.unwrap();
    
    let log = data.db.run(|conn| {
        moderation_settings
            .filter(guild_id.eq(log_data.guild_id.unwrap() as i64))
            .select(ModerationSettings::as_select())
            .first::<ModerationSettings>(conn)
    }).await?;
    
    let log_embed = create_log_embed(&log_type, &log_data).await;

    if log.default_log_channel.is_none() {
        return Ok(());
    }

    let active_types: u32 = log.log_types as u32;
    
    if active_types & log_type.as_bit() == 0 {
        return Ok(());
    }

    ChannelId::new(log.default_log_channel.unwrap() as u64)
        .send_message(log_data.ctx.unwrap().http(), CreateMessage::new().embed(log_embed))
        .await?;

    Ok(())
}

async fn create_log_embed(log_type: &LogType, log_data: &LogData<'_>) -> CreateEmbed<'static> {
    let (title, description) = match log_type {
        LogType::ClearMessages => (
            format!("{} Messages Purged", log_data.messages_deleted.unwrap_or(0)),
            format!(
                "`Channel:` <#{}> \n\n ```{}```",
                log_data.channel_id.unwrap(),
                log_data.messages.as_ref().map_or("Could not log messages".to_string(), |m| m.join("\n"))
            ),
        ),
        LogType::ClearChannel => (
            "Channel nuked".to_string(),
            format!("`Channel:` <#{}> \n\n ```All messages purged```", log_data.channel_id.unwrap()),
        ),
        LogType::Mute => (
            "User Muted".to_string(),
            format!(
                "`User:` <@{}> \n`Reason:` {} \n`Duration:` {} \n`Case ID:` {}",
                log_data.user_id.unwrap(),
                log_data.reason.as_deref().unwrap_or("No reason provided"),
                log_data.duration.as_deref().unwrap_or("N/A"),
                log_data.case_id.unwrap_or(0)
            ),
        ),
        LogType::Unmute => (
            "User Unmuted".to_string(),
            format!("`User:` <@{}>", log_data.user_id.unwrap()),
        ),
        LogType::Kick => (
            "User Kicked".to_string(),
            format!(
                "`User:` <@{}> \n`Reason:` {} \n`Case ID:` {}",
                log_data.user_id.unwrap(),
                log_data.reason.as_deref().unwrap_or("No reason
                provided"),
                log_data.case_id.unwrap_or(0)
            ),
        ),
        LogType::Lock => (
            "Channel Locked".to_string(),
            format!("`Channel:` <#{}> \n`Reason:` **{}**", log_data.channel_id.unwrap(), log_data.reason.as_deref().unwrap_or("No reason provided")),
        ),
        LogType::Unlock => (
            "Channel Unlocked".to_string(),
            format!("`Channel:` <#{}> \n`Reason:` **{}**", log_data.channel_id.unwrap(), log_data.reason.as_deref().unwrap_or("No reason provided")),
        ),
        LogType::Ban => (
            "User Banned".to_string(),
            format!(
                "`User:` <@{}> \n`Reason:` {} \n`Duration:` {} \n`Case ID:` {}",
                log_data.user_id.unwrap(),
                log_data.reason.as_deref().unwrap_or("No reason provided"),
                log_data.duration.as_deref().unwrap_or("N/A"),
                log_data.case_id.unwrap_or(0)
            ),
        ),
        LogType::Unban => (
            "User Unbanned".to_string(),
            format!("`User:` <@{}>", log_data.user_id.unwrap()),
        ),
        LogType::Warn => (
            "User Warned".to_string(),
            format!(
                "`User:` <@{}> \n`Reason:` {} \n`Points:` {} \n`Case ID:` #{}",
                log_data.user_id.unwrap(),
                log_data.reason.as_deref().unwrap_or("No reason provided"),
                log_data.points.unwrap_or(0),
                log_data.case_id.unwrap_or(0)
            ),
        ),
        LogType::RemoveWarn => (
            "Warning Removed".to_string(),
            format!("`User:` <@{}> \n`Points:` {}", log_data.user_id.unwrap(), log_data.points.unwrap_or(0)),
        ),
        LogType::RemoveMultipleWarns => {
            let warns = log_data.removed_warns.as_ref().unwrap();
            let mut user_warns: std::collections::HashMap<UserId, Vec<(i32, i32)>> = std::collections::HashMap::new();

            for (user_id, case_id, points) in warns {
                user_warns.entry(*user_id).or_default().push((*case_id, *points));
            }

            let warn_list = user_warns.iter()
                .map(|(user_id, cases)| {
                    let case_list = cases.iter()
                        .map(|(case_id, points)| format!("  Case #{}: {} points", case_id, points))
                        .collect::<Vec<String>>()
                        .join("\n");
                    format!("<@{}>:\n{}", user_id, case_list)
                })
                .collect::<Vec<String>>()
                .join("\n\n");

            (
                "Multiple Warnings Removed".to_string(),
                format!("`Warnings Removed:` \n{}", warn_list),
            )
        },
    };

    let author = log_data.moderator_id.unwrap().to_user(log_data.ctx.unwrap().http()).await.unwrap();
    let guild = GuildId::new(log_data.guild_id.unwrap()).to_partial_guild(log_data.ctx.unwrap().http()).await.unwrap();
    CreateEmbed::new()
        .color(BotColors::Default.color())
        .author(CreateEmbedAuthor::new(title).icon_url(guild.icon_url().unwrap_or_default())).url(guild.icon_url().unwrap_or_default())
        .description(description)
        .footer(CreateEmbedFooter::new(format!("Action by: {} ({})", author.clone().global_name.unwrap(), author.clone().id)).icon_url(author.clone().avatar_url().unwrap()))
        .timestamp(Timestamp::from(Utc::now()))
}
