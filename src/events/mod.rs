mod ready;
mod message;
mod cache_ready;
mod guild_create;

use poise::serenity_prelude as serenity;
use crate::Data;
use crate::BotError;

pub async fn handle_event(
    framework: poise::FrameworkContext<'_, Data, BotError>,
    event: &serenity::FullEvent,
) -> Result<(), BotError> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            ready::handle(framework, data_about_bot).await
        },
        serenity::FullEvent::Message { new_message } => {
            message::handle(framework, new_message).await
        },
        serenity::FullEvent::CacheReady { guilds } => {
            cache_ready::handle(framework, guilds).await
        },
        serenity::FullEvent::GuildCreate { guild, is_new } => {
            guild_create::handle(framework, guild, is_new).await
        },
        _ => Ok(()),
    }
}