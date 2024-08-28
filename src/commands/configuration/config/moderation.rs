use std::borrow::Cow;
use std::sync::Arc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poise::serenity_prelude::{ChannelId, ChannelType, ComponentInteraction, CreateActionRow, CreateEmbed, CreateInputText, CreateInteractionResponse, CreateInteractionResponseMessage, CreateModal, CreateSelectMenuKind, InputTextStyle};
use poise::serenity_prelude::CreateInteractionResponse::Modal;
use strum::IntoEnumIterator;
use crate::{BotError, Context};
use crate::database::models::{ModerationSettings};
use crate::localization::manager::{Language, LocalizationManager, TranslationParam};
use crate::modules::moderation::logs::{get_active_log_types, string_to_log_type, LogType};
use crate::util::color::BotColors;
use crate::util::interaction::{await_interaction, await_modal_interaction, create_select_menu, create_select_menu_with_default, get_modal_value, get_selected_value, get_selected_values};

pub async fn handle_moderation_config(ctx: Context<'_>, interaction: ComponentInteraction) -> Result<(), BotError> {
    let locales = ctx.data().localization_manager.clone();
    let lang = locales.get_guild_language(ctx.data().db.clone(), ctx.guild_id().unwrap()).await.unwrap();

    let moderation_table = fetch_moderation_settings(ctx).await;

    interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::default()
            .embed(create_moderation_config_embed(&moderation_table, lang, locales.clone()))
            .components(vec![create_select_menu("core_config", vec![
                (locales.get("commands.configuration.moderation.warn_expire_time.display_name", lang, &[]), "warn_expire".parse().unwrap()),
                (locales.get("commands.configuration.moderation.default_log_channel.display_name", lang, &[]), "default_log_channel".parse().unwrap()),
                (locales.get("commands.configuration.moderation.log_types.display_name", lang, &[]), "log_types".parse().unwrap()),
            ], &locales.get("commands.configuration.config.placeholder.option", lang, &[]), CreateSelectMenuKind::String { options: Default::default() })])
    )).await?;

    if let Some(interaction) = await_interaction(&ctx, &interaction.message, "core_config").await {
        if get_selected_value(&interaction)? == "warn_expire" {
            edit_warn_expire(ctx, locales, interaction, moderation_table, lang).await?;
        } else if get_selected_value(&interaction)? == "default_log_channel" {
            edit_default_log_channel(ctx, interaction, moderation_table).await?;
        } else if get_selected_value(&interaction)? == "log_types" {
            edit_log_types(ctx, interaction, moderation_table).await?;
        }
    }

    Ok(())
}

async fn fetch_moderation_settings(ctx: Context<'_>) -> ModerationSettings {
    use crate::database::schema::moderation_settings::dsl::*;

    ctx.data().db.run(|conn| {
        moderation_settings
            .filter(guild_id.eq(ctx.guild_id().unwrap().get() as i64))
            .first::<ModerationSettings>(conn)
    }).await.unwrap()
}

fn create_moderation_config_embed(
    moderation_table: &ModerationSettings,
    lang: Language,
    locales: Arc<LocalizationManager>,
) -> CreateEmbed {

    CreateEmbed::new()
        .title(locales.get("commands.configuration.config.embed_title", lang, &[]))
        .color(BotColors::Default.color())
        .field(locales.get("commands.configuration.moderation.warn_expire_time.display_name", lang, &[]), locales.get("commands.configuration.moderation.warn_expire_time.value", lang, &[
            TranslationParam::from(moderation_table.warn_expire_time.to_string())
        ]), false)
        .field(locales.get("commands.configuration.moderation.default_log_channel.display_name", lang, &[]), moderation_table.default_log_channel.map_or(locales.get("commands.configuration.config.none", lang, &[]), |id| format!("<#{}>", id)), false)
        .field(locales.get("commands.configuration.moderation.log_types.display_name", lang, &[]), get_active_log_types(moderation_table.log_types as u32, &locales.clone(), lang).iter().map(|log_type| log_type.to_string()).collect::<Vec<_>>().join(", "), false)
}


async fn edit_warn_expire(ctx: Context<'_>, locales: Arc<LocalizationManager>, interaction: ComponentInteraction, mod_table: ModerationSettings, lang: Language) -> Result<(), BotError> {

    let days_text = locales.get("commands.configuration.moderation.warn_expire_time.days", lang, &[]);
    let custom_text = locales.get("commands.configuration.moderation.warn_expire_time.custom", lang, &[]);

    let mut options: Vec<(String, String, bool)> = vec![
        {
            let text = format!("3 {}", days_text);
            (text, "3".to_string(), mod_table.warn_expire_time == 3)
        },
        {
            let text = format!("7 {}", days_text);
            (text, "7".to_string(), mod_table.warn_expire_time == 7)
        },
        {
            let text = format!("14 {}", days_text);
            (text, "14".to_string(), mod_table.warn_expire_time == 14)
        },
        {
            let text = format!("30 {}", days_text);
            (text, "30".to_string(), mod_table.warn_expire_time == 30)
        },
        (custom_text, "custom".to_string(), false),
    ];


    let custom_label = format!("{} days", mod_table.warn_expire_time);

    if mod_table.warn_expire_time != 3 && mod_table.warn_expire_time != 7 && mod_table.warn_expire_time != 14 && mod_table.warn_expire_time != 30 {
        options.insert(5, (
            custom_label,
            "other".parse().unwrap(),
            true
        ));
    }

    interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::default()
            .embed(CreateEmbed::new()
                .title(locales.get("commands.configuration.moderation.warn_expire_time.set.title", lang, &[]))
                .description(locales.get("commands.configuration.moderation.warn_expire_time.set.description", lang, &[])).color(BotColors::Default.color()))
            .components(vec![create_select_menu_with_default(
                "warn_expire_time",
                options,
                &locales.get("commands.configuration.moderation.warn_expire_time.set.placeholder", lang, &[]),
                CreateSelectMenuKind::String { options: Default::default() }, None
            )])
    )).await?;
    if let Some(interaction) = await_interaction(&ctx, &interaction.message, "warn_expire_time").await {
        let selected_days = get_selected_value(&interaction)?;


        if selected_days == "custom" {
            let input_text = CreateInputText::new(InputTextStyle::Short, locales.get("commands.configuration.moderation.warn_expire_time.set.custom.placeholder", lang, &[]), "days")
                .placeholder(locales.get("commands.configuration.moderation.warn_expire_time.set.custom.placeholder", lang, &[]))
                .min_length(1)
                .max_length(3);

            let action_row = CreateActionRow::InputText(input_text);

            let modal = CreateModal::new("custom_warn_expire_time", locales.get("commands.configuration.moderation.warn_expire_time.set.custom.title", lang, &[]))
                .components(vec![action_row]);

            interaction.create_response(ctx.http(), Modal(modal)).await?;

            if let Some(interaction) = await_modal_interaction(&ctx, &interaction.message, "custom_warn_expire_time").await {
                let selected_days = get_modal_value(&interaction)?;

                if let Ok(days) = selected_days.parse::<i64>() {
                    if days >= 3 {
                        update_warn_expire_time(&ctx, days).await?;
                    } else {
                        interaction.create_response(ctx.http(), CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::default()
                                .content(locales.get("commands.configuration.moderation.warn_expire_time.set.custom.error_too_low", lang, &[]))
                        )).await?;
                        return Ok(());
                    }
                } else {
                    interaction.create_response(ctx.http(), CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::default()
                            .content(locales.get("commands.configuration.moderation.warn_expire_time.set.custom.error_invalid", lang, &[]))
                    )).await?;
                    return Ok(());
                }

                update_warn_expire_time(&ctx, selected_days.parse().unwrap()).await?;


                interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::default()
                        .embed(CreateEmbed::new()
                            .title(locales.get("commands.configuration.moderation.warn_expire_time.done.title", lang, &[]))
                            .description(ctx.data().localization_manager.get("commands.configuration.moderation.warn_expire_time.done.description", lang, &[
                                TranslationParam::from(selected_days)
                            ]))
                            .color(BotColors::Default.color())
                        )
                        .components(vec![])
                )).await?;
            }
        } else {
            update_warn_expire_time(&ctx, selected_days.parse().unwrap()).await?;
            
            interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::default()
                    .embed(CreateEmbed::new()
                        .title(locales.get("commands.configuration.moderation.warn_expire_time.done.title", lang, &[]))
                        .description(locales.get("commands.configuration.moderation.warn_expire_time.done.description", lang, &[
                            TranslationParam::from(selected_days)
                        ]))
                        .color(BotColors::Default.color())
                    )
                    .components(vec![])
            )).await?;
        }
    }

    Ok(())
}

async fn update_warn_expire_time(ctx: &Context<'_>, warn_time: i64) -> Result<(), BotError> {
    use crate::database::schema::moderation_settings::dsl::*;

    ctx.data().db.run(move |conn| {
        diesel::update(moderation_settings.filter(guild_id.eq(ctx.guild_id().unwrap().get() as i64)))
            .set(warn_expire_time.eq(warn_time))
            .execute(conn)
    }).await?;

    Ok(())
}


async fn edit_default_log_channel(ctx: Context<'_>, interaction: ComponentInteraction, mod_table: ModerationSettings) -> Result<(), BotError> {
    let locales = ctx.data().localization_manager.clone();
    let lang = locales.get_guild_language(ctx.data().db.clone(), ctx.guild_id().unwrap()).await.unwrap();

    interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::default()
            .embed(CreateEmbed::new()
                .title(locales.get("commands.configuration.moderation.default_log_channel.set.title", lang, &[]))
                .description(locales.get("commands.configuration.moderation.default_log_channel.set.description", lang, &[])).color(BotColors::Default.color()))
            .components(vec![create_select_menu_with_default(
                "default_log_channel",
                vec![],
                &locales.get("commands.configuration.moderation.default_log_channel.set.placeholder", lang, &[]),
                CreateSelectMenuKind::Channel {
                    channel_types: Some(Cow::from(vec![ChannelType::Text])),
                    default_channels: mod_table.default_log_channel.map(|id| Cow::Owned(vec![ChannelId::new(id as u64)]))
                }, None
            )])
    )).await?;
    if let Some(interaction) = await_interaction(&ctx, &interaction.message, "default_log_channel").await {
        let selected_channel = get_selected_value(&interaction)?;

        let channel_id = if selected_channel == "none" {
            None
        } else {
            Some(selected_channel.parse::<i64>().unwrap())
        };

        update_default_log_channel(&ctx, channel_id).await?;

        interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::default()
                .embed(CreateEmbed::new()
                    .title(locales.get("commands.configuration.moderation.default_log_channel.done.title", lang, &[]))
                    .description(locales.get("commands.configuration.moderation.default_log_channel.done.description", lang, &[
                        TranslationParam::from(channel_id.map_or("None".to_string(), |id| format!("<#{}>", id)))
                    ]))
                    .color(BotColors::Default.color())
                )
                .components(vec![])
        )).await?;
    }

    Ok(())
}

async fn update_default_log_channel(ctx: &Context<'_>, channel_id: Option<i64>) -> Result<(), BotError> {
    use crate::database::schema::moderation_settings::dsl::*;

    ctx.data().db.run(move |conn| {
        diesel::update(moderation_settings.filter(guild_id.eq(ctx.guild_id().unwrap().get() as i64)))
            .set(default_log_channel.eq(channel_id))
            .execute(conn)
    }).await?;

    Ok(())
}

async fn edit_log_types(ctx: Context<'_>, interaction: ComponentInteraction, mod_table: ModerationSettings) -> Result<(), BotError> {
    let locales = ctx.data().localization_manager.clone();
    let lang = locales.get_guild_language(ctx.data().db.clone(), ctx.guild_id().unwrap()).await.unwrap();

    let mut options: Vec<(String, String, bool)> = vec![];

    let active_log_types = get_active_log_types(mod_table.log_types as u32, &locales.clone(), lang);

    for log_type in LogType::iter() {
        let log_type_string = log_type.to_string(&locales.clone(), lang);

        options.push((
            log_type_string.clone(),
            log_type_string.clone(),
            active_log_types.contains(&log_type_string)
        ));
    }

    let length = options.len() as u8;

    interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::default()
            .embed(CreateEmbed::new()
                .title(locales.get("commands.configuration.moderation.log_types.set.title", lang, &[]))
                .description(locales.get("commands.configuration.moderation.log_types.set.description", lang, &[]))
                .color(BotColors::Default.color()))
            .components(vec![create_select_menu_with_default(
                "log_types",
                options,
                &locales.get("commands.configuration.moderation.log_types.set.placeholder", lang, &[]),
                CreateSelectMenuKind::String { options: Default::default()},
                Some(length)
            )])
    )).await?;

    if let Some(interaction) = await_interaction(&ctx, &interaction.message, "log_types").await {
        let selected_log_types = get_selected_values(&interaction)?;

        let mut log_types = 0u32;
        for log_type in selected_log_types.clone() {
            if let Some(log_type_enum) = string_to_log_type(&log_type, &locales.clone(), lang) {
                log_types |= log_type_enum.as_bit();
            }
        }

        update_log_types(&ctx, log_types as i32).await?;

        interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::default()
                .embed(CreateEmbed::new()
                    .title(locales.get("commands.configuration.moderation.log_types.done.title", lang, &[]))
                    .description(locales.get("commands.configuration.moderation.log_types.done.description", lang, &[
                        TranslationParam::from(selected_log_types.join(", "))
                    ]))
                    .color(BotColors::Default.color())
                )
                .components(vec![])
        )).await?;
    }

    Ok(())
}

async fn update_log_types(ctx: &Context<'_>, types: i32) -> Result<(), BotError> {
    use crate::database::schema::moderation_settings::dsl::*;

    ctx.data().db.run(move |conn| {
        diesel::update(moderation_settings.filter(guild_id.eq(ctx.guild_id().unwrap().get() as i64)))
            .set(log_types.eq(types))
            .execute(conn)
    }).await?;

    Ok(())
}