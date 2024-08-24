use diesel::associations::HasTable;
use diesel::QueryDsl;
use diesel_async::RunQueryDsl;
use poise::{command, CreateReply, send_reply};
use poise::serenity_prelude::{Member};
use crate::{BotError, Context};
use crate::database::models::Cases;
use crate::localization::manager::TranslationParam;
use crate::modules::moderation::logs::{log_action, LogData, LogType};

/// Kick a user
#[command(slash_command, default_member_permissions="ADMINISTRATOR", guild_only)]
pub async fn kick(
    ctx: Context<'_>,
    #[description = "a user to kick"]
    user: Member,
    #[description = "reason for kicking the user"] #[rename = "reason"]
    action_reason: Option<String>
) -> Result<(), BotError> {
    use crate::database::schema::cases::dsl::*;

    let db = ctx.data().db.clone();
    let locales = ctx.data().localization_manager.clone();
    let guild_lang = locales
        .get_guild_language(db, ctx.guild_id().unwrap()).await.unwrap();


    if user.user.bot() {
        let error_msg = locales.get("commands.moderation.kick.error_user_bot", guild_lang, &[]).await;

        send_reply(ctx, CreateReply::new().content(error_msg).ephemeral(true)).await?;
        return Ok(());
    }

    if user.user.id == ctx.author().id {
        let error_msg = locales.get("commands.moderation.kick.error_user_self", guild_lang, &[]).await;

        send_reply(ctx, CreateReply::new().content(error_msg).ephemeral(true)).await?;
        return Ok(());
    }

    let guild = ctx.guild().unwrap().clone();
    if guild.owner_id == user.user.id {
        let error_msg = locales.get("commands.moderation.kick.error_user_owner", guild_lang, &[]).await;

        send_reply(ctx, CreateReply::new().content(error_msg).ephemeral(true)).await?;
        return Ok(());
    }

    if user.permissions(ctx.cache()).unwrap().administrator() {
        let error_msg = locales.get("commands.moderation.kick.error_user_admin", guild_lang, &[]).await;

        send_reply(ctx, CreateReply::new().content(error_msg).ephemeral(true)).await?;
        return Ok(());
    }

    let author_highest_role_position = guild.member_highest_role(&ctx.author_member().await.unwrap()).map(|r| r.position).unwrap_or(0);
    let user_highest_role_position = guild.member_highest_role(&user).map(|r| r.position).unwrap_or(0);
    let bot_highest_role_position = guild.member_highest_role(&guild.id.member(ctx.http(), ctx.http().get_current_user().await.unwrap().id).await.unwrap()).map(|r| r.position).unwrap_or(0);

    if user_highest_role_position >= bot_highest_role_position {
        let error_msg = locales.get("commands.moderation.kick.user_me_higher", guild_lang, &[]).await;

        send_reply(ctx, CreateReply::new().content(error_msg).ephemeral(true)).await?;
        return Ok(());
    }

    if guild.owner_id != ctx.author().id && author_highest_role_position <= user_highest_role_position {
        let error_msg = locales.get("commands.moderation.kick.error_user_higher", guild_lang, &[]).await;

        send_reply(ctx, CreateReply::new().content(error_msg).ephemeral(true)).await?;
        return Ok(());
    }


    user.kick(ctx.http(), action_reason.as_deref().or(None)).await?;

    let data = ctx.data();

    let new_case_id: i32 = data.db.run(|conn| {
        cases
            .select(diesel::dsl::max(case_id))
            .first::<Option<i32>>(conn)
    }).await?.unwrap_or(0) + 1;

    let new_case: Cases = Cases {
        guild_id: ctx.guild_id().unwrap().get() as i64,
        user_id: user.user.id.get() as i64,
        moderator_id: ctx.author().id.get() as i64,
        case_id: new_case_id,
        case_type: "KICK".to_string(),
        reason: action_reason.clone().or(None),
        created_at: chrono::Utc::now(),
        end_date: None,
        points: None,
    };
    
    data.db.run(|conn| {
        diesel::insert_into(cases::table())
            .values(&new_case)
            .execute(conn)
    }).await?;

    let action_reason = if let Some(action_reason) = action_reason.clone() {
        action_reason
    } else {
        locales.get(
            "commands.moderation.kick.no_reason",
            guild_lang,
            &[]
        ).await
    };
    
    let content = locales.get(
        "commands.moderation.kick.reply_success",
        guild_lang,
        &[
            TranslationParam::from(user.user.tag()),
            TranslationParam::from(action_reason.clone()),
        ]
    ).await;

    send_reply(ctx, CreateReply::new().content(content).ephemeral(true)).await?;
    
    let log_data = LogData {
        data: Some(&*data),
        ctx: Some(ctx.serenity_context()),
        guild_id: Some(ctx.guild_id().unwrap().get()),
        channel_id: Some(ctx.channel_id().get()),
        user_id: Some(user.user.id.get()),
        reason: Some(action_reason), 
        case_id: Some(new_case_id),
        ..Default::default()
    };

    log_action(LogType::Kick, log_data).await?;

    Ok(())
}
