use diesel::prelude::*;
use chrono::{DateTime, Utc};
use diesel::associations::HasTable;
use diesel_async::RunQueryDsl;
use poise::{command, CreateReply, send_reply};
use poise::serenity_prelude::{Member};
use crate::{BotError, Context};
use crate::database::models::Cases;
use crate::modules::moderation::logs::{log_action, LogData, LogType};

use crate::util::time::parse_to_time;

/// Ban a user
#[command(slash_command, default_member_permissions="ADMINISTRATOR", guild_only)]
pub async fn ban(
    ctx: Context<'_>,
    #[description = "a user to ban"]
    user: Member,
    #[description = "The ban duration (e.g. \"13d\"). Leave empty for permanent."]
    duration: Option<String>,
    #[description = "reason for banning the user"]
    #[rename = "reason"]
    action_reason: Option<String>,
    #[description = "number of days to delete messages from the user"]
    #[min = 1] #[max = 7]
    delete_message_days: Option<u8>
) -> Result<(), BotError> {
    use crate::database::schema::cases::dsl::*;

    if user.user.bot() {
        send_reply(ctx,
                   CreateReply::new().content("You can't ban a bot").ephemeral(true)
        ).await?;
        return Ok(());
    }
    if user.user.id == ctx.author().id {
        send_reply(ctx,
                   CreateReply::new().content("You can't ban yourself").ephemeral(true)
        ).await?;
        return Ok(());
    }

    let guild = ctx.guild().unwrap().clone();
    if guild.owner_id == user.user.id {
        send_reply(ctx,
                   CreateReply::new().content("You can't ban the owner of the server").ephemeral(true)
        ).await?;
        return Ok(());
    }

    if user.permissions(ctx.cache()).unwrap().administrator() {
        send_reply(ctx,
                   CreateReply::new().content("You can't ban an administrator").ephemeral(true)
        ).await?;
        return Ok(());
    }

    let author_highest_role_position = guild.member_highest_role(&ctx.author_member().await.unwrap()).map(|r| r.position).unwrap_or(0);

    let user_highest_role_position = guild.member_highest_role(&user).map(|r| r.position).unwrap_or(0);

    let bot_highest_role_position = guild.member_highest_role(&guild.id.member(ctx.http(), ctx.http().get_current_user().await.unwrap().id).await.unwrap()).map(|r| r.position).unwrap_or(0);

    if user_highest_role_position >= bot_highest_role_position {
        send_reply(ctx,
                   CreateReply::new().content("I can't ban a user with the same/higher role than me").ephemeral(true)
        ).await?;
        return Ok(());
    }

    if guild.owner_id != ctx.author().id && author_highest_role_position <= user_highest_role_position {
        send_reply(ctx,
                   CreateReply::new().content("You can't ban a user with the same/higher role than you").ephemeral(true)
        ).await?;
        return Ok(());
    }

    if parse_to_time(duration.clone().unwrap()).lt(&60.into()) {
        send_reply(ctx,
                   CreateReply::new().content("You can't ban a user for less than 1 minute").ephemeral(true)
        ).await?;
        return Ok(());
    }
    
    let expires_at: Option<DateTime<Utc>> = duration.clone().map(|d| {
        let duration = parse_to_time(d).unwrap();
        #[warn(clippy::needless_return)]
        return Utc::now() + chrono::Duration::seconds(duration as i64);
    });
    
    
    
    user.ban(ctx.http(), delete_message_days.unwrap_or(0), action_reason.as_deref().or(None)).await?;

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
        case_type: "BAN".to_string(),
        reason: action_reason.clone().or(None),
        created_at: Utc::now(),
        end_date: Some(expires_at.unwrap()).or(None),
        points: None
    };

    let _ = data.db.run(|conn| {
        diesel::insert_into(cases::table())
            .values(&new_case)
            .execute(conn)
    }).await?;

    

    let duration_text = duration.clone().map(|d| format!(" for {}", d)).unwrap_or_else(|| "permanently".to_string());
    send_reply(ctx,
               CreateReply::new().content(format!("Banned {} {}{}", user.user.tag(), duration_text, action_reason.clone().map(|r| format!(" with reason: {}", r)).unwrap_or("No reason provided".to_string()))).ephemeral(false)
    ).await?;
    
    let data = ctx.data().clone();
    
    let log_data = LogData {
        ctx: Some(ctx.serenity_context()),
        data: Some(&*data),
        guild_id: Some(ctx.guild_id().unwrap().get()),
        user_id: Some(user.user.id.get()),
        moderator_id: Some(ctx.author().id),
        reason: action_reason.clone(),
        duration: duration.clone(),
        case_id: Some(new_case_id),
        ..LogData::default()
    };
    
    log_action(LogType::Ban, log_data).await?;
    
    Ok(())
}