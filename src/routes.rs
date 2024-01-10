use {
    axum::{
        extract::Path,
        http::StatusCode,
        response::{IntoResponse, Response},
    },
    axum_extra::{
        headers::{CacheControl, ContentType},
        TypedHeader,
    },
    include_dir::{include_dir, Dir},
    std::time::Duration,
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
