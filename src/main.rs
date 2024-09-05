
#![feature(async_closure)]
#![feature(duration_constructors)]

use std::{fs};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool};
use std::time::Duration;
use axum::{Json, Router};
use axum::routing::get;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{Command, Settings, UserId};
use serde::{Deserialize, Serialize};
use crate::commands::configuration::config;
use crate::commands::utils::*;
use crate::commands::moderation::*;
use crate::database::manager::DbManager;
use crate::events::handle_event;
use crate::localization::manager::{Language, LocalizationManager};

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
mod events;

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
            event_handler: |framework, event| Box::pin(handle_event(framework, event)),
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

    let mut settings = Settings::default();
    settings.max_messages = 1000;
    
    
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
        .cache_settings(settings)
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
