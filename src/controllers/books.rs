use axum::{
    extract::{Query, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::Error,
    models::books::{Book, BookMetadata, BooksClient},
    AppState,
};

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub struct CreateBookRequest {
    pub title: String,
    pub author: String,
    pub metadata: BookMetadata,
}

pub async fn create_book_handler(
    State(state): State<AppState>,
    Json(request): Json<CreateBookRequest>,
) -> Result<Json<Book>, Error> {
    let pool = state.pool;
    let client = BooksClient::new(&pool);
    let book = client
        .create_book(&request.title, &request.author, &request.metadata)
        .await?;
    Ok(book.into())
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub struct UpdateBookRequest {
    pub id: Uuid,
    pub title: Option<String>,
    pub author: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct UpdateBookResponse {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub updated_at: chrono::DateTime<Utc>,
}

pub async fn update_book_handler(
    State(state): State<AppState>,
    Json(request): Json<UpdateBookRequest>,
) -> Result<Json<UpdateBookResponse>, Error> {
    let pool = state.pool;
    let client = BooksClient::new(&pool);
    let book = client
        .update_book(
            &request.id,
            request.title.as_deref(),
            request.author.as_deref(),
        )
        .await?;
    Ok(UpdateBookResponse {
        id: book.id,
        title: request.title,
        author: request.author,
        updated_at: book.updated_at,
    }
    .into())
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub struct GetBookRequest {
    pub id: Uuid,
}

pub async fn get_book_handler(
    State(state): State<AppState>,
    Query(request): Query<GetBookRequest>,
) -> Result<Json<Book>, Error> {
    let pool = state.pool;
    let client = BooksClient::new(&pool);
    let book = client.get_book(request.id).await?;
    match book {
        Some(x) => Ok(x.into()),
        None => Err(Error::ResourceNotFound()),
    }
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct ListBooksResult {
    pub books: Vec<Book>,
}

pub async fn list_books_handler(
    State(state): State<AppState>,
) -> Result<Json<ListBooksResult>, Error> {
    let pool = state.pool;
    let client = BooksClient::new(&pool);
    let books = client.list_books().await?;
    Ok(ListBooksResult { books }.into())
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub struct DeleteBookRequest {
    pub id: Uuid,
}

pub async fn delete_book_handler(
    State(state): State<AppState>,
    Json(request): Json<DeleteBookRequest>,
) -> Result<Json<serde_json::Value>, Error> {
    let pool = state.pool;
    let client = BooksClient::new(&pool);
    client.delete_book(request.id).await?;
    Ok(json!({}).into())
}
