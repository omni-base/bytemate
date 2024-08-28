use std::borrow::Cow;
use std::time::Duration;
use poise::serenity_prelude::{ActionRowComponent, ComponentInteraction, ComponentInteractionDataKind, CreateActionRow, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption, InputText, Message, ModalInteraction};
use poise::serenity_prelude::small_fixed_array::FixedArray;
use crate::{BotError, Context};

pub(crate) const INTERACTION_TIMEOUT: Duration = Duration::from_secs(60);


pub fn create_select_menu<'a, K>(
    custom_id: &'a str,
    options: Vec<(String, String)>,
    placeholder: &'a str,
    kind: K,
) -> CreateActionRow<'a>
where
    K: Into<CreateSelectMenuKind<'a>>,
{
    let options = options.into_iter().map(|(name, value)|
        CreateSelectMenuOption::new(name, value)).collect::<Vec<_>>();

    let kind = match kind.into() {
        CreateSelectMenuKind::String { .. } => CreateSelectMenuKind::String { options: options.into() },
        CreateSelectMenuKind::Channel { channel_types, default_channels } => CreateSelectMenuKind::Channel {
            channel_types,
            default_channels,
        },
        CreateSelectMenuKind::Role { default_roles } => CreateSelectMenuKind::Role { default_roles },
        CreateSelectMenuKind::Mentionable { default_users, default_roles } => CreateSelectMenuKind::Mentionable {
            default_users,
            default_roles,
        },
        CreateSelectMenuKind::User { default_users } => CreateSelectMenuKind::User { default_users },
    };

    CreateActionRow::SelectMenu(
        CreateSelectMenu::new(custom_id, kind)
            .placeholder(placeholder)
    )
}

pub fn create_select_menu_with_default<'a, K>(
    custom_id: &'a str,
    options: Vec<(String, String, bool)>,
    placeholder: &'a str,
    kind: K,
    max_values: Option<u8>,
) -> CreateActionRow<'a>
where
    K: Into<CreateSelectMenuKind<'a>>,
{
    let options = options.into_iter().map(|(name, value, default)|
        CreateSelectMenuOption::new(name, value).default_selection(default)).collect::<Vec<_>>();

    let kind = match kind.into() {
        CreateSelectMenuKind::String { .. } => CreateSelectMenuKind::String { options: options.into() },
        CreateSelectMenuKind::Channel { channel_types, default_channels } => CreateSelectMenuKind::Channel {
            channel_types,
            default_channels,
        },
        CreateSelectMenuKind::Role { default_roles } => CreateSelectMenuKind::Role { default_roles },
        CreateSelectMenuKind::Mentionable { default_users, default_roles } => CreateSelectMenuKind::Mentionable {
            default_users,
            default_roles,
        },
        CreateSelectMenuKind::User { default_users } => CreateSelectMenuKind::User { default_users },
    };

    CreateActionRow::SelectMenu(
        CreateSelectMenu::new(custom_id, kind)
            .placeholder(placeholder)
            .max_values(max_values.unwrap_or(1))
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

pub fn get_selected_values(interaction: &ComponentInteraction) -> Result<Vec<String>, BotError> {
    match &interaction.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => Ok(Vec::from(values.clone())),
        ComponentInteractionDataKind::ChannelSelect { values } => Ok(values.iter().map(|v| v.get().to_string()).collect()),
        _ => Err(BotError::from("Invalid interaction data kind"))
    }
}

pub fn get_modal_value(interaction: &ModalInteraction) -> Result<String, BotError> {
    match &interaction.data.components[0].components[0] {
        ActionRowComponent::InputText { 0: InputText { value, .. } } => Ok(value.clone().unwrap().to_string()),
        _ => Err(Box::from("Invalid component")),
    }
}