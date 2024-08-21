use std::error;
use std::sync::Arc;
use std::time::Duration;
use diesel::dsl::now;
use diesel::QueryDsl;
use poise::serenity_prelude::{CacheHttp, Context, GuildId, Ready, UserId};
use crate::Data;

use crate::database::models::*;
use diesel::prelude::*;
use poise::serenity_prelude::nonmax::NonMaxU64;
use crate::modules::moderation::logs::{log_action, LogData, LogType};

async fn unban_check(data: Arc<Data>, ctx: Arc<Context>, data_about_bot: Ready) -> Result<(), Box<dyn error::Error>> {
    use crate::database::schema::cases::dsl::*;

    let mut db_conn = data.db.lock().await;

    let cases_results = cases
        .filter(case_type.eq("BAN"))
        .filter(end_date.lt(now))
        .select(Cases::as_select())
        .load::<Cases>(&mut *db_conn)
        .unwrap();
    
    for case in cases_results {
       let guild = GuildId::new(u64::from(NonMaxU64::try_from(case.guild_id as u64).unwrap()));
        let user = UserId::new(u64::from(NonMaxU64::try_from(case.user_id as u64).unwrap()));
        
        guild.unban(ctx.http(), user, "Ban expired".into()).await?;

        let log_data = LogData {
            ctx: Some(&ctx),
            data: Some(&data),
            guild_id: Some(u64::from(NonMaxU64::try_from(case.guild_id as u64).unwrap())),
            user_id: Some(u64::from(NonMaxU64::try_from(case.user_id as u64).unwrap())),
            moderator_id: Some(data_about_bot.user.id),
            reason: Some("Expired ban".into()),
            case_id: Some(case.case_id),
            ..LogData::default()
        };
        
        log_action(LogType::Unban, log_data).await.unwrap();
        
        let _ = diesel::delete(cases.filter(case_id.eq(case.case_id))).execute(&mut *db_conn);
    }
    
    Ok(())
}


async fn remove_warn_check(data: Arc<Data>, ctx: Arc<Context>, data_about_bot: Ready) -> Result<(), Box<dyn error::Error>> {
    use crate::database::schema::cases::dsl::*;
    
    let mut db_conn = data.db.lock().await;
    
    let cases_results = cases
        .filter(case_type.eq("WARN"))
        .filter(end_date.lt(now))
        .select(Cases::as_select())
        .load::<Cases>(&mut *db_conn)
        .unwrap();
    
    
    
    for case in cases_results {
        let log_data = LogData {
            ctx: Some(&ctx),
            data: Some(&data),
            guild_id: Some(u64::from(NonMaxU64::try_from(case.guild_id as u64).unwrap())),
            user_id: Some(u64::from(NonMaxU64::try_from(case.user_id as u64).unwrap())),
            moderator_id: Some(data_about_bot.user.id),
            reason: Some("Expired warn".into()),
            case_id: Some(case.case_id),
            points: Some(case.points.unwrap()),
            ..LogData::default()
        };

        log_action(LogType::RemoveWarn, log_data).await.unwrap();

        let _ = diesel::delete(cases.filter(case_id.eq(case.case_id))).execute(&mut *db_conn);
    }
    Ok(())
}

pub async fn notification_loop(data: Arc<Data>, ctx: Arc<Context>, data_about_bot: Ready) {
    let data1 = data.clone();
    let ctx1 = ctx.clone();
    let data_about_bot1 = data_about_bot.clone();
    tokio::spawn(async move {
        loop {

            if let Err(why) = unban_check(data.clone(), ctx.clone(), data_about_bot.clone()).await {
                eprintln!("Error checking for unbans: {:?}", why);
            }

            tokio::time::sleep(Duration::from_secs(15)).await;
        }
    });

    tokio::spawn(async move {
        loop {
            if let Err(why) = remove_warn_check(data1.clone(), ctx1.clone(), data_about_bot1.clone()).await {
                eprintln!("Error checking for expired warns: {:?}", why);
            }
            tokio::time::sleep(Duration::from_hours(1)).await;
        }
    });


}