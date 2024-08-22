use chrono::Utc;
use poise::{command, CreateReply, send_reply};
use poise::serenity_prelude::{Member};
use crate::{BotError, Context};
use diesel_async::RunQueryDsl;
use diesel::prelude::*;
use crate::modules::moderation::logs::{log_action, LogData, LogType};

/// Remove a mute from a user
#[command(slash_command, default_member_permissions="ADMINISTRATOR", guild_only)]
pub async fn unmute(
    ctx: Context<'_>,
    #[description = "a user to unmute"]
    mut user: Member,
) -> Result<(), BotError> {
    use crate::database::schema::cases::dsl::*;
    
    if user.user.bot() {
        send_reply(ctx,
                   CreateReply::new().content("You can't unmute a bot").ephemeral(true)
        ).await?;
        return Ok(());
    }

    if user.user.id == ctx.author().id {
        send_reply(ctx,
                   CreateReply::new().content("You can't unmute yourself").ephemeral(true)
        ).await?;
        return Ok(());
    }

    let is_muted = ctx.data().db.run(|conn| {
        cases
            .filter(user_id.eq(user.user.id.get() as i64))
            .filter(case_type.eq("MUTE"))
            .filter(end_date.gt(Utc::now()))
            .select(case_id)
            .first::<i32>(conn)
    }).await.ok().is_some();
    
    if !is_muted {
        send_reply(ctx,
                   CreateReply::new().content("User is not muted").ephemeral(true)
        ).await?;
        return Ok(());
    }

    user.enable_communication(ctx.http()).await?;


    send_reply(ctx,
               CreateReply::new().content(format!("Unmuted {}", user.user.tag())).ephemeral(false)
    ).await?;
    
    let data = ctx.data().clone();
    
    let log_data = LogData {
        data: Some(&*data),
        ctx: Some(ctx.serenity_context()),
        guild_id: Some(ctx.guild_id().unwrap().get()),
        user_id: Some(user.user.id.get()),
        moderator_id: Some(ctx.author().id),
        ..LogData::default()
    };
    
    log_action(LogType::Unmute, log_data).await?;
    

    Ok(())
}