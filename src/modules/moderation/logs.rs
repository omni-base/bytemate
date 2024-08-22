use std::ops::BitAnd;
use chrono::Utc;
use diesel::{ExpressionMethods};
use poise::serenity_prelude::{CacheHttp, ChannelId, Context, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateMessage, GuildId, Timestamp, UserId};
use crate::{BotError, Data};
use crate::database::models::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use crate::database::schema::logs::dsl::logs;
use crate::util::color::BotColors;

#[derive(Debug, Clone, Copy)]
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
    pub fn to_string(&self) -> &'static str {
        match self {
            LogType::ClearMessages => "Clear Messages",
            LogType::ClearChannel => "Clear Channel",
            LogType::Mute => "Mute",
            LogType::Unmute => "Unmute",
            LogType::Kick => "Kick",
            LogType::Lock => "Lock",
            LogType::Unlock => "Unlock",
            LogType::Ban => "Ban",
            LogType::Unban => "Unban",
            LogType::Warn => "Warn",
            LogType::RemoveWarn => "Remove Warn",
            LogType::RemoveMultipleWarns => "Remove Multiple Warns",
        }
    }
    pub fn as_bit(&self) -> u32 {
        *self as u32
    }
}

pub fn get_active_log_types(mask: u32) -> Vec<&'static str> {
    let mut active_types = Vec::new();

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
            active_types.push(log_type.to_string());
        }
    }

    active_types
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

impl BitAnd<LogType> for i32 {
    type Output = bool;

    fn bitand(self, rhs: LogType) -> Self::Output {
        let mask: u32 = self as u32;
        mask & rhs.as_bit() != 0
    }
}

pub async fn log_action(log_type: LogType, log_data: LogData<'_>) -> Result<(), BotError> {
    use crate::database::schema::logs::*;
    let data = log_data.data.unwrap();
    
    let log = data.db.run(|conn| {
        logs
            .filter(guild_id.eq(log_data.guild_id.unwrap() as i64))
            .select(Logs::as_select())
            .first::<Logs>(conn)
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
