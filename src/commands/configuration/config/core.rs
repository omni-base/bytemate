
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poise::serenity_prelude::{ComponentInteraction, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage};
use crate::{BotError, Context};
use crate::database::models::GuildSettings;
use crate::localization::manager::{Language, TranslationParam};
use crate::util::color::BotColors;
use crate::util::interaction::{await_interaction, create_select_menu, create_select_menu_with_default, get_selected_value};

pub async fn handle_core_config(ctx: Context<'_>, interaction: ComponentInteraction) -> Result<(), BotError> {
    let guild_table = fetch_guild_settings(ctx).await;

    let placeholder = ctx.data().localization_manager.get("commands.configuration.config.placeholder.option", guild_table.lang.to_string().parse().unwrap(), &[]).await;

    let bot_language = ctx.data().localization_manager.get("commands.configuration.core.bot_language.display_name", guild_table.lang.to_string().parse().unwrap(), &[]).await;

    let translated_lang_name = ctx.data().localization_manager.get_translated_lang_name(Language::from_str(&guild_table.lang).unwrap()).await;


    let embed_title = ctx.data().localization_manager.get("commands.configuration.config.embed_title", guild_table.lang.to_string().parse().unwrap(), &[]).await;

    interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::default()
            .embed(create_core_config_embed(&guild_table, embed_title, bot_language.clone(), translated_lang_name))
            .components(vec![create_select_menu("core_config", vec![
                (&*bot_language, "bot_language"),
            ], placeholder.as_str())])
    )).await?;

    if let Some(interaction) = await_interaction(&ctx, &interaction.message, "core_config").await {
        if get_selected_value(&interaction)? == "bot_language" {
            edit_bot_language(ctx, interaction, guild_table).await?;
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

fn create_core_config_embed(guild_table: &GuildSettings, embed_title: String, bot_language: String, bot_language_value: String) -> CreateEmbed {
    CreateEmbed::new()
        .title(embed_title)
        .color(BotColors::Default.color())
        .field(bot_language, bot_language_value, false)
}


async fn edit_bot_language(ctx: Context<'_>, interaction: ComponentInteraction, guild_settings: GuildSettings) -> Result<(), BotError> {
    let pl = ctx.data().localization_manager.get("languages.pl", guild_settings.lang.to_string().parse().unwrap(), &[]).await;
    let en = ctx.data().localization_manager.get("languages.en", guild_settings.lang.to_string().parse().unwrap(), &[]).await;

    let before_embed_title = ctx.data().localization_manager.get("commands.configuration.core.bot_language.set.title", guild_settings.lang.to_string().parse().unwrap(), &[]).await;
    let before_embed_description = ctx.data().localization_manager.get("commands.configuration.core.bot_language.set.description", guild_settings.lang.to_string().parse().unwrap(), &[]).await;

    let placeholder = ctx.data().localization_manager.get("commands.configuration.core.bot_language.set.placeholder", guild_settings.lang.to_string().parse().unwrap(), &[]).await;

    interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::default()
            .embed(CreateEmbed::new().title(before_embed_title).description(before_embed_description).color(BotColors::Default.color()))
            .components(vec![create_select_menu_with_default("bot_language", vec![
                (&*pl, "pl", guild_settings.lang == "pl"),
                (&*en, "en", guild_settings.lang == "en")
            ], placeholder.as_str())])
    )).await?;
    if let Some(interaction) = await_interaction(&ctx, &interaction.message, "bot_language").await {
        let selected_lang = get_selected_value(&interaction)?;
        update_bot_language(&ctx, selected_lang.clone()).await?;
        
        let after_embed_title = ctx.data().localization_manager.get("commands.configuration.core.bot_language.done.title", selected_lang.to_string().parse().unwrap(), &[]).await;

        let translated_lang_name = ctx.data().localization_manager.get_translated_lang_name(Language::from_str(&selected_lang).unwrap()).await;

        let after_embed_description = ctx.data().localization_manager.get("commands.configuration.core.bot_language.done.description", selected_lang.to_string().parse().unwrap(), &[
            TranslationParam::from(translated_lang_name)
        ]).await;


        
        interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::default()
                .embed(CreateEmbed::new()
                    .title(after_embed_title)
                    .description(after_embed_description)
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