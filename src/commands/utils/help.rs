use futures::stream::{self, StreamExt};
use poise::{command, CreateReply, send_reply};
use poise::serenity_prelude::{CommandOptionType, CreateEmbed};
use crate::{BotError, Context};
use crate::util::color::BotColors;

/// Browse all available commands
#[command(slash_command)]
pub async fn help(
    ctx: Context<'_>,
) -> Result<(), BotError> {
    let embed = custom_help(ctx).await;
    send_reply(ctx, CreateReply::new().embed(embed)).await?;
    Ok(())
}

async fn custom_help(ctx: Context<'_>) -> CreateEmbed {
    let data = ctx.data();
    let unfiltered_cmds = {
        let guard = data.global_commands.read().unwrap();
        guard.clone()
    };
    
    let cmds = filter_commands(&ctx, &unfiltered_cmds).await;
    
    let help_string = cmds.iter()
        .flat_map(|cmd| {
            
            let mut entries = Vec::new();
            if !cmd.options.iter().any(|subcmd| subcmd.kind == CommandOptionType::SubCommand) {
                entries.push(format!("</{}:{}>\n", cmd.name, cmd.id));
            }
            entries.extend(cmd.options.iter()
                .filter(|subcmd| subcmd.kind == CommandOptionType::SubCommand)
                .map(|subcmd| format!("</{} {}:{}>\n", cmd.name, subcmd.name, cmd.id)));
            entries
        })
        .collect::<String>();

    CreateEmbed::default().description(help_string).color(BotColors::Default.color())
}

async fn filter_commands(ctx: &Context<'_>, cmds: &[poise::serenity_prelude::Command]) -> Vec<poise::serenity_prelude::Command> {
    stream::iter(cmds.iter())
        .filter_map(|cmd| {

            async move {
                let has_permissions = stream::iter(&cmd.default_member_permissions)
                    .all(|perm| {
                        async move {
                            ctx.author_member().await.unwrap().permissions(ctx.cache()).unwrap().contains(*perm)
                        }
                    })
                    .await;
                if has_permissions {
                    Some(cmd.clone())
                } else {
                    None
                }
            }
        })
        .collect()
        .await
}