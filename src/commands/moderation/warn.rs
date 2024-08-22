use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::{RunQueryDsl};
use diesel::dsl::sum;
use diesel::row::NamedRow;
use poise::{command, CreateReply};
use poise::serenity_prelude::{CreateEmbed, Member, Timestamp};
use crate::{BotError, Context};
use crate::database::models::Cases;
use crate::database::schema::moderation_settings::dsl::moderation_settings;
use crate::modules::moderation::logs::{log_action, LogData, LogType};
use crate::util::color::BotColors;
use crate::util::timestamp::{Format, TimestampExt};

#[command(slash_command, default_member_permissions="ADMINISTRATOR", guild_only)]
pub async fn warn(
    ctx: Context<'_>,
    #[description = "User to warn"] user: Member,
    #[description = "Number of warning points"] #[min = 1] #[max = 100] #[rename = "points"] action_points: Option<i32>,
    #[description = "Reason for the warning"] #[rename = "reason"] action_reason: Option<String>,
    #[description = "Should the warning expire?"] expire: Option<bool>,
) -> Result<(), BotError> {
    use crate::database::schema::cases::dsl::*;
    use crate::database::schema::moderation_settings::*;

    if user.user.bot() {
        ctx.reply("You can't warn a bot").await?;
        return Ok(());
    }

    if user.user.id == ctx.author().id {
        ctx.reply("You can't warn yourself").await?;
        return Ok(());
    }
    let guild = ctx.guild().unwrap().clone();
    if guild.owner_id == user.user.id.get() {
        ctx.reply("You can't warn the owner of the server").await?;
        return Ok(());
    }

    if user.permissions(ctx.cache()).unwrap().administrator() {
        ctx.reply("You can't warn an administrator").await?;
        return Ok(());
    }

    if guild.owner_id != ctx.author().id {
        let author_highest_role_position = guild.member_highest_role(&ctx.author_member().await.unwrap()).map(|r| r.position).unwrap_or(0);
        let user_highest_role_position = guild.member_highest_role(&user).map(|r| r.position).unwrap_or(0);
        if user_highest_role_position >= author_highest_role_position {
            ctx.reply("You can't warn a user with the same/higher role than you").await?;
            return Ok(());
        }
    }

    let guild = ctx.guild_id().unwrap();
    let data = ctx.data();
    let action_points = action_points.unwrap_or(1);
    let action_reason = action_reason;

    let new_case_id: i32 = data.db.run(|conn| {
        cases
            .select(diesel::dsl::max(case_id))
            .first::<Option<i32>>(conn)
    }).await?.unwrap_or(0) + 1;

    use crate::database::schema::cases::dsl::guild_id as cases_guild_id;
    use crate::database::schema::moderation_settings::dsl::guild_id as moderation_settings_guild_id;

    let expire_time: i64 = data.db.run(|conn| {
        moderation_settings
            .filter(moderation_settings_guild_id.eq(guild.get() as i64))
            .select(warn_expire_time)
            .first::<Option<i64>>(conn)
    }).await?.unwrap_or(3);



    let end_res_date = if expire.unwrap_or(false) {
        Some(chrono::Utc::now() + chrono::Duration::days(expire_time))
    } else {
        None
    };

    let action_reason_for_logging = action_reason.clone();

    let new_case: Cases = Cases {
        guild_id: guild.get() as i64,
        user_id: user.user.id.get() as i64,
        moderator_id: ctx.author().id.get() as i64,
        case_id: new_case_id,
        case_type: "WARN".to_string(),
        reason: action_reason.clone().or(None),
        created_at: chrono::Utc::now(),
        end_date: end_res_date,
        points: Some(action_points).or(None),
    };

    data.db.run(|conn| {
        diesel::insert_into(crate::database::schema::cases::table)
            .values(&new_case)
            .execute(conn)
    }).await?;

    let total_points = data.db.run(|conn| {
        cases
            .filter(cases_guild_id.eq(guild.get() as i64))
            .filter(user_id.eq(user.user.id.get() as i64))
            .select(sum(points))
            .first::<Option<i64>>(conn)
    }).await.unwrap_or(Option::from(action_points as i64));

    let mut e = CreateEmbed::new()
        .title(format!("A warning of {} point{} has been issued", action_points, if action_points == 1 { "" } else { "s" }))
        .color(BotColors::Default.color())
        .field("User", format!("<@{}>", user.user.id), true)
        .field("Moderator", format!("<@{}>", ctx.author().id), true)
        .field("Total Points", total_points.unwrap().to_string(), true);

    if let Some(end_res_date) = end_res_date {
        e = e.field("Expires", Timestamp::from(end_res_date).to_discord_timestamp(Format::LongDateShortTime), true);
    }

    ctx.send(CreateReply::new().embed(e)).await?;

    log_action(LogType::Warn, LogData {
        data: Some(&*ctx.data()),
        ctx: Some(ctx.serenity_context()),
        guild_id: Some(guild.get()),
        user_id: Some(user.user.id.get()),
        moderator_id: Some(ctx.author().id),
        reason: action_reason_for_logging,
        case_id: Some(new_case_id),
        points: Some(action_points),
        duration: end_res_date.map(|_| expire_time.to_string()),
        ..Default::default()
    }).await?;

    Ok(())
}
