// @generated automatically by Diesel CLI.

diesel::table! {
    _sqlx_migrations (version) {
        version -> Int8,
        description -> Text,
        installed_on -> Timestamptz,
        success -> Bool,
        checksum -> Bytea,
        execution_time -> Int8,
    }
}

diesel::table! {
    cases (id) {
        id -> Int4,
        guild_id -> Int8,
        user_id -> Int8,
        moderator_id -> Int8,
        case_id -> Int4,
        #[max_length = 255]
        case_type -> Varchar,
        reason -> Nullable<Text>,
        created_at -> Timestamptz,
        end_date -> Nullable<Timestamptz>,
        points -> Nullable<Int4>,
    }
}

diesel::table! {
    guild_settings (guild_id) {
        guild_id -> Int8,
        #[max_length = 255]
        lang -> Varchar,
    }
}

diesel::table! {
    logs (id) {
        id -> Int4,
        guild_id -> Int8,
        default_log_channel -> Nullable<Int8>,
        log_types -> Int4,
    }
}

diesel::table! {
    moderation_settings (guild_id) {
        guild_id -> Int8,
        warn_expire_time -> Int8,
    }
}

diesel::joinable!(cases -> guild_settings (guild_id));

diesel::allow_tables_to_appear_in_same_query!(
    _sqlx_migrations,
    cases,
    guild_settings,
    logs,
    moderation_settings,
);
