
use poise::serenity_prelude::{CacheHttp, Message};
use crate::{BotError, Data};
pub async fn handle(
    framework: poise::FrameworkContext<'_, Data, BotError>,
    new_message: &Message
) -> Result<(), BotError> {
    let client_id = framework.user_data().client_id.read().unwrap().to_string();
    if new_message.content.contains(&format!("<@{}>", client_id)) {
        new_message.reply(framework.serenity_context.http(), "Hello!").await?;
    }

    Ok(())
}