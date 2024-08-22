use chrono::{DateTime, Utc};
use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable, Clone, Debug)]
#[diesel(table_name = crate::database::schema::guild_settings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GuildSettings {
    pub guild_id: i64,
}

#[derive(Queryable, Selectable, Insertable, Clone, Debug)]
#[diesel(table_name = crate::database::schema::logs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Logs {
    pub id: i32,
    pub guild_id: i64,
    pub default_log_channel: i64,
    pub log_types: i32,
}

#[derive(Queryable, Selectable, Insertable, Clone)]
#[diesel(table_name = crate::database::schema::moderation_settings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ModerationSettings {
    pub guild_id: i64,
    pub warn_expire_time: Option<i64>,
}

#[derive(Queryable, Selectable, Insertable, Clone, Debug)]
#[diesel(table_name = crate::database::schema::cases)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Cases {
    pub guild_id: i64,
    pub user_id: i64,
    pub moderator_id: i64,
    pub case_id: i32,
    pub case_type: String,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub points: Option<i32>,
}