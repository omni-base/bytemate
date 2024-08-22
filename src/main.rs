#![feature(trivial_bounds)]
#![feature(async_closure)]
#![feature(duration_constructors)]

use std::{env, fs};
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};

use axum::{Json, Router};
use axum::routing::get;
use diesel::{Connection, PgConnection};
use diesel_async::{AsyncConnection, AsyncPgConnection};
use dotenvy::dotenv;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{CacheHttp, Command, UserId};
use serde::{Deserialize, Serialize};
use crate::commands::utils::*;
use crate::commands::moderation::*;
use crate::database::manager::DbManager;
use crate::modules::moderation::notifications::notification_loop;

pub mod database {
    pub mod schema;
    pub mod models;
    
    pub mod manager;
}

pub mod util {
    pub mod util;
    
    pub mod color;
    pub mod time;
    pub mod timestamp;
}

pub mod modules {
    pub mod moderation {
        pub mod logs;
        pub mod notifications;
    }
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
            ],
            ..Default::default()
        })
        .build();


    let db = Arc::new(DbManager::new(&config.database_url).await.unwrap()); 
    
    let mut client = serenity::ClientBuilder::new(&config.token, serenity::GatewayIntents::all())
        .framework(framework)
        .activity(poise::serenity_prelude::ActivityData::custom("If you could fly a plane to Pluto, the trip would take more than 800 years!"))
        .data(Arc::new(Data {
            has_started: AtomicBool::new(false),
            db,
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
    if let serenity::FullEvent::Ready { data_about_bot, .. } = event {
        let data = framework.user_data();
        let ctx = Arc::new(framework.serenity_context.clone());
        
        if !data.has_started.load(Ordering::Relaxed) {
            let commands = &framework.options().commands;
            poise::builtins::register_globally(ctx.http(), commands).await?;
            println!("Successfully registered slash commands!");

            let global_commands = ctx.http().get_global_commands().await?;
            *data.global_commands.write().unwrap() = global_commands;

            data.has_started.store(true, Ordering::Relaxed);
            *data.client_id.write().unwrap() = data_about_bot.user.id;
            println!("Logged in as {}", data_about_bot.user.name);

            let data_about_bot = data_about_bot.clone();
            
            tokio::spawn(async move { notification_loop(data.clone(), ctx, data_about_bot).await });
        }
    } else if let serenity::FullEvent::Message { new_message} = event {
        
        if new_message.content.contains(format!("<@{}>", framework.user_data().client_id.read().unwrap()).as_str()) {
            new_message.reply(framework.serenity_context.http(), "Hello!").await?;
        }
    }
    Ok(())
}