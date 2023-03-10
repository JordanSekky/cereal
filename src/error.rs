use axum::{http::StatusCode, response::IntoResponse};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("{0}")]
    InvalidRequest(String),
    #[error("Resource of type {resource_type} with id {id:?} not found.")]
    ResourceNotFound { resource_type: String, id: String },
    #[error("Failed to serialize a value to json: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("A database error occurred: {0}")]
    Database(#[from] sqlx::Error),
    #[error("A server error occurred: {0}")]
    TowerServer(#[from] hyper::Error),
    #[error("An io error occurred: {0}")]
    Io(#[from] std::io::Error),
}

pub type ApiResult<T> = Result<T, ApiError>;

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        println!("{:?}", self);
        match &self {
            ApiError::ResourceNotFound {
                resource_type: _,
                id: _,
            } => (StatusCode::NOT_FOUND, self.to_string()).into_response(),
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
