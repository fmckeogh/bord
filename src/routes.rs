use axum_typed_multipart::TypedMultipartError;
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
    pseudonym: String,
    filename: String,
    filesize: i32,
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
                    pseudonym,
                    file_name as filename,
                    file_size as filesize
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
    filename: String,
    filesize: i32,
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
                    pseudonym,
                    file_name as filename,
                    length(file_contents) as "filesize!"
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
#[try_from_multipart(strict)]
pub struct Submission {
    username: String,
    #[form_data(limit = "unlimited")]
    file: FieldData<Bytes>,
}

pub async fn new_submission(
    State(db): State<Pool<Postgres>>,
    BaseMultipart {
        data: Submission { username, file },
        ..
    }: BaseMultipart<Submission, Error>,
) -> Result<impl IntoResponse, Error> {
    let file_contents = file.contents.to_vec();
    let file_name = file
        .metadata
        .file_name
        .ok_or(TypedMultipartError::MissingField {
            field_name: "filename".to_owned(),
        })?;

    debug!(
        "received {:?} ({} bytes) from {}",
        file_name,
        file_contents.len(),
        username
    );

    {
        let mut tx = db.begin().await?;

        sqlx::query!(
            r#"
                INSERT INTO users (username, pseudonym) VALUES ($1, $2) ON CONFLICT DO NOTHING;
            "#,
            &username,
            "beep",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
                INSERT INTO submissions (username, file_contents, file_name) VALUES ($1, $2, $3);
            "#,
            &username,
            file_contents,
            file_name,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
    }

    Ok(StatusCode::OK)
}
