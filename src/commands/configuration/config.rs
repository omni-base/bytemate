// use std::borrow::Cow;
// use std::time::Duration;
// use crate::{BotError, Context};
// use poise::{command, CreateReply, Modal, send_reply};
// use poise::serenity_prelude::{ActionRowComponent, CacheHttp, ChannelId, ChannelType, ComponentInteraction, ComponentInteractionDataKind, CreateActionRow, CreateEmbed,  CreateInteractionResponse, CreateInteractionResponseMessage, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption, InputText, Message, ModalInteraction};
// use poise::serenity_prelude::small_fixed_array::{FixedArray};
// use crate::database::database_manager::{Logs, LogsManager, ModerationSettings, ModerationSettingsManager, QueryBuilder};
// 
// const INTERACTION_TIMEOUT: Duration = Duration::from_secs(60);
// 
// /// See or modify the current bot configuration
// #[command(slash_command, default_member_permissions = "ADMINISTRATOR", guild_only)]
// pub async fn config(ctx: Context<'_>) -> Result<(), BotError> {
//     let reply = send_reply(ctx, CreateReply::new().embed(create_config_embed()).components(vec![create_select_menu("config_module", vec![("Moderation", "moderation")])])).await?;
//     let message = reply.message().await?;
// 
//     if let Some(interaction) = await_interaction(&ctx, &message, "config_module").await {
//         if get_selected_value(&interaction)? == "moderation" {
//             moderation_config(ctx, interaction).await?
//         }
//     }
// 
//     Ok(())
// }
// 
// fn create_select_menu<'a>(custom_id: &'a str, options: Vec<(&'a str, &'a str)>) -> CreateActionRow<'a> {
//     CreateActionRow::SelectMenu(
//         CreateSelectMenu::new(custom_id, CreateSelectMenuKind::String {
//             options: Cow::Owned(options.into_iter().map(|(name, value)|
//             CreateSelectMenuOption::new(name, value)).collect())
//         })
//             .placeholder("Select an option")
//     )
// }
// 
// fn create_config_embed() -> CreateEmbed<'static> {
//     CreateEmbed::new()
//         .title("Configuration")
//         .description("Select a module to configure")
//         .field("Modules", "1. Moderation", false)
// }
// 
// 
// 
// async fn create_moderation_config_embed(moderation: ModerationSettings, logs: Logs) -> CreateEmbed<'static> {
//     CreateEmbed::new()
//         .title("Moderation Config")
//         .field("Log Channel", logs.default_log_channel.map_or("None".to_string(), |id| format!("<#{}>", id)), false)
//         .field("Warn Expire Time", format!("{:?} days", moderation.warn_expire_time), false)
// }
// 
// fn create_log_channel_config_embed() -> CreateEmbed<'static> {
//     CreateEmbed::new()
//         .title("Set Log Channel")
//         .description("Select a channel to set as the log channel")
// }
// 
// fn create_log_channel_select_menu(logs: Logs) -> CreateActionRow<'static> {
//     let select_menu = CreateSelectMenu::new("log_channel", CreateSelectMenuKind::Channel {
//         channel_types: Some(Cow::Owned(vec![ChannelType::Text])),
//         default_channels: Some(Cow::Owned(vec![logs.default_log_channel.map_or(ChannelId::default(), |id| ChannelId::new(id as u64))])),
//     });
//     CreateActionRow::SelectMenu(select_menu)
// }
// 
// fn create_warn_expire_time_config_embed() -> CreateEmbed<'static> {
//     CreateEmbed::new()
//         .title("Set Warn Expire Time")
//         .description("Select a time for warnings to expire")
// }
// 
// fn create_warn_expire_time_select_menu(moderation: ModerationSettings) -> CreateActionRow<'static> {
//     let current_time = moderation.warn_expire_time;
//     
//     let select_menu = CreateSelectMenu::new("warn_expire_time", CreateSelectMenuKind::String {
//         options: Cow::Owned(vec![
//             CreateSelectMenuOption::new("3 days", "3").default_selection(current_time == 3),
//             CreateSelectMenuOption::new("7 days", "7").default_selection(current_time == 7),
//             CreateSelectMenuOption::new("14 days", "14").default_selection(current_time == 14),
//             CreateSelectMenuOption::new("30 days", "30").default_selection(current_time == 30),
//             CreateSelectMenuOption::new("Custom", "custom").default_selection(current_time != 3 && current_time != 7 && current_time != 14 && current_time != 30),
//         ]),
//     });
// 
//     CreateActionRow::SelectMenu(select_menu)
// } 
// 
// async fn await_interaction(ctx: &Context<'_>, message: &Message, custom_id: &str) -> Option<ComponentInteraction> {
//     let interaction = message.await_component_interaction(ctx.serenity_context().shard.clone())
//         .timeout(INTERACTION_TIMEOUT)
//         .author_id(ctx.author().id)
//         .channel_id(ctx.channel_id())
//         .custom_ids(FixedArray::from([custom_id.parse().unwrap()]))
//         .await;
// 
//     if interaction.is_none() {
//         if let Err(e) = message.reply(ctx.http(), "Timed out. Please try again.").await {
//             eprintln!("Failed to send timeout message: {:?}", e);
//         }
//     }
// 
//     interaction
// }
// 
// async fn await_modal_interaction(ctx: &Context<'_>, message: &Message, custom_id: &str) -> Option<ModalInteraction> {
//     let interaction = message.await_modal_interaction(ctx.serenity_context().shard.clone())
//         .timeout(INTERACTION_TIMEOUT)
//         .author_id(ctx.author().id)
//         .custom_ids(vec![custom_id.to_string().parse().unwrap()])
//         .await;
//     
// 
//     if interaction.is_none() {
//         if let Err(e) = message.reply(ctx.http(), "Timed out. Please try again.").await {
//             eprintln!("Failed to send timeout message: {:?}", e);
//         }
//     }
// 
//     interaction
// }
// 
// fn get_selected_value(interaction: &ComponentInteraction) -> Result<String, BotError> {
//     match &interaction.data.kind {
//         ComponentInteractionDataKind::StringSelect { values } => Ok(values[0].clone()),
//         ComponentInteractionDataKind::ChannelSelect { values } => Ok(values[0].get().to_string().clone()),
//         _ => Err(BotError::from("Invalid interaction data kind"))
//     }
// }
// 
// fn get_modal_value(interaction: &ModalInteraction) -> Result<String, BotError> {
//     match &interaction.data.components[0].components[0] {
//         ActionRowComponent::InputText { 0: InputText { value, .. } } => Ok(value.clone().unwrap().to_string()),
//         _ => Err(Box::from("Invalid component")),
//     }
// }
// 
// 
// 
// 
// async fn moderation_config(ctx: Context<'_>, interaction: ComponentInteraction) -> Result<(), BotError> {
//     let moderation = ctx.data().db.guild_settings().moderation_settings().get(
//         QueryBuilder::new().one().where_clause("guild_id = ?").bind(ctx.guild_id().unwrap().get() as i64)
//     ).await?;
//     
//     let logs = ctx.data().db.guild_settings().moderation_settings().logs().get(
//         QueryBuilder::new().one().where_clause("guild_id = ?").bind(ctx.guild_id().unwrap().get() as i64)
//     ).await?;
//     
// 
//     interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
//         CreateInteractionResponseMessage::default()
//             .embed(create_moderation_config_embed(moderation.clone(), logs.clone()).await)
//             .components(vec![create_select_menu("moderation_config", vec![
//                 ("Log Channel", "log_channel"),
//                 ("Warn Expire Time", "warn_expire_time")
//             ])])
//     )).await?;
// 
//     if let Some(interaction) = await_interaction(&ctx, &interaction.message, "moderation_config").await {
//         match get_selected_value(&interaction)?.as_str() {
//             "log_channel" => log_channel(ctx, interaction).await?,
//             "warn_expire_time" => warn_expire_time(ctx, interaction).await?,
//             _ => {}
//         }
//     }
// 
//     Ok(())
// }
// 
// async fn log_channel(ctx: Context<'_>, interaction: ComponentInteraction) -> Result<(), BotError> {
//     let mut logs = ctx.data().db.guild_settings().moderation_settings().logs().get(
//         QueryBuilder::new().one().where_clause("guild_id = ?").bind(ctx.guild_id().unwrap().get() as i64)
//     ).await?;
// 
//     interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
//         CreateInteractionResponseMessage::default()
//             .embed(create_log_channel_config_embed())
//             .components(vec![create_log_channel_select_menu(logs.clone())])
//     )).await?;
// 
//     if let Some(interaction) = await_interaction(&ctx, &interaction.message, "log_channel").await {
//         let selected_channel = ChannelId::new(get_selected_value(&interaction)?.parse()?);
//     
//         
//         logs.default_log_channel = Some(selected_channel.get() as i64);
//         // logs.create_or_update(&ctx.data().db).await?;
// 
//         interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
//             CreateInteractionResponseMessage::default()
//                 .embed(CreateEmbed::new()
//                     .title("Log Channel Set")
//                     .description(format!("Log channel set to <#{}>", selected_channel.get())))
//                 .components(vec![])
//         )).await?;
// 
//         selected_channel.say(ctx.http(), "This channel has been set as the log channel").await?;
//     }
// 
//     Ok(())
// }
// 
// async fn warn_expire_time(ctx: Context<'_>, interaction: ComponentInteraction) -> Result<(), BotError> {
//     let moderation = ctx.data().db.guild_settings().moderation_settings().get(
//         QueryBuilder::new().one().where_clause("guild_id = ?").bind(ctx.guild_id().unwrap().get() as i64)
//     ).await?;
// 
//     interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
//         CreateInteractionResponseMessage::default()
//             .embed(create_warn_expire_time_config_embed())
//             .components(vec![create_warn_expire_time_select_menu(moderation.clone())])
//     )).await?;
// 
//     if let Some(interaction) = await_interaction(&ctx, &interaction.message, "warn_expire_time").await {
//         let selected_days = get_selected_value(&interaction)?;
// 
//         let days = if selected_days == "custom" {
//             let response = WarnExpireTimeModal::create(
//                 Some(WarnExpireTimeModal { warn_expire_custom_time: moderation.warn_expire_time.to_string() }),
//                 "warn_expire_custom_time_modal".parse()?
//             );
//             interaction.create_response(ctx.http(), response).await?;
// 
//             if let Some(modal_interaction) = await_modal_interaction(&ctx, &interaction.message, "warn_expire_custom_time_modal").await {
//                 let custom_days = get_modal_value(&modal_interaction)?.parse()?;
//                 if custom_days < 3 {
//                     modal_interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
//                         CreateInteractionResponseMessage::default()
//                             .embed(CreateEmbed::new()
//                                 .title("Invalid Custom Input")
//                                 .description("The number of days must be at least 3"))
//                             .components(vec![])
//                     )).await?;
//                     return Ok(());
//                 }
//                 custom_days
//             } else {
//                 return Ok(());
//             }
//         } else {
//             selected_days.parse()?
//         };
//         
//         
//         
//         // moderation.create_or_update(&ctx.data().db).await?;
// 
//         interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(
//             CreateInteractionResponseMessage::default()
//                 .embed(CreateEmbed::new()
//                     .title("Warn Expire Time Set")
//                     .description(format!("Warn expire time set to {} days", days)))
//                 .components(vec![])
//         )).await?;
//     }
// 
//     Ok(())
// }
// 
// #[derive(Debug, Modal)]
// #[name = "Warn Expire Time"]
// struct WarnExpireTimeModal {
//     #[name = "Enter a number of days, min. 3"]
//     #[placeholder = "e.g 30"]
//     warn_expire_custom_time: String,
// }