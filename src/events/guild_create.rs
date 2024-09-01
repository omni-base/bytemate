
use poise::serenity_prelude::{Guild};
use crate::{database, BotError, Data};
pub async fn handle(
    framework: poise::FrameworkContext<'_, Data, BotError>,
    guild: &Guild,
    is_new: &Option<bool>
) -> Result<(), BotError> {
    let data = framework.user_data();
    let db = data.db.clone();
    if is_new.expect("Expected a boolean value for is_new") {
        let guild_ids = vec![guild.id];
        database::upsert::upsert_database(db, &guild_ids).await?;
    }

    Ok(())
}