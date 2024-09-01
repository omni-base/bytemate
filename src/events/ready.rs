use std::sync::Arc;
use std::sync::atomic::Ordering;
use poise::serenity_prelude::{CacheHttp, Ready};
use crate::{BotError, Data};
use crate::modules::moderation::notifications::notification_loop;

pub async fn handle(
    framework: poise::FrameworkContext<'_, Data, BotError>,
    data_about_bot: &Ready,
) -> Result<(), BotError> {
    let data = framework.user_data();
    let ctx = Arc::new(framework.serenity_context.clone());

    if !data.has_started.load(Ordering::Relaxed) {
        let commands = &framework.options().commands;
        poise::builtins::register_globally(ctx.http(), commands).await?;
        println!("Successfully registered slash commands!");

        let global_commands = ctx.http().get_global_commands().await?;

        {
            let mut global_commands_lock = data.global_commands.write().unwrap();
            *global_commands_lock = global_commands;
        }


        data.has_started.store(true, Ordering::Relaxed);
        *data.client_id.write().unwrap() = data_about_bot.user.id;
        println!("Logged in as {}", data_about_bot.user.name);

        let data_about_bot = data_about_bot.clone();
        let data_clone = data.clone();
        tokio::spawn(async move { notification_loop(data_clone, ctx, data_about_bot).await });
    }
    Ok(())
}