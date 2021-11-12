use std::{
    net::SocketAddr,
    sync::Arc,
};

use axum;
use envconfig::Envconfig;

use crate::api::Api;

mod api;
mod config;
mod database;
mod db;
mod error;
mod extractors;
mod models;
mod routes;


#[tokio::main]
async fn main() {
    let config = Arc::new(
        config::Configuration::init_from_env()
            .unwrap()
    );

    tracing_subscriber::fmt()
        .with_env_filter(
            config
                .log_level
                .to_string()
        )
        .init();

    let config = Arc::new(
        config::Configuration::init_from_env()
            .unwrap()
    );

    let database = database::get_db_pool(&config)
        .await
        .expect("Failed to setup database connection pool!");

    // attempt to setup migrations as part of application startup
    database::MIGRATE.run(&database)
        .await
        .expect("Failed to run database migrations!");

    let socket = SocketAddr::from((config.http_bind_address, config.http_port));
    let api: axum::Router = Api::new(
        config.clone(),
        database.clone()
    ).into();

    axum::Server::bind(&socket)
        .serve(api.into_make_service())
        .await
        .unwrap()
}
