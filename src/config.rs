use {
    color_eyre::eyre::{Result, WrapErr},
    config::Environment,
    serde::Deserialize,
    std::net::SocketAddr,
};

/// Configuration parameters
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    /// Socket to bind HTTP server to
    pub bind_address: SocketAddr,

    /// Postgres URL
    pub database_url: String,

    /// Path to Docker socket
    pub docker_socket: String,

    /// Log level filter
    pub log_level: String,
}

impl Config {
    /// Builds a new Config instance from environment
    /// variables
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        config::Config::builder()
            .add_source(Environment::default())
            .build()
            .wrap_err("Failed build configuration")?
            .try_deserialize()
            .wrap_err("Failed deserialize configuration")
    }
}
