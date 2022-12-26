use axum::{http::StatusCode, response::IntoResponse};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    InvalidRequest(String),
    #[error("Resource of type {resource_type} with id {id:?} not found.")]
    ResourceNotFound { resource_type: String, id: String },
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
        println!("{:?}", self);
        match &self {
            Error::ResourceNotFound {
                resource_type: _,
                id: _,
            } => (StatusCode::NOT_FOUND, self.to_string()).into_response(),
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
