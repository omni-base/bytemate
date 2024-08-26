use std::borrow::Cow;
use std::time::Duration;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use crate::{BotError, Context};
use poise::{command, send_reply, CreateReply};
use poise::serenity_prelude::CreateEmbed;
use crate::localization::manager::TranslationParam;
use crate::util::color::BotColors;
use crate::util::interaction::{await_interaction, create_select_menu, get_selected_value};

mod core;
mod moderation;


/// See or modify the current bot configuration
#[command(slash_command, default_member_permissions = "ADMINISTRATOR", guild_only)]
pub async fn config(ctx: Context<'_>) -> Result<(), BotError> {
    let db = ctx.data().db.clone();
    let locales = ctx.data().localization_manager.clone();
    let guild_lang = locales
        .get_guild_language(db, ctx.guild_id().unwrap()).await.unwrap();

    let embed_title = locales.get("commands.configuration.config.embed_title", guild_lang, &[]);
    let embed_description = locales.get("commands.configuration.config.embed_description", guild_lang, &[
        TranslationParam::from(locales.get("commands.configuration.config.core", guild_lang, &[])),
        TranslationParam::from(locales.get("commands.configuration.config.moderation", guild_lang, &[]))
    ]);

    let reply = send_reply(ctx, CreateReply::new()
        .embed(create_config_embed(embed_title, embed_description))
        .components(vec![create_select_menu("config_module", vec![
            (&*locales.get("commands.configuration.config.core", guild_lang, &[]), "core"),
            (&*locales.get("commands.configuration.config.moderation", guild_lang, &[]), "moderation")
        ], &*locales.get("commands.configuration.config.placeholder.module", guild_lang, &[]))])
    ).await?;

    let message = reply.message().await?;

    if let Some(interaction) = await_interaction(&ctx, &message, "config_module").await {
        match get_selected_value(&interaction)?.as_str() {
            "core" => core::handle_core_config(ctx, interaction).await?,
            "moderation" => moderation::handle_moderation_config(ctx, interaction).await?,
            _ => {}
        }
    }

    Ok(())
}

fn create_config_embed(embed_title: String, embed_description: String) -> CreateEmbed<'static> {
    CreateEmbed::new()
        .title(embed_title)
        .description(embed_description)
        .color(BotColors::Default.color())
}
