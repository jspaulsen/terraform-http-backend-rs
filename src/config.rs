use std::{
    net::IpAddr,
    sync::Arc,
};

use envconfig::Envconfig;
use tracing_subscriber::EnvFilter;

pub type SharedConfiguration = Arc<Configuration>;



#[derive(Debug, Envconfig)]
pub struct Configuration {
    #[envconfig(from = "DATABASE_URI", default = "/var/lib/terraform-http-backend/tf_state.db")]
    pub database_uri: String,

    #[envconfig(from = "TF_HTTP_USERNAME")]
    pub tf_http_username: String,

    #[envconfig(from = "TF_HTTP_PASSWORD")]
    pub tf_http_password: String,

    #[envconfig(from = "HTTP_PORT", default = "8080")]
    pub http_port: u16,

    #[envconfig(from = "HTTP_BIND_ADDRESS", default = "0.0.0.0")]
    pub http_bind_address: IpAddr,

    #[envconfig(from = "LOG_LEVEL", default = "INFO")]
    pub log_level: EnvFilter,
}
