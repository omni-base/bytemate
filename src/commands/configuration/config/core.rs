use std::sync::Arc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poise::serenity_prelude::{ComponentInteraction, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage};
use crate::{BotError, Context};
use crate::database::models::GuildSettings;
use crate::localization::manager::{Language, LocalizationManager, TranslationParam};
use crate::util::color::BotColors;
use crate::util::interaction::{await_interaction, create_select_menu, create_select_menu_with_default, get_selected_value};

pub async fn handle_core_config(ctx: Context<'_>, interaction: ComponentInteraction) -> Result<(), BotError> {
    let guild_table = fetch_guild_settings(ctx).await;
    let locales = ctx.data().localization_manager.clone();

    let lang = Language::from_str(&guild_table.lang).unwrap();
    
    interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::default()
            .embed(create_core_config_embed(&guild_table, lang, locales.clone()))
            .components(vec![create_select_menu("core_config", vec![
                (&*locales.get("commands.configuration.core.bot_language.display_name", lang, &[]), "bot_language"),
            ], &*locales.get("commands.configuration.config.placeholder.option", lang, &[]))])
    )).await?;

    if let Some(interaction) = await_interaction(&ctx, &interaction.message, "core_config").await {
        if get_selected_value(&interaction)? == "bot_language" {
            edit_bot_language(ctx, locales, interaction, guild_table).await?;
        }
    }

    Ok(())
}

async fn fetch_guild_settings(ctx: Context<'_>) -> GuildSettings {
    use crate::database::schema::guild_settings::dsl::*;

    ctx.data().db.run(|conn| {
        guild_settings
            .filter(guild_id.eq(ctx.guild_id().unwrap().get() as i64))
            .first::<GuildSettings>(conn)
    }).await.unwrap()
}

fn create_core_config_embed(guild_table: &GuildSettings, lang: Language, locales: Arc<LocalizationManager>) -> CreateEmbed {
    CreateEmbed::new()
        .title(locales.get("commands.configuration.config.embed_title", lang, &[]))
        .color(BotColors::Default.color())
        .field(locales.get("commands.configuration.core.bot_language.display_name", lang, &[]), locales.get_translated_lang_name(lang), false)
}


async fn edit_bot_language(ctx: Context<'_>, locales: Arc<LocalizationManager>, interaction: ComponentInteraction, guild_settings: GuildSettings) -> Result<(), BotError> {
    interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::default()
            .embed(CreateEmbed::new()
                .title(locales.get("commands.configuration.core.bot_language.set.title", Language::from_str(&guild_settings.lang).unwrap(), &[]))
                .description(locales.get("commands.configuration.core.bot_language.set.description", Language::from_str(&guild_settings.lang).unwrap(), &[])).color(BotColors::Default.color()))
            .components(vec![create_select_menu_with_default("bot_language", vec![
                (&*locales.get("languages.pl", Language::from_str(&guild_settings.lang).unwrap(), &[]), "pl", guild_settings.lang == "pl"),
                (&*locales.get("languages.en", Language::from_str(&guild_settings.lang).unwrap(), &[]), "en", guild_settings.lang == "en")
            ], &*locales.get("commands.configuration.core.bot_language.set.placeholder", Language::from_str(&guild_settings.lang).unwrap(), &[]))])
    )).await?;
    if let Some(interaction) = await_interaction(&ctx, &interaction.message, "bot_language").await {
        let selected_lang = get_selected_value(&interaction)?;
        update_bot_language(&ctx, selected_lang.clone()).await?;
        
        interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::default()
                .embed(CreateEmbed::new()
                    .title(locales.get("commands.configuration.core.bot_language.done.title", Language::from_str(&selected_lang).unwrap(), &[]))
                    .description(ctx.data().localization_manager.get("commands.configuration.core.bot_language.done.description", Language::from_str(&selected_lang).unwrap(), &[
                        TranslationParam::from(locales.get_translated_lang_name(Language::from_str(&selected_lang).unwrap()))
                    ]))
                    .color(BotColors::Default.color())
                )
                .components(vec![])
        )).await?;
    }

    Ok(())
}

async fn update_bot_language(ctx: &Context<'_>, new_lang: String) -> Result<(), BotError> {
    use crate::database::schema::guild_settings::dsl::*;

    ctx.data().db.run(move |conn| {
        diesel::update(guild_settings.filter(guild_id.eq(ctx.guild_id().unwrap().get() as i64)))
            .set(lang.eq(new_lang.clone()))
            .execute(conn)
    }).await?;

    Ok(())
}