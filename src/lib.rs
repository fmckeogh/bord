use {
    crate::routes::{index, leaderboard, new_submission, static_files, submissions},
    axum::{extract::DefaultBodyLimit, routing::get, Router},
    bollard::Docker,
    color_eyre::eyre::Context,
    sqlx::postgres::PgPoolOptions,
    std::time::Duration,
    tokio::signal,
    tower_http::{
        classify::{ServerErrorsAsFailures, SharedClassifier},
        limit::RequestBodyLimitLayer,
        trace::{
            DefaultMakeSpan, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer,
        },
    },
    tracing::{info, warn, Level},
    tracing_subscriber::{fmt, prelude::*, EnvFilter},
};

mod config;
mod error;
mod routes;

pub use crate::config::Config;

/// Maximum file upload size in bytes
const MAX_UPLOAD_SIZE: usize = 1024 * 1024 * 1024;

const DATABASE_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);
const DATABASE_MIN_CONNECTIONS: u32 = 5;

const DOCKER_CONNECT_TIMEOUT: u64 = 5;

pub async fn start(config: Config) -> color_eyre::Result<()> {
    // initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::builder().parse(config.log_level)?)
        .try_init()?;

    // connect to postgres
    let db = PgPoolOptions::new()
        .acquire_timeout(DATABASE_ACQUIRE_TIMEOUT)
        .min_connections(DATABASE_MIN_CONNECTIONS)
        .connect(&config.database_url)
        .await
        .wrap_err(format!(
            "Failed to connect to database @ {:?}",
            &config.database_url
        ))?;

    info!(
        "connected to postgres @ {}, running migrations",
        config.database_url
    );
    sqlx::migrate!().run(&db).await?;

    // connect to Docker
    let docker = Docker::connect_with_socket(
        &config.docker_socket,
        DOCKER_CONNECT_TIMEOUT,
        bollard::API_DEFAULT_VERSION,
    )?;
    info!("connected to docker: {}", docker.ping().await?);

    let app = Router::new()
        // `GET /` goes to `/static/index.html`
        .route("/", get(index))
        .route("/leaderboard", get(leaderboard))
        .route("/submissions", get(submissions).post(new_submission))
        // serve static files included in binary
        .route("/static/*path", get(static_files))
        .with_state(db)
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(MAX_UPLOAD_SIZE))
        .layer(create_trace_layer());

    // start app
    let listener = tokio::net::TcpListener::bind(config.bind_address).await?;
    info!("listening @ {:?}", listener.local_addr()?);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(Into::into)
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

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
        warn!("received Ctrl+C")
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
        warn!("received terminate signal")
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
