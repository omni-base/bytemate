
#![feature(async_closure)]
#![feature(duration_constructors)]

use std::{fs};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use axum::{Json, Router};
use axum::routing::get;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{CacheHttp, Command, UserId};
use serde::{Deserialize, Serialize};
use crate::commands::configuration::config;
use crate::commands::utils::*;
use crate::commands::moderation::*;
use crate::database::manager::DbManager;
use crate::localization::manager::{Language, LocalizationManager};
use crate::modules::moderation::notifications::notification_loop;

pub mod database {
    pub mod schema;
    pub mod models;

    pub mod manager;

    pub mod upsert;
}

pub mod util {
    
    pub mod color;
    pub mod time;
    pub mod timestamp;

    pub mod interaction;
}

pub mod modules {
    pub mod moderation {
        pub mod logs;
        pub mod notifications;
    }
}

pub mod localization {
    pub mod manager;
}
pub mod commands;



#[derive(Deserialize)]
struct Config {
    token: String,
    database_url: String,
}



pub struct Data {
    pub has_started: AtomicBool,
    pub db: Arc<DbManager>,
    pub localization_manager: Arc<LocalizationManager>,
    pub global_commands: Arc<RwLock<Vec<Command>>>,
    pub client_id: Arc<RwLock<UserId>>,
}
pub type BotError = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, BotError>;




#[tokio::main]
async fn main() {
    let config: Config = serde_yaml::from_str(&fs::read_to_string("config.yaml").unwrap()).unwrap();
    
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            event_handler: |framework, event| Box::pin(event_handler(framework, event)),
            commands: vec![
                ban::ban(), kick::kick(), mute::mute(), unmute::unmute(),
                help::help(), cases::cases(), clear::clear(), channel::channel(),
                warn::warn(),
                config::config(),
            ],
            ..Default::default()
        })
        .build();


    let db = Arc::new(DbManager::new(&config.database_url).await.unwrap());

    let mut client = serenity::ClientBuilder::new(&config.token, serenity::GatewayIntents::all())
        .framework(framework)
        .activity(poise::serenity_prelude::ActivityData::custom("𝗜'𝗠 𝗧𝗛𝗘 𝗠𝗘𝗢𝗪 𝗠𝗢𝗗𝗘𝗥𝗔𝗧𝗢𝗥"))
        .data(Arc::new(Data {
            has_started: AtomicBool::new(false),
            db,
            localization_manager: Arc::new(LocalizationManager::new(Language::English, PathBuf::from("translations_cache.bin"), Duration::from_secs(24 * 60 * 60)).unwrap()),
            global_commands: Arc::new(RwLock::new(Vec::new())),
            client_id: Arc::new(RwLock::new(UserId::default())),
        }) as _)
        .await.unwrap();

    

    let ctx = client.cache.clone();

    tokio::task::spawn(async move {
        create_api_server(ctx).await;
    });


    client.start().await.unwrap();
}

async fn create_api_server(ctx: Arc<serenity::Cache>) {
    #[derive(Serialize)]
    struct GuildIds(Vec<serenity::GuildId>);

    let app = Router::new()
        .route("/", get(|| async { "test" }))
        .route("/api/__botguilds", get(move || async move {
            let guilds = ctx.guilds();
            Json(GuildIds(guilds.into_iter().collect()))
        }));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:2137").await.unwrap();
    println!("API server listening on: {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();

}

async fn event_handler(
    framework: poise::FrameworkContext<'_, Data, BotError>,
    event: &serenity::FullEvent,
) -> Result<(), BotError> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
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
        },
        serenity::FullEvent::Message { new_message } => {
            let client_id = framework.user_data().client_id.read().unwrap().to_string();
            if new_message.content.contains(&format!("<@{}>", client_id)) {
                new_message.reply(framework.serenity_context.http(), "Hello!").await?;
            }
        },
        serenity::FullEvent::CacheReady { guilds} => {
            let data = framework.user_data();
            let db = data.db.clone();
            database::upsert::upsert_database(db, guilds).await?;
        },
        serenity::FullEvent::GuildCreate { guild, is_new } => {
            let data = framework.user_data();
            let db = data.db.clone();
            if is_new.expect("Expected a boolean value for is_new") {
                let guild_ids = vec![guild.id];
                database::upsert::upsert_database(db, &guild_ids).await?;
            }
        }
        _ => {}
    }
    Ok(())
}