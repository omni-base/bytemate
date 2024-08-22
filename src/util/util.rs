
use diesel::{QueryDsl};
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use tokio::sync::MutexGuard;

pub async fn generate_case_id<'a>(db: &mut MutexGuard<'a, AsyncPgConnection>) -> i32 {
    use crate::database::schema::cases::dsl::*;

    let max_case_id: Option<i32> = cases
        .select(diesel::dsl::max(case_id))
        .first(&mut **db).await.unwrap_or(None);

    max_case_id.unwrap_or(0) + 1
}