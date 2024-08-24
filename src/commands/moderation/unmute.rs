use chrono::Utc;
use poise::{command, CreateReply, send_reply};
use poise::serenity_prelude::{Member};
use crate::{BotError, Context};
use diesel_async::RunQueryDsl;
use diesel::prelude::*;
use crate::localization::manager::TranslationParam;
use crate::modules::moderation::logs::{log_action, LogData, LogType};

/// Remove a mute from a user
#[command(slash_command, default_member_permissions="ADMINISTRATOR", guild_only)]
pub async fn unmute(
    ctx: Context<'_>,
    #[description = "a user to unmute"]
    mut user: Member,
) -> Result<(), BotError> {
    use crate::database::schema::cases::dsl::*;
    
    let db = ctx.data().db.clone();
    let locales = ctx.data().localization_manager.clone();
    let guild_lang = locales
        .get_guild_language(db, ctx.guild_id().unwrap()).await.unwrap();
    
    let guild = ctx.guild().unwrap().clone();
    
    if user.user.bot() {
        let error_msg = locales.get("commands.moderation.unmute.error_user_bot", guild_lang, &[]).await;
        send_reply(ctx,
                   CreateReply::new().content(error_msg).ephemeral(true)
        ).await?;
        return Ok(());
    }

    if user.user.id == ctx.author().id {
        let error_msg = locales.get("commands.moderation.unmute.error_user_self", guild_lang, &[]).await;
        send_reply(ctx,
                   CreateReply::new().content(error_msg).ephemeral(true)
        ).await?;
        return Ok(());
    }

    if user.user.id == ctx.guild().unwrap().owner_id {
        let error_msg = locales.get("commands.moderation.unmute.error_user_owner", guild_lang, &[]).await;
        send_reply(ctx, CreateReply::new().content(error_msg).ephemeral(true)).await?;
        return Ok(());
    }

    if user.permissions(ctx.cache()).unwrap().administrator() {
        let error_msg = locales.get("commands.moderation.unmute.error_user_admin", guild_lang, &[]).await;
        send_reply(ctx, CreateReply::new().content(error_msg).ephemeral(true)).await?;
        return Ok(());
    }

    let author_highest_role_position = guild.member_highest_role(&ctx.author_member().await.unwrap()).map(|r| r.position).unwrap_or(0);

    let user_highest_role_position = guild.member_highest_role(&user).map(|r| r.position).unwrap_or(0);

    let bot_highest_role_position = guild.member_highest_role(&guild.id.member(ctx.http(), ctx.http().get_current_user().await.unwrap().id).await.unwrap()).map(|r| r.position).unwrap_or(0);

    if user_highest_role_position >= bot_highest_role_position {
        let error_msg = locales.get("commands.moderation.unmute.error_user_higher_role", guild_lang, &[]).await;

        send_reply(ctx,
                   CreateReply::new().content(error_msg).ephemeral(true)
        ).await?;
        return Ok(());
    }

    if guild.owner_id != ctx.author().id && author_highest_role_position <= user_highest_role_position {
        let error_msg = locales.get("commands.moderation.unmute.error_user_higher_role", guild_lang, &[]).await;

        send_reply(ctx,
                   CreateReply::new().content(error_msg).ephemeral(true)
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
        let error_msg = locales.get("commands.moderation.unmute.error_user_not_muted", guild_lang, &[]).await;
        send_reply(ctx,
                   CreateReply::new().content(error_msg).ephemeral(true)
        ).await?;
        return Ok(());
    }

    user.enable_communication(ctx.http()).await?;

    let content = locales.get("commands.moderation.unmute.reply_success", guild_lang, &[
        TranslationParam::from(user.user.tag().as_str())
    ]).await;

    send_reply(ctx,
               CreateReply::new().content(content).ephemeral(false)
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