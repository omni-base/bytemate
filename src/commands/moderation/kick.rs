use diesel::associations::HasTable;
use diesel_async::RunQueryDsl;
use poise::{command, CreateReply, send_reply};
use poise::serenity_prelude::{Member};
use crate::{BotError, Context};
use crate::database::models::Cases;
use crate::modules::moderation::logs::{log_action, LogData, LogType};
use crate::util::util::generate_case_id;

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
    
    if user.user.bot() {
        send_reply(ctx, CreateReply::new().content("You can't kick a bot").ephemeral(true)).await?;
        return Ok(());
    }

    if user.user.id == ctx.author().id {
        send_reply(ctx, CreateReply::new().content("You can't kick yourself").ephemeral(true)).await?;
        return Ok(());
    }

    let guild = ctx.guild().unwrap().clone();
    if guild.owner_id == user.user.id {
        send_reply(ctx, CreateReply::new().content("You can't kick the owner of the server").ephemeral(true)).await?;
        return Ok(());
    }

    if user.permissions(ctx.cache()).unwrap().administrator() {
        send_reply(ctx, CreateReply::new().content("You can't kick an administrator").ephemeral(true)).await?;
        return Ok(());
    }

    let author_highest_role_position = guild.member_highest_role(&ctx.author_member().await.unwrap()).map(|r| r.position).unwrap_or(0);
    let user_highest_role_position = guild.member_highest_role(&user).map(|r| r.position).unwrap_or(0);
    let bot_highest_role_position = guild.member_highest_role(&guild.id.member(ctx.http(), ctx.http().get_current_user().await.unwrap().id).await.unwrap()).map(|r| r.position).unwrap_or(0);

    if user_highest_role_position >= bot_highest_role_position {
        send_reply(ctx, CreateReply::new().content("I can't kick a user with the same/higher role than me").ephemeral(true)).await?;
        return Ok(());
    }

    if guild.owner_id != ctx.author().id && author_highest_role_position <= user_highest_role_position {
        send_reply(ctx, CreateReply::new().content("You can't kick a user with the same/higher role than you").ephemeral(true)).await?;
        return Ok(());
    }
    
    let action_reason_string = action_reason.clone().unwrap_or_else(|| "No action_reason".to_string());

    user.kick(ctx.http(), action_reason.as_deref()).await?;

    let data = ctx.data();
    
    let mut db_conn = data.db.lock().await;
    
    let new_case_id = generate_case_id(&mut db_conn).await;

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
    
    diesel::insert_into(cases::table())
        .values(&new_case)
        .execute(&mut *db_conn)
        .await
        .expect("Failed to insert case into database");
    

    send_reply(ctx, CreateReply::new().content(format!(
        "Kicked {} with action_reason: {}",
        user.user.tag(),
        action_reason.as_deref().unwrap_or("No action_reason")
    )).ephemeral(false)).await?;

    let log_data = LogData {
        data: Some(&*data),
        ctx: Some(ctx.serenity_context()),
        guild_id: Some(ctx.guild_id().unwrap().get()),
        channel_id: Some(ctx.channel_id().get()),
        user_id: Some(user.user.id.get()),
        reason: Option::from(action_reason_string), 
        case_id: Some(new_case_id),
        ..Default::default()
    };

    log_action(LogType::Kick, log_data).await?;

    Ok(())
}
