use std::str::FromStr;

use log::LevelFilter;
use sqlx::{
    ConnectOptions,
    Error,
    SqlitePool,
    migrate::Migrator,
    sqlite::SqliteConnectOptions,
};

use super::config::Configuration;


pub static MIGRATE: Migrator = sqlx::migrate!();


pub async fn get_db_pool(config: &Configuration) -> Result<SqlitePool, Error> {
    let mut opts = SqliteConnectOptions::from_str(&config.database_uri)?
        .create_if_missing(true);

    opts.log_statements(LevelFilter::Debug);

    SqlitePool::connect_with(opts)
        .await
}
