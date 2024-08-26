use std::borrow::Cow;
use std::time::Duration;
use poise::serenity_prelude::{ActionRowComponent, ComponentInteraction, ComponentInteractionDataKind, CreateActionRow, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption, InputText, Message, ModalInteraction};
use poise::serenity_prelude::small_fixed_array::FixedArray;
use crate::{BotError, Context};

pub(crate) const INTERACTION_TIMEOUT: Duration = Duration::from_secs(60);


pub fn create_select_menu<'a>(custom_id: &'a str, options: Vec<(&'a str, &'a str)>, placeholder: &'a str) -> CreateActionRow<'a> {
    CreateActionRow::SelectMenu(
        CreateSelectMenu::new(custom_id, CreateSelectMenuKind::String {
            options: Cow::Owned(options.into_iter().map(|(name, value)|
                CreateSelectMenuOption::new(name, value)).collect())
        })
            .placeholder(placeholder)
    )
}

pub fn create_select_menu_with_default<'a>(custom_id: &'a str, options: Vec<(&'a str, &'a str, bool)>, placeholder: &'a str) -> CreateActionRow<'a> {
    CreateActionRow::SelectMenu(
        CreateSelectMenu::new(custom_id, CreateSelectMenuKind::String {
            options: Cow::Owned(options.into_iter().map(|(name, value, default)|
                CreateSelectMenuOption::new(name, value).default_selection(default)).collect())
        })
            .placeholder(placeholder)
    )
}


pub async fn await_interaction(ctx: &Context<'_>, message: &Message, custom_id: &str) -> Option<ComponentInteraction> {
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

pub async fn await_modal_interaction(ctx: &Context<'_>, message: &Message, custom_id: &str) -> Option<ModalInteraction> {
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

pub fn get_selected_value(interaction: &ComponentInteraction) -> Result<String, BotError> {
    match &interaction.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => Ok(values[0].clone()),
        ComponentInteractionDataKind::ChannelSelect { values } => Ok(values[0].get().to_string().clone()),
        _ => Err(BotError::from("Invalid interaction data kind"))
    }
}

pub fn get_modal_value(interaction: &ModalInteraction) -> Result<String, BotError> {
    match &interaction.data.components[0].components[0] {
        ActionRowComponent::InputText { 0: InputText { value, .. } } => Ok(value.clone().unwrap().to_string()),
        _ => Err(Box::from("Invalid component")),
    }
}