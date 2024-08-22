use chrono::{DateTime, Utc};
use diesel_async::RunQueryDsl;
use poise::{command, CreateReply, send_reply};
use poise::serenity_prelude::{EditMember, Member};
use crate::{BotError, Context};
use crate::database::models::Cases;
use crate::modules::moderation::logs::{log_action, LogData, LogType};
use crate::util::time::{date_after, parse_to_time};
use crate::util::util::generate_case_id;

/// Mute a user
#[command(slash_command, default_member_permissions="ADMINISTRATOR", guild_only)]
pub async fn mute(
    ctx: Context<'_>,
    #[description = "a user to mute"]
    mut user: Member,
    #[description = "time to mute the user (no more than 28 days)"]
    duration: String,
    #[description = "reason for muting the user"] #[rename = "reason"]
    action_reason: Option<String>
) -> Result<(), BotError> {
    use crate::database::schema::cases::dsl::*;
    
    if user.user.bot() {
        send_reply(ctx, CreateReply::new().content("You can't mute a bot").ephemeral(true)).await?;
        return Ok(());
    }

    if user.user.id == ctx.author().id {
        send_reply(ctx, CreateReply::new().content("You can't mute yourself").ephemeral(true)).await?;
        return Ok(());
    }

    if user.permissions(ctx.cache()).unwrap().administrator() {
        send_reply(ctx, CreateReply::new().content("You can't mute an administrator").ephemeral(true)).await?;
        return Ok(());
    }

    if parse_to_time(duration.clone()).is_none() {
        send_reply(ctx, CreateReply::new().content("Invalid time format").ephemeral(true)).await?;
        return Ok(());
    }

    let date = date_after(parse_to_time(duration.clone()).unwrap());

    if date.timestamp() - Utc::now().timestamp() > 2419200 {
        send_reply(ctx, CreateReply::new().content("You can't mute a user for more than 28 days").ephemeral(true)).await?;
        return Ok(());
    }

    if user.communication_disabled_until.is_some_and(|c| c.timestamp() > Utc::now().timestamp()) {
        send_reply(ctx, CreateReply::new().content("User is already muted").ephemeral(true)).await?;
        return Ok(());
    }

    let mut builder = EditMember::new().disable_communication_until(date);

    if let Some(ref action_reason) = action_reason {
        builder = builder.audit_log_reason(action_reason);
    }

    user.edit(ctx.http(), builder).await?;

    let data = ctx.data().clone();
    
    let mut db_conn = data.db.lock().await;
    
    let new_case_id = generate_case_id(&mut db_conn).await;
    
    let expires_at: Option<DateTime<Utc>> = parse_to_time(duration.clone()).map(|d| {
        Utc::now() + chrono::Duration::seconds(d as i64)
    });

    let reason_string = action_reason.clone().unwrap_or_else(|| "No reason".to_string());

    // TODO: Rewrite all reasons to be Reason or None
    let new_case: Cases = Cases {
        guild_id: ctx.guild_id().unwrap().get() as i64,
        user_id: user.user.id.get() as i64,
        moderator_id: ctx.author().id.get() as i64,
        case_id: new_case_id,
        case_type: "MUTE".to_string(),
        reason: Option::from(reason_string.clone()),
        created_at: Utc::now(),
        end_date: Some(expires_at.unwrap()).or(None),
        points: None
    };
    
    diesel::insert_into(cases)
        .values(&new_case)
        .execute(&mut *db_conn).await.expect("Failed to insert case into database");
    

    send_reply(ctx, CreateReply::new().content(format!(
        "Muted {} for {} with reason: {}",
        user.user.tag(), duration, action_reason.as_deref().unwrap_or("No reason provided"))
    ).ephemeral(false)).await?;

    let data = ctx.data().clone();

    let log_data = LogData {
        data: Some(&*data),
        ctx: Some(ctx.serenity_context()),
        guild_id: Some(ctx.guild_id().unwrap().get()),
        user_id: Some(user.user.id.get()),
        moderator_id: Some(ctx.author().id),
        duration: Some(duration),
        reason: Some(reason_string),  
        case_id: Some(new_case_id),
        ..LogData::default()
    };

    log_action(LogType::Mute, log_data).await?;

    Ok(())
}
