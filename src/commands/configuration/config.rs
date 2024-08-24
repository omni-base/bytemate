use std::borrow::Cow;
use std::time::Duration;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use crate::{BotError, Context};
use poise::{command, CreateReply, Modal, send_reply};
use poise::serenity_prelude::{ActionRowComponent, CacheHttp, ChannelId, ChannelType, ComponentInteraction, ComponentInteractionDataKind, CreateActionRow, CreateEmbed,  CreateInteractionResponse, CreateInteractionResponseMessage, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption, InputText, Message, ModalInteraction};
use poise::serenity_prelude::small_fixed_array::{FixedArray};
use crate::database::models::{GuildSettings, Logs, ModerationSettings};
use crate::modules::moderation::logs::{get_active_log_types, LogType};
use crate::util::color::BotColors;

const INTERACTION_TIMEOUT: Duration = Duration::from_secs(60);

/// See or modify the current bot configuration
#[command(slash_command, default_member_permissions = "ADMINISTRATOR", guild_only)]
pub async fn config(ctx: Context<'_>) -> Result<(), BotError> {
    let reply = send_reply(ctx, CreateReply::new().embed(create_config_embed()).components(vec![create_select_menu("config_module", vec![("Core", "core"),("Moderation", "moderation")])])).await?;
    let message = reply.message().await?;

    if let Some(interaction) = await_interaction(&ctx, &message, "config_module").await {
        match get_selected_value(&interaction)?.as_str() {
            "core" => core_config(ctx, interaction).await?,
            "moderation" => moderation_config(ctx, interaction).await?,
            _ => {}
        }
    }

    Ok(())
}

fn create_select_menu<'a>(custom_id: &'a str, options: Vec<(&'a str, &'a str)>) -> CreateActionRow<'a> {
    CreateActionRow::SelectMenu(
        CreateSelectMenu::new(custom_id, CreateSelectMenuKind::String {
            options: Cow::Owned(options.into_iter().map(|(name, value)|
            CreateSelectMenuOption::new(name, value)).collect())
        })
            .placeholder("Select an option")
    )
}

fn create_config_embed() -> CreateEmbed<'static> {
    CreateEmbed::new()
        .title("Configuration")
        .description("Select a module to configure: \n\n **Core**, **Moderation**")
        .color(BotColors::Default.color())

}





fn create_log_channel_config_embed() -> CreateEmbed<'static> {
    CreateEmbed::new()
        .title("Set Log Channel")
        .color(BotColors::Default.color())
        .description("Select a channel to set as the log channel")
}

fn create_log_channel_select_menu(logs_table: Logs) -> CreateActionRow<'static> {
    let select_menu = CreateSelectMenu::new("log_channel", CreateSelectMenuKind::Channel {
        channel_types: Some(Cow::Owned(vec![ChannelType::Text])),
        default_channels: Some(Cow::Owned(vec![logs_table.default_log_channel.map(|id| ChannelId::new(id as u64)).unwrap_or(ChannelId::default())])),
    });
    CreateActionRow::SelectMenu(select_menu)
}

fn create_warn_expire_time_config_embed() -> CreateEmbed<'static> {
    CreateEmbed::new()
        .title("Set Warn Expire Time")
        .color(BotColors::Default.color())
        .description("Select a time for warnings to expire")
}

fn create_warn_expire_time_select_menu(moderation: ModerationSettings) -> CreateActionRow<'static> {
    let current_time = moderation.warn_expire_time;

    let mut options = vec![
        CreateSelectMenuOption::new("3 days", "3").default_selection(current_time == 3),
        CreateSelectMenuOption::new("7 days", "7").default_selection(current_time == 7),
        CreateSelectMenuOption::new("14 days", "14").default_selection(current_time == 14),
        CreateSelectMenuOption::new("30 days", "30").default_selection(current_time == 30),
        CreateSelectMenuOption::new("Custom", "custom").default_selection(false),
    ];

    if current_time != 3 && current_time != 7 && current_time != 14 && current_time != 30 {
        options.insert(5, CreateSelectMenuOption::new(format!("{} days", current_time), current_time.to_string()).default_selection(true));
    }

    let select_menu = CreateSelectMenu::new("warn_expire_time", CreateSelectMenuKind::String {
        options: Cow::Owned(options),
    });

    CreateActionRow::SelectMenu(select_menu)
}

async fn await_interaction(ctx: &Context<'_>, message: &Message, custom_id: &str) -> Option<ComponentInteraction> {
    let interaction = message.await_component_interaction(ctx.serenity_context().shard.clone())
        .timeout(INTERACTION_TIMEOUT)
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .custom_ids(FixedArray::from([custom_id.parse().unwrap()]))
        .await;

    if interaction.is_none() {
        if let Err(e) = message.reply(ctx.http(), "Timed out. Please try again.").await {
            eprintln!("Failed to send timeout message: {:?}", e);
        }
    }

    interaction
}

async fn await_modal_interaction(ctx: &Context<'_>, message: &Message, custom_id: &str) -> Option<ModalInteraction> {
    let interaction = message.await_modal_interaction(ctx.serenity_context().shard.clone())
        .timeout(INTERACTION_TIMEOUT)
        .author_id(ctx.author().id)
        .custom_ids(vec![custom_id.to_string().parse().unwrap()])
        .await;


    if interaction.is_none() {
        if let Err(e) = message.reply(ctx.http(), "Timed out. Please try again.").await {
            eprintln!("Failed to send timeout message: {:?}", e);
        }
    }

    interaction
}

fn get_selected_value(interaction: &ComponentInteraction) -> Result<String, BotError> {
    match &interaction.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => Ok(values[0].clone()),
        ComponentInteractionDataKind::ChannelSelect { values } => Ok(values[0].get().to_string().clone()),
        _ => Err(BotError::from("Invalid interaction data kind"))
    }
}

fn get_modal_value(interaction: &ModalInteraction) -> Result<String, BotError> {
    match &interaction.data.components[0].components[0] {
        ActionRowComponent::InputText { 0: InputText { value, .. } } => Ok(value.clone().unwrap().to_string()),
        _ => Err(Box::from("Invalid component")),
    }
}

async fn create_moderation_config_embed(moderation_table: ModerationSettings, logs_table: Logs) -> CreateEmbed<'static> {
    let active_log_types = get_active_log_types(logs_table.log_types as u32);

    CreateEmbed::new()
        .title("Moderation Config")
        .color(BotColors::Default.color())
        .field("Log Channel", logs_table.default_log_channel
                .map(|id| format!("<#{}>", id))
                .unwrap_or_else(|| "None".to_string()),
            false
        )
        .field("Log Types", active_log_types.join(", "), false)
        .field("Warn Expire Time", format!("{:?} days", moderation_table.warn_expire_time), false)
}


async fn moderation_config(ctx: Context<'_>, interaction: ComponentInteraction) -> Result<(), BotError> {
    use crate::database::schema::moderation_settings::dsl::*;
    use crate::database::schema::logs::dsl::*;
    use crate::database::schema::logs::dsl::guild_id as logs_guild_id;
    use crate::database::schema::moderation_settings::dsl::guild_id as moderation_guild_id;

    let moderation_table = ctx.data().db.run(|conn| {
        moderation_settings
            .filter(moderation_guild_id.eq(ctx.guild_id().unwrap().get() as i64))
            .first::<ModerationSettings>(conn)
    }).await?;

    let logs_table = ctx.data().db.run(|conn| {
        logs
            .filter(logs_guild_id.eq(ctx.guild_id().unwrap().get() as i64))
            .first::<Logs>(conn)
    }).await?;

    interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::default()
            .embed(create_moderation_config_embed(moderation_table.clone(), logs_table.clone()).await)
            .components(vec![create_select_menu("moderation_config", vec![
                ("Log Channel", "log_channel"),
                ("Log Types", "log_types"),
                ("Warn Expire Time", "warn_expire_time")
            ])])
    )).await?;

    if let Some(interaction) = await_interaction(&ctx, &interaction.message, "moderation_config").await {
        match get_selected_value(&interaction)?.as_str() {
            "log_channel" => edit_log_channel(ctx, interaction).await?,
            "log_types" => edit_log_types(ctx, interaction).await?,
            "warn_expire_time" => edit_warn_expire_time(ctx, interaction).await?,
            _ => {}
        }
    }

    Ok(())
}

async fn create_core_config_embed(guild_table: GuildSettings) -> CreateEmbed<'static> {
    CreateEmbed::new()
        .title("Core Config")
        .color(BotColors::Default.color())
        .field("Bot Language", guild_table.lang, false)
}

async fn core_config(ctx: Context<'_>, interaction: ComponentInteraction) -> Result<(), BotError> {
    use crate::database::schema::guild_settings::dsl::*;

    let guild_table = ctx.data().db.run(|conn| {
        guild_settings
            .filter(guild_id.eq(ctx.guild_id().unwrap().get() as i64))
            .first::<GuildSettings>(conn)
    }).await?;

    interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::default()
            .embed(create_core_config_embed(guild_table.clone()).await)
            .components(vec![create_select_menu("core_config", vec![
                ("Bot Language", "bot_language"),
            ])])
    )).await?;

    if let Some(interaction) = await_interaction(&ctx, &interaction.message, "core_config").await {
        if get_selected_value(&interaction)?.as_str() == "bot_language" { edit_bot_language(ctx, interaction).await? }
    }


    Ok(())
}

fn create_bot_language_select_menu(guild_table: GuildSettings) -> CreateActionRow<'static> {
    let select_menu = CreateSelectMenu::new("bot_language", CreateSelectMenuKind::String {
        options: Cow::Owned(vec![
            CreateSelectMenuOption::new("English", "en").default_selection(guild_table.lang == "en"),
            CreateSelectMenuOption::new("Polish", "pl").default_selection(guild_table.lang == "pl"),
        ])
    });
    CreateActionRow::SelectMenu(select_menu)
}


async fn edit_bot_language(ctx: Context<'_>, interaction: ComponentInteraction) -> Result<(), BotError> {
    use crate::database::schema::guild_settings::dsl::*;

    
    let guild_table = ctx.data().db.run(|conn| {
        guild_settings
            .filter(guild_id.eq(ctx.guild_id().unwrap().get() as i64))
            .first::<GuildSettings>(conn)
    }).await?;

    interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::default()
            .embed(create_log_channel_config_embed())
            .components(vec![create_bot_language_select_menu(guild_table.clone())])
    )).await?;
    
    if let Some(interaction) = await_interaction(&ctx, &interaction.message, "bot_language").await {
        let selected_lang = get_selected_value(&interaction)?;
        let selected_lang_clone = selected_lang.clone();
        
        ctx.data().db.run(move |conn| {
            diesel::update(guild_settings.filter(guild_id.eq(ctx.guild_id().unwrap().get() as i64)))
                .set(lang.eq(selected_lang))
                .execute(conn)
        }).await?;
        
        interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::default()
                .embed(CreateEmbed::new()
                    .title("Bot Language Set")
                    .description(format!("Bot language set to: {}", selected_lang_clone))
                )
                .components(vec![])
        )).await?;
    }

    Ok(())
}

async fn edit_log_channel(ctx: Context<'_>, interaction: ComponentInteraction) -> Result<(), BotError> {
    use crate::database::schema::logs::dsl::*;

    let logs_table = ctx.data().db.run(|conn| {
        logs
            .filter(guild_id.eq(ctx.guild_id().unwrap().get() as i64))
            .first::<Logs>(conn)
    }).await?;

    interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::default()
            .embed(create_log_channel_config_embed())
            .components(vec![create_log_channel_select_menu(logs_table.clone())])
    )).await?;


    let mut new_logs = logs_table.clone();

    if let Some(interaction) = await_interaction(&ctx, &interaction.message, "log_channel").await {
        let selected_channel = ChannelId::new(get_selected_value(&interaction)?.parse()?);

        new_logs.default_log_channel = Option::from(selected_channel.get() as i64);

        interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::default()
                .embed(CreateEmbed::new()
                    .title("Log Channel Set")
                    .description(format!("Log channel set to <#{}>", selected_channel.get())))
                .components(vec![])
        )).await?;

        ctx.data().db.run(move |conn| {
            diesel::update(logs.filter(guild_id.eq(ctx.guild_id().unwrap().get() as i64)))
                .set(default_log_channel.eq(new_logs.default_log_channel))
                .execute(conn)
        }).await?;


        selected_channel.say(ctx.http(), "This channel has been set as the log channel").await?;
    }

    Ok(())
}

fn get_selected_values(interaction: &ComponentInteraction) -> Result<Vec<String>, BotError> {
    match &interaction.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => Ok(Vec::from(values.clone())),
        _ => Err(BotError::from("Invalid interaction data kind"))
    }
}


async fn edit_log_types(ctx: Context<'_>, interaction: ComponentInteraction) -> Result<(), BotError> {
    use crate::database::schema::logs::dsl::*;

    let logs_table = ctx.data().db.run(|conn| {
        logs
            .filter(guild_id.eq(ctx.guild_id().unwrap().get() as i64))
            .first::<Logs>(conn)
    }).await?;

    let active_log_types = get_active_log_types(logs_table.log_types as u32);

    interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::default()
            .embed(CreateEmbed::new()
                .title("Log Types")
                .description("Select the log types you want to enable")
                .color(BotColors::Default.color())
                .field("Active Log Types", active_log_types.join(", "), false)
                .field("Log Types", "Clear Messages, Clear Channel, Mute, Unmute, Kick, Lock, Unlock, Ban, Unban, Warn, Remove Warn, Remove Multiple Warns", false)
            )
            .components(vec![CreateActionRow::SelectMenu(
                CreateSelectMenu::new("log_types", CreateSelectMenuKind::String {
                    options: Cow::Owned(vec![
                        CreateSelectMenuOption::new("Clear Messages", "1").default_selection(active_log_types.contains(&LogType::ClearMessages.to_string())),
                        CreateSelectMenuOption::new("Clear Channel", "2").default_selection(active_log_types.contains(&LogType::ClearChannel.to_string())),
                        CreateSelectMenuOption::new("Mute", "4").default_selection(active_log_types.contains(&LogType::Mute.to_string())),
                        CreateSelectMenuOption::new("Unmute", "8").default_selection(active_log_types.contains(&LogType::Unmute.to_string())),
                        CreateSelectMenuOption::new("Kick", "16").default_selection(active_log_types.contains(&LogType::Kick.to_string())),
                        CreateSelectMenuOption::new("Lock", "32").default_selection(active_log_types.contains(&LogType::Lock.to_string())),
                        CreateSelectMenuOption::new("Unlock", "64").default_selection(active_log_types.contains(&LogType::Unlock.to_string())),
                        CreateSelectMenuOption::new("Ban", "128").default_selection(active_log_types.contains(&LogType::Ban.to_string())),
                        CreateSelectMenuOption::new("Unban", "256").default_selection(active_log_types.contains(&LogType::Unban.to_string())),
                        CreateSelectMenuOption::new("Warn", "512").default_selection(active_log_types.contains(&LogType::Warn.to_string())),
                        CreateSelectMenuOption::new("Remove Warn", "1024").default_selection(active_log_types.contains(&LogType::RemoveWarn.to_string())),
                        CreateSelectMenuOption::new("Remove Multiple Warns", "2048").default_selection(active_log_types.contains(&LogType::RemoveMultipleWarns.to_string())),
                    ])
                }).max_values(12)
            )])
    )).await?;

    if let Some(interaction) = await_interaction(&ctx, &interaction.message, "log_types").await {
        let selected_types: i32 = get_selected_values(&interaction)?.iter().map(|s| s.parse::<i32>().unwrap()).sum();

        ctx.data().db.run(move |conn| {
            diesel::update(logs.filter(guild_id.eq(ctx.guild_id().unwrap().get() as i64)))
                .set(log_types.eq(selected_types))
                .execute(conn)
        }).await?;

        interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::default()
                .embed(CreateEmbed::new()
                    .title("Log Types Set")
                    .description(format!("Log types set to: {}", get_active_log_types(selected_types as u32).join(", ")))
                )
                .components(vec![])
        )).await?;
    }

    Ok(())
}
async fn edit_warn_expire_time(ctx: Context<'_>, interaction: ComponentInteraction) -> Result<(), BotError> {
    use crate::database::schema::moderation_settings::dsl::*;

    let moderation_table = ctx.data().db.run(|conn| {
        moderation_settings
            .filter(guild_id.eq(ctx.guild_id().unwrap().get() as i64))
            .first::<ModerationSettings>(conn)
    }).await?;

    interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::default()
            .embed(create_warn_expire_time_config_embed())
            .components(vec![create_warn_expire_time_select_menu(moderation_table.clone())])
    )).await?;

    if let Some(interaction) = await_interaction(&ctx, &interaction.message, "warn_expire_time").await {
        let selected_days = get_selected_value(&interaction)?;

        let days = if selected_days == "custom" {
            let response = WarnExpireTimeModal::create(
                Some(WarnExpireTimeModal { warn_expire_custom_time: moderation_table.warn_expire_time.to_string() }),
                "warn_expire_custom_time_modal".parse()?
            );
            interaction.create_response(ctx.http(), response).await?;

            if let Some(modal_interaction) = await_modal_interaction(&ctx, &interaction.message, "warn_expire_custom_time_modal").await {
                let custom_days = get_modal_value(&modal_interaction)?.parse::<i64>()?;
                if custom_days < 3 {
                    modal_interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::default()
                            .embed(CreateEmbed::new()
                                .title("Invalid Custom Input")
                                .description("The number of days must be at least 3"))
                            .components(vec![])
                    )).await?;
                    return Ok(());
                }

                modal_interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::default()
                        .embed(CreateEmbed::new()
                            .title("Warn Expire Time Set")
                            .description(format!("Warn expire time set to {} days", custom_days)))
                        .components(vec![])
                )).await?;

                custom_days
            } else {
                return Ok(());
            }
        } else {
            selected_days.parse()?
        };

        ctx.data().db.run(move |conn| {
            diesel::update(moderation_settings.filter(guild_id.eq(ctx.guild_id().unwrap().get() as i64)))
                .set(warn_expire_time.eq(days))
                .execute(conn)
        }).await?;


    }

    Ok(())
}

#[derive(Debug, Modal)]
#[name = "Warn Expire Time"]
struct WarnExpireTimeModal {
    #[name = "Enter a number of days, min. 3"]
    #[placeholder = "e.g 30"]
    warn_expire_custom_time: String,
}