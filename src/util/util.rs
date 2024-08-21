
use diesel::{PgConnection, QueryDsl, RunQueryDsl};
use tokio::sync::MutexGuard;

pub fn generate_case_id(db: &mut MutexGuard<PgConnection>) -> i32 {
    use crate::database::schema::cases::dsl::*;

    let max_case_id: Option<i32> = cases
        .select(diesel::dsl::max(case_id))
        .first(&mut **db)
        .unwrap_or(None);

    max_case_id.unwrap_or(0) + 1
}