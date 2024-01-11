use {
    crate::error::Error,
    axum::{
        body::Bytes,
        extract::{Path, State},
        http::StatusCode,
        response::{IntoResponse, Response},
        Json,
    },
    axum_extra::{
        headers::{CacheControl, ContentType},
        TypedHeader,
    },
    axum_typed_multipart::{BaseMultipart, FieldData, TryFromMultipart},
    include_dir::{include_dir, Dir},
    serde::Serialize,
    sqlx::{Pool, Postgres},
    std::time::Duration,
    tracing::debug,
};

/// Index response
pub async fn index() -> impl IntoResponse {
    static_files(Path("index.html".to_owned())).await
}

/// Handler for static files
pub async fn static_files(Path(path): Path<String>) -> Response {
    const STATIC_FILES_MAX_AGE: Duration = Duration::from_secs(3600);

    static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

    let Some(file) = STATIC_DIR.get_file(&path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let mime_type = mime_guess::from_path(path).first_or_text_plain();

    (
        TypedHeader(ContentType::from(mime_type)),
        TypedHeader(
            CacheControl::new()
                .with_max_age(STATIC_FILES_MAX_AGE)
                .with_public(),
        ),
        file.contents(),
    )
        .into_response()
}

#[derive(Debug, Clone, Serialize)]
struct LeaderboardEntry {
    timestamp: i64,
    score: i32,
    language: String,
    pseudonym: String,
}

pub async fn leaderboard(State(db): State<Pool<Postgres>>) -> Result<impl IntoResponse, Error> {
    Ok(Json(
        sqlx::query_as!(
            LeaderboardEntry,
            r#"
            SELECT
                CAST(
                    EXTRACT(
                        EPOCH FROM leaderboard.submitted_at
                    ) as bigint
                ) as "timestamp!",
                score,
                language,
                pseudonym
            FROM leaderboard
            INNER JOIN users
                ON leaderboard.username=users.username
        "#
        )
        .fetch_all(&db)
        .await?,
    ))
}

#[derive(Debug, Clone, Serialize)]
struct SubmissionEntry {
    timestamp: i64,
    pseudonym: String,
}

pub async fn submissions(State(db): State<Pool<Postgres>>) -> Result<impl IntoResponse, Error> {
    Ok(Json(
        sqlx::query_as!(
            SubmissionEntry,
            r#"
            SELECT
                CAST(
                    EXTRACT(
                        EPOCH FROM submissions.submitted_at
                    ) as bigint
                ) as "timestamp!",
                pseudonym
            FROM submissions
            INNER JOIN users
                ON submissions.username=users.username
        "#
        )
        .fetch_all(&db)
        .await?,
    ))
}

#[derive(Debug, TryFromMultipart)]
pub struct Submission {
    username: String,
    file: FieldData<Bytes>,
}

pub async fn new_submission(
    data: BaseMultipart<Submission, Error>,
) -> Result<impl IntoResponse, Error> {
    debug!(
        "received {:?} ({} bytes) from {}",
        data.file.metadata.file_name,
        data.file.contents.len(),
        data.username
    );
    Ok(StatusCode::OK)
}
