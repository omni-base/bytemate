
use poise::serenity_prelude::{GuildId};
use crate::{database, BotError, Data};
pub async fn handle(
    framework: poise::FrameworkContext<'_, Data, BotError>,
    guilds: &[GuildId]
) -> Result<(), BotError> {
    let data = framework.user_data();
    let db = data.db.clone();
    database::upsert::upsert_database(db, guilds).await?;
    Ok(())
}