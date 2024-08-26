use std::sync::Arc;
use crate::BotError;
use crate::database::manager::DbManager;
use diesel_async::RunQueryDsl;
use diesel::prelude::*;
use poise::serenity_prelude::GuildId;

pub async fn upsert_database(
    db: Arc<DbManager>,
    guilds: &[GuildId]
) -> Result<(), BotError> {


    upsert_guild_settings(db.clone(), guilds).await?;
    
    upsert_moderation_settings(db.clone(), guilds).await?;

    Ok(())
}

async fn upsert_guild_settings(
    db: Arc<DbManager>,
    guilds: &[GuildId]
) -> Result<(), BotError> {
    use crate::database::schema::guild_settings::dsl::*;

    let new_guild_settings: Vec<_> = guilds.iter().map(|guild| {
        (guild_id.eq(guild.get() as i64), lang.eq("en"))
    }).collect::<Vec<_>>();
    
    db.run(|conn| {
        diesel::insert_into(guild_settings)
            .values(&new_guild_settings)
            .on_conflict(guild_id)
            .do_nothing()
            .execute(conn)
    }).await?;
    
    Ok(())
}

async fn upsert_moderation_settings(
    db: Arc<DbManager>,
    guilds: &[GuildId]
) -> Result<(), BotError> {
    use crate::database::schema::moderation_settings::dsl::*;

    let new_moderation_settings: Vec<_> = guilds.iter().map(|guild| {
        (
            guild_id.eq(guild.get() as i64),
            warn_expire_time.eq(3),
            log_types.eq(4095),
            default_log_channel.eq::<Option<i64>>(None),
        )
    }).collect::<Vec<_>>();

    db.run(|conn| {
        diesel::insert_into(moderation_settings)
            .values(&new_moderation_settings)
            .on_conflict(guild_id)
            .do_nothing()
            .execute(conn)
    }).await?;
    
    Ok(())
}

