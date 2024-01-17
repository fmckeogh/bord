use {
    crate::error::Error,
    axum::{
        body::Bytes,
        extract::{Path, State},
        http::StatusCode,
        response::{IntoResponse, Response},
        Json,
    },
    axum_extra::{headers::ContentType, TypedHeader},
    axum_typed_multipart::{BaseMultipart, FieldData, TryFromMultipart, TypedMultipartError},
    include_dir::{include_dir, Dir},
    serde::Serialize,
    sqlx::{Pool, Postgres},
    std::hash::Hasher,
    tracing::debug,
    twox_hash::XxHash64,
};

/// Index response
pub async fn index() -> impl IntoResponse {
    static_files(Path("index.html".to_owned())).await
}

/// Handler for static files
pub async fn static_files(Path(path): Path<String>) -> Response {
    static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

    // retrieve file from static dir if it exists
    let Some(file) = STATIC_DIR.get_file(&path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    // try to guess the mime type
    let mime_type = mime_guess::from_path(path).first_or_text_plain();

    // serve the file
    (TypedHeader(ContentType::from(mime_type)), file.contents()).into_response()
}

#[derive(Debug, Clone, Serialize)]
pub struct LeaderboardEntry {
    timestamp: i64,
    score: i32,
    pseudonym: String,
    filename: String,
    filesize: i32,
}

// Returns a JSON array of leaderboard entries
pub async fn leaderboard(State(db): State<Pool<Postgres>>) -> Result<impl IntoResponse, Error> {
    Ok(Json(
        sqlx::query_as!(
            LeaderboardEntry,
            r#"
                SELECT
                    submitted_at as "timestamp",
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
pub struct SubmissionEntry {
    timestamp: i64,
    pseudonym: String,
    filename: String,
    filesize: i32,
}

// Returns a JSON array of submission entries
pub async fn submissions(State(db): State<Pool<Postgres>>) -> Result<impl IntoResponse, Error> {
    Ok(Json(
        sqlx::query_as!(
            SubmissionEntry,
            r#"
                SELECT
                    submitted_at as "timestamp",
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
    // still limited by the axum request body limiter, just not limited within the multipart
    // handler
    file: FieldData<Bytes>,
}

/// Handles new submissions from students
pub async fn new_submission(
    State(db): State<Pool<Postgres>>,
    // type-safe multipart extractor
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
            hash_username(&username),
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

/// Emits the first 6 hex characters of the hash of the username
fn hash_username(username: &str) -> String {
    let mut hasher = XxHash64::with_seed(0);
    hasher.write(username.as_bytes());
    format!("{:X}", hasher.finish() as u32)
}
