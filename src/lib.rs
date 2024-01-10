use tracing::{info, Level};
use {
    crate::routes::{index, static_files},
    axum::{routing::get, Router},
    serde::Deserialize,
    std::net::SocketAddr,
    tower_http::{
        classify::{ServerErrorsAsFailures, SharedClassifier},
        trace::{
            DefaultMakeSpan, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer,
        },
    },
    tracing_subscriber::{fmt, prelude::*, EnvFilter},
};

mod routes;

/// Configuration parameters
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    /// Socket to bind HTTP server to
    pub bind_address: SocketAddr,

    /// Postgres URL
    pub database_url: String,

    /// Log level filter
    pub log_level: String,
}

pub async fn start(config: Config) -> color_eyre::Result<()> {
    // initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::builder().parse(config.log_level)?)
        .try_init()?;

    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(index))
        // serve static files included in binary
        .route("/static/*path", get(static_files))
        .layer(create_trace_layer());

    // start app
    let listener = tokio::net::TcpListener::bind(config.bind_address).await?;
    info!("listening @ {:?}", listener.local_addr()?);
    Ok(axum::serve(listener, app).await?)
}

/// Creates a TraceLayer for request, response and failure logging
pub fn create_trace_layer() -> TraceLayer<SharedClassifier<ServerErrorsAsFailures>> {
    TraceLayer::new_for_http()
        .make_span_with(
            DefaultMakeSpan::new()
                .level(Level::INFO)
                .include_headers(true),
        )
        // failures have the ERROR level
        .on_failure(DefaultOnFailure::new().level(Level::ERROR))
        // requests have the INFO level
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        // responses have the INFO level
        .on_response(
            DefaultOnResponse::new()
                .level(Level::INFO)
                .include_headers(true),
        )
}
