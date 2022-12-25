use axum::{http::StatusCode, response::IntoResponse};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Not Found")]
    ResourceNotFound(),
    #[error("Failed to serialize a value to json")]
    Serialization(#[from] serde_json::Error),
    #[error("A database error occurred")]
    Database(#[from] sqlx::Error),
    #[error("A server error occurred")]
    TowerServer(#[from] hyper::Error),
}

pub type Result<T> = core::result::Result<T, Error>;

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Error::ResourceNotFound() => StatusCode::NOT_FOUND.into_response(),
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
