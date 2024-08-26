use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::{RunQueryDsl};
use diesel::dsl::sum;
use poise::{command, CreateReply};
use poise::serenity_prelude::{CreateEmbed, Member, Timestamp};
use crate::{BotError, Context};
use crate::database::models::Cases;
use crate::database::schema::moderation_settings::dsl::moderation_settings;
use crate::localization::manager::TranslationParam;
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

    let db = ctx.data().db.clone();
    let locales = ctx.data().localization_manager.clone();
    let guild_lang = locales
        .get_guild_language(db, ctx.guild_id().unwrap()).await.unwrap();
    
    if user.user.bot() {
        ctx.reply(locales.get("commands.moderation.warn.error_user_bot", guild_lang, &[])).await?;
        return Ok(());
    }

    if user.user.id == ctx.author().id {
        ctx.reply(locales.get("commands.moderation.warn.error_user_self", guild_lang, &[])).await?;
        return Ok(());
    }
    let guild = ctx.guild().unwrap().clone();
    if guild.owner_id == user.user.id.get() {
        ctx.reply(locales.get("commands.moderation.warn.error_user_owner", guild_lang, &[])).await?;
        return Ok(());
    }

    if user.permissions(ctx.cache()).unwrap().administrator() {
        ctx.reply(locales.get("commands.moderation.warn.error_user_admin", guild_lang, &[])).await?;
        return Ok(());
    }

    if guild.owner_id != ctx.author().id {
        let author_highest_role_position = guild.member_highest_role(&ctx.author_member().await.unwrap()).map(|r| r.position).unwrap_or(0);
        let user_highest_role_position = guild.member_highest_role(&user).map(|r| r.position).unwrap_or(0);
        if user_highest_role_position >= author_highest_role_position {
            ctx.reply(locales.get("commands.moderation.warn.error_user_higher", guild_lang, &[])).await?;
            return Ok(());
        }
    }

    let guild = ctx.guild_id().unwrap();
    let data = ctx.data();
    let action_points = action_points.unwrap_or(1);

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
            .first::<i64>(conn)
    }).await?;



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

    let points_text = if action_points == 1 {
        locales.get("commands.moderation.warn.point", guild_lang, &[])
    } else {
        locales.get("commands.moderation.warn.points", guild_lang, &[])
    };
    
    let title = locales.get("commands.moderation.warn.reply_success_title", guild_lang, &[
        TranslationParam::from(action_points.to_string()),
        TranslationParam::from(points_text)
    ]);
    
    
    
    let mut e = CreateEmbed::new()
        .title(title)
        .color(BotColors::Default.color())
        .field(locales.get("commands.moderation.warn.reply_success_field_user", guild_lang, &[]), format!("<@{}>", user.user.id), true)
        .field(locales.get("commands.moderation.warn.reply_success_field_mod", guild_lang, &[]), format!("<@{}>", ctx.author().id), true)
        .field(locales.get("commands.moderation.warn.reply_success_field_total", guild_lang, &[]), total_points.unwrap().to_string(), true);

    if let Some(end_res_date) = end_res_date {
        e = e.field(locales.get("commands.moderation.warn.reply_success_field_expires", guild_lang, &[]), Timestamp::from(end_res_date).to_discord_timestamp(Format::LongDateShortTime), true);
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
