
use std::collections::{BTreeMap, HashMap};
use diesel::{ExpressionMethods, QueryDsl,  SelectableHelper};
use diesel_async::RunQueryDsl;
use futures::future::join_all;
use poise::CreateReply;
use poise::serenity_prelude::{ButtonStyle,  CreateActionRow, CreateButton, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage, Timestamp, User, UserId};
use poise::serenity_prelude::nonmax::NonMaxU64;
use crate::{BotError, Context};
use crate::database::models::Cases;
use crate::modules::moderation::logs::{log_action, LogData, LogType};
use crate::util::color::{BotColors};
use crate::util::timestamp::{Format, TimestampExt};

#[poise::command(slash_command, guild_only, subcommands("view", "remove"), subcommand_required)]
pub async fn cases(_: Context<'_>) -> Result<(), BotError> {
    Ok(())
}

/// View case(s) for a user or the entire server
#[poise::command(slash_command, guild_only)]
pub async fn view(
    ctx: Context<'_>,
    #[description = "Case(s) of this user"] user: Option<User>,
    #[description = "The case ID"] case: Option<i32>,
    #[description = "The case type"] #[rename = "type"] case_res_type: Option<String>,
    #[description = "The case(s) moderator"] #[rename = "mod"] case_res_moderator: Option<User>,
) -> Result<(), BotError> {
    use crate::database::schema::cases::dsl::*;

    let guild = ctx.guild_id().unwrap().get() as i64;

    let data = ctx.data();
    

    if let Some(case_res_id) = case {
        if user.is_some() {
            ctx.say("Don't provide both a user and a case ID.").await?;
            return Ok(());
        }

        if case_res_type.is_some() {
            ctx.say("Don't provide both a case ID and a case type.").await?;
            return Ok(());
        }

        if case_res_moderator.is_some() {
            ctx.say("Don't provide both a case ID and a moderator.").await?;
            return Ok(());
        }

        
        let case = data.db.run(|conn| {
            cases
                .filter(guild_id.eq(guild))
                .filter(case_id.eq(case_res_id))
                .select(Cases::as_select())
                .first::<Cases>(conn)
        }).await.ok();

        if case.is_none() {
            ctx.say("No case found with the given ID.").await?;
            return Ok(());
        }

        let case = case.unwrap();


        let before_user = UserId::new(u64::from(NonMaxU64::try_from(case.user_id as u64).unwrap()));

        let moderator = UserId::new(u64::from(NonMaxU64::try_from(case.moderator_id as u64).unwrap()));

        let user = before_user.to_user(ctx.http()).await?;

        let points_info = if case.case_type == "WARN" {
            format!("`Points:` {}", case.points.unwrap_or(0))
        } else {
            String::new()
        };

        let embed = CreateEmbed::new().color(BotColors::Default.color())
            .author(CreateEmbedAuthor::new(format!("Case Info for {}", user.clone().global_name.unwrap_or_else(|| user.name.clone())))
                .icon_url(user.avatar_url().unwrap_or_default()))
            .description(format!(
                "`Case ID:` #{}\n`User:` <@{}> ({})\n`Action Type:` **{}**\n`Moderator:` <@{}>\n`Reason:` {}\n{}\n`Created:` <t:{}:R>\n`Expires:` {}",
                case.case_id,
                before_user,
                before_user,
                case.case_type,
                moderator,
                case.reason.clone().unwrap_or_else(|| "No reason provided".to_string()),
                points_info,
                case.created_at.timestamp(),
                case.end_date.map_or("Never".to_string(), |dt| {
                    Timestamp::from(dt).to_discord_timestamp(Format::LongDateShortTime)
                })
            ));

            ctx.send(CreateReply::new().embed(embed)).await?;
            return Ok(())
        }

    if let Some(moderator_res) = case_res_moderator {
        if user.is_some() {
            ctx.say("Don't provide both a user and a moderator.").await?;
            return Ok(());
        }

        if case_res_type.is_some() {
            ctx.say("Don't provide both a moderator and a case type.").await?;
            return Ok(());
        }

        if case.is_some() {
            ctx.say("Don't provide both a moderator and a case ID.").await?;
            return Ok(());
        }

        let moderator = UserId::new(u64::from(NonMaxU64::try_from(moderator_res.id.get()).unwrap())).to_user(ctx.http()).await?;

        let cases_results = data.db.run(|conn| {
            cases
                .filter(guild_id.eq(guild))
                .filter(moderator_id.eq(moderator.id.get() as i64))
                .select(Cases::as_select())
                .load::<Cases>(conn)
        }).await.ok();

        if cases_results.is_none() {
            ctx.say(format!("No cases found for {}.", moderator.clone().global_name.unwrap())).await?;
            return Ok(());
        }

        let cases_results = cases_results.unwrap();

        let mut result = String::new();

        for case in cases_results {
            let user = UserId::new(u64::from(NonMaxU64::try_from(case.user_id as u64).unwrap()));
            let points_info = if case.case_type == "WARN" {
                format!("`Points:` {}", case.points.unwrap_or(0))
            } else {
                String::new()
            };

            result += &format!(
                "`Case ID:` #{}\n`User:` <@{}> ({:?})\n`Action Type:` **{}**\n`Reason:` {}\n{}\n`Created:` <t:{}:R>\n`Expires:` {}\n\n",
                case.case_id,
                user.get(),
                user.get(),
                case.case_type,
                case.reason.clone().unwrap_or_else(|| "No reason provided".to_string()),
                points_info,
                case.created_at.timestamp(),
                case.end_date.map_or("Never".to_string(), |dt| {
                    Timestamp::from(dt).to_discord_timestamp(Format::LongDateShortTime)
                })
            );
        }

        let embed = CreateEmbed::new().color(BotColors::Default.color())
            .author(CreateEmbedAuthor::new(format!("{}'s cases", moderator.clone().global_name.unwrap()))
                .icon_url(moderator.avatar_url().unwrap_or_default()))
            .description(result);

        ctx.send(CreateReply::new().embed(embed)).await?;
        return Ok(());
    }

    let cases_result = if let Some(ref user) = user {
        let user = UserId::new(u64::from(NonMaxU64::try_from(user.id.get()).unwrap()));

        data.db.run(|conn| {
            cases
                .filter(guild_id.eq(guild))
                .filter(user_id.eq(user.get() as i64))
                .select(Cases::as_select())
                .load::<Cases>(conn)
        }).await?
    } else {
        data.db.run(|conn| {
            cases
                .filter(guild_id.eq(guild))
                .select(Cases::as_select())
                .load::<Cases>(conn)
        }).await?
    };
    
    if cases_result.is_empty() {
        ctx.say("No cases found.").await?;
        return Ok(());
    }


    let mut grouped_cases: BTreeMap<i64, Vec<Cases>> = BTreeMap::new();
    let mut user_ids = Vec::new();
    let mut moderator_ids = Vec::new();

    for case in &cases_result {
        grouped_cases.entry(case.user_id)
            .or_default()
            .push(case.clone());
        if !user_ids.contains(&case.user_id) {
            user_ids.push(case.user_id);
        }
        if !moderator_ids.contains(&case.moderator_id) {
            moderator_ids.push(case.moderator_id);
        }
    }

    for all_cases in grouped_cases.values_mut() {
        all_cases.sort_by_key(|case| case.case_id);
    }

    let user_futures: Vec<_> = user_ids.iter()
        .map(|&id_res| UserId::new(u64::from(NonMaxU64::new(id_res as u64).unwrap())).to_user(ctx.http()))
        .collect();
    let moderator_futures: Vec<_> = moderator_ids.iter()
        .map(|&id_res| UserId::new(u64::from(NonMaxU64::new(id_res as u64).unwrap())).to_user(ctx.http()))
        .collect();

    let (users, moderators) = futures::join!(join_all(user_futures), join_all(moderator_futures));

    let mut user_info = HashMap::new();
    for user in users.into_iter().filter_map(Result::ok) {
        user_info.insert(user.id, user.global_name.unwrap_or_else(|| user.name.clone()));
    }

    let mut moderator_info = HashMap::new();
    for moderator in moderators.into_iter().filter_map(Result::ok) {
        moderator_info.insert(moderator.id, moderator.global_name.unwrap_or_else(|| moderator.name.clone()));
    }

    let cases_per_page = 10;
    let total_cases: usize = grouped_cases.values().map(|v| v.len()).sum();
    let pages = (total_cases as f32 / cases_per_page as f32).ceil() as usize;
    let mut current_page = 0;

    let create_message = move |page: usize| {
        let mut cases_embed = CreateEmbed::new().color(BotColors::Default.color());
        let mut result = String::new();
        let mut count = 0;
        let start = page * cases_per_page;
        let end = start + cases_per_page;

        for (&user_res_id, cases_res) in &grouped_cases {
            let user_res_id = UserId::new(u64::from(NonMaxU64::new(user_res_id as u64).unwrap()));
            let user_name = user_info.get(&user_res_id).cloned().unwrap_or_else(|| user_res_id.to_string().parse().unwrap());

            if count >= start && count < end {
                result += &format!("**{}'s cases**\n", user_name);
            }

            for case in cases_res {
                if count >= start && count < end {
                    let moderator = UserId::new(u64::from(NonMaxU64::try_from(case.moderator_id as u64).unwrap()));
                    let _moderator_name = moderator_info.get(&moderator).cloned().unwrap_or_else(|| moderator.to_string().parse().unwrap());

                    let points_info = if case.case_type == "WARN" {
                        format!("`Points:` {}", case.points.unwrap_or(0))
                    } else {
                        String::new()
                    };
                    result += &format!(
                        "`Case ID:` #{}\n`User:` <@{}> ({})\n`Action Type:` **{}**\n`Moderator:` <@{}>\n`Reason:` {}\n{}\n`Expires:` {}\n\n",
                        case.case_id,
                        user_res_id.get(),
                        user_res_id.get(),
                        case.case_type,
                        moderator,
                        case.reason.clone().unwrap_or_else(|| "No reason provided".to_string()),
                        points_info,
                        case.end_date.map_or("Never".to_string(), |dt| {
                            Timestamp::from(dt).to_discord_timestamp(Format::LongDateShortTime)
                        })
                    );
                }
                count += 1;
                if count >= end {
                    break;
                }
            }
            if count >= end {
                break;
            }
        }

        cases_embed = cases_embed.description(result);
        if let Some(ref user) = user {
            cases_embed = cases_embed.author(CreateEmbedAuthor::new(format!("Cases for {}", user.name)));
        } else {
            cases_embed = cases_embed.author(CreateEmbedAuthor::new("Cases for the server").icon_url(ctx.guild().unwrap().icon_url().unwrap_or_default()));
        }
        cases_embed = cases_embed.footer(CreateEmbedFooter::new(format!("Page {}/{}", page + 1, pages)));

        let mut components = vec![];
        let previous_button = CreateButton::new("prev").style(ButtonStyle::Primary).emoji('⬅').disabled(page == 0);
        let next_button = CreateButton::new("next")
            .style(ButtonStyle::Primary)
            .emoji('➡')
            .disabled(page >= pages - 1);
        let action_row = CreateActionRow::Buttons(vec![previous_button, next_button]);
        components.push(action_row);

        (cases_embed, components)
    };

    let (content, components) = create_message(current_page);
    let message = ctx.send(CreateReply::new()
        .embed(content)
        .components(components)
    ).await?;

    while let Some(interaction) = message.message().await?.await_component_interaction(ctx.serenity_context().clone().shard).await {
        match interaction.data.custom_id.as_str() {
            "prev" => current_page = current_page.saturating_sub(1),
            "next" => current_page = (current_page + 1).min(pages - 1),
            _ => continue,
        }

        let (content, components) = create_message(current_page);
        interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(CreateInteractionResponseMessage::new().embed(content).components(components))).await?;
    }

    Ok(())
}



/// Remove one or multiple Cases
#[poise::command(slash_command, guild_only)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "Case ID(s) to remove (e.g., 1 or 1,2,3,4)"] case_ids: String,
) -> Result<(), BotError> {
    use crate::database::schema::cases::dsl::*;

    ctx.defer().await?;
    let guild = ctx.guild_id().unwrap().get();
    let data = ctx.data();

    let case_ids: Vec<i32> = case_ids.split(',')
        .filter_map(|id_res| id_res.trim().parse().ok())
        .collect();

    if case_ids.is_empty() {
        ctx.say("No valid case IDs provided.").await?;
        return Ok(());
    }
    

    let mut response = String::new();
    let mut removed_warns: Vec<(UserId, i32, i32)> = Vec::new();

    let all_cases = data.db.run(|conn| {
        cases
            .filter(guild_id.eq(guild as i64))
            .filter(case_id.eq_any(&case_ids))
            .select(Cases::as_select())
            .load::<Cases>(conn)
    }).await?;


    let cases_map: HashMap<i32, Cases> = all_cases.into_iter().map(|c| (c.case_id, c)).collect();

    for &case_res_id in &case_ids {
        if let Some(case) = cases_map.get(&case_res_id) {
            let user = UserId::new(u64::from(NonMaxU64::try_from(case.user_id as u64).unwrap()));
            let guild = ctx.guild_id().unwrap();

            match case.case_type.as_str() {
                "MUTE" => {
                    let log_data = LogData {
                        ctx: Some(ctx.serenity_context()),
                        guild_id: Some(guild.get()),
                        user_id: Some(user.get()),
                        moderator_id: Some(ctx.author().id),
                        case_id: Some(case_res_id),
                        data: Some(&*data),
                        ..LogData::default()
                    };

                    log_action(LogType::Unmute, log_data).await?;
                    guild.member(ctx.http(), user).await?.enable_communication(ctx.http()).await?;
                },
                "BAN" => {
                    let log_data = LogData {
                        ctx: Some(ctx.serenity_context()),
                        guild_id: Some(guild.get()),
                        user_id: Some(user.get()),
                        moderator_id: Some(ctx.author().id),
                        case_id: Some(case_res_id),
                        data: Some(&*data),
                        ..LogData::default()
                    };

                    log_action(LogType::Unban, log_data).await?;
                    guild.unban(ctx.http(), user, Some(&format!("Case {} removed", case_res_id))).await?;
                },
                "WARN" => {
                    let points_res = case.points.unwrap_or(1);
                    removed_warns.push((user, case_res_id, points_res));
                },
                _ => {}
            }

            response.push_str(&format!("Case {} removed for {}.\n", case_res_id, user.to_user(ctx.http()).await?.name));
        } else {
            response.push_str(&format!("Case {} not found.\n", case_res_id));
        }
    }

    let _ = data.db.run(|conn| {
        diesel::delete(
            cases.filter(guild_id.eq(guild as i64))
                .filter(case_id.eq_any(&case_ids))
        ).execute(conn)
    }).await?;

    if !removed_warns.is_empty() {
        let log_data = LogData {
            ctx: Some(ctx.serenity_context()),
            guild_id: Some(guild),
            moderator_id: Some(ctx.author().id),
            data: Some(&*data),
            removed_warns: Some(removed_warns.clone()),
            ..LogData::default()
        };

        if removed_warns.len() > 1 {
            log_action(LogType::RemoveMultipleWarns, log_data).await?;
        } else if let Some((user, case_res_id, action_points)) = removed_warns.get(0) {
            let single_warn_log_data = LogData {
                ctx: Some(ctx.serenity_context()),
                guild_id: Some(guild),
                user_id: Some(user.get()),
                moderator_id: Some(ctx.author().id),
                case_id: Some(*case_res_id),
                data: Some(&*data),
                points: Some(*action_points),
                ..LogData::default()
            };
            log_action(LogType::RemoveWarn, single_warn_log_data).await?;
        }
    }

    ctx.say(response).await?;

    Ok(())
}
