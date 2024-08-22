use diesel_async::{AsyncConnection, AsyncPgConnection};
use tokio::sync::Mutex;
use futures::Future;

pub struct DbManager {
    connection: Mutex<AsyncPgConnection>,
}

impl DbManager {
    pub async fn new(database_url: &str) -> Result<Self, diesel::result::ConnectionError> {
        let connection = AsyncPgConnection::establish(database_url).await?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    
    pub async fn run<F, Fut, R>(&self, f: F) -> Result<R, diesel::result::Error>
    where
        F: FnOnce(&mut AsyncPgConnection) -> Fut,
        Fut: Future<Output = Result<R, diesel::result::Error>>,
    {
        let mut conn = self.connection.lock().await;
        f(&mut *conn).await
    }
}
