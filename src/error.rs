//! Error handling

use {
    axum::{
        http::StatusCode,
        response::{IntoResponse, Response},
    },
    axum_typed_multipart::TypedMultipartError,
    tracing::error,
};

/// Route error, presented to user through `IntoResponse` impl
#[derive(Debug, displaydoc::Display, thiserror::Error)]
pub enum Error {
    /// Database error
    Database(#[from] sqlx::Error),

    /// Multipart file upload failed
    Multipart(#[from] TypedMultipartError),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        error!("Error occurred when handling request: {}", self);

        let status_code = match self {
            Error::Database(_) | Error::Multipart(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status_code, self.to_string()).into_response()
    }
}
