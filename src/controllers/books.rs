use axum::{
    extract::{Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::Error,
    models::{Book, BookClient, BookMetadata},
    AppState,
};

#[derive(Debug, PartialEq, Clone, Deserialize)]
struct CreateBookRequest {
    title: String,
    author: String,
    metadata: BookMetadata,
}

async fn create_book_handler(
    State(state): State<AppState>,
    Json(request): Json<CreateBookRequest>,
) -> Result<Json<Book>, Error> {
    let pool = state.pool;
    let client = BookClient::new(&pool);
    let book = client
        .create_book(&request.title, &request.author, &request.metadata)
        .await?;
    Ok(book.into())
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
struct UpdateBookRequest {
    id: Uuid,
    title: Option<String>,
    author: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
struct UpdateBookResponse {
    id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,
    updated_at: chrono::DateTime<Utc>,
}

async fn update_book_handler(
    State(state): State<AppState>,
    Json(request): Json<UpdateBookRequest>,
) -> Result<Json<UpdateBookResponse>, Error> {
    let pool = state.pool;
    let client = BookClient::new(&pool);
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
struct GetBookRequest {
    id: Uuid,
}

async fn get_book_handler(
    State(state): State<AppState>,
    Query(request): Query<GetBookRequest>,
) -> Result<Json<Book>, Error> {
    let pool = state.pool;
    let client = BookClient::new(&pool);
    let book = client.get_book(request.id).await?;
    match book {
        Some(x) => Ok(x.into()),
        None => Err(Error::ResourceNotFound {
            resource_type: String::from("book"),
            id: request.id.to_string(),
        }),
    }
}

#[derive(Debug, PartialEq, Clone, Serialize)]
struct ListBooksResult {
    books: Vec<Book>,
}

async fn list_books_handler(State(state): State<AppState>) -> Result<Json<ListBooksResult>, Error> {
    let pool = state.pool;
    let client = BookClient::new(&pool);
    let books = client.list_books().await?;
    Ok(ListBooksResult { books }.into())
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
struct DeleteBookRequest {
    id: Uuid,
}

async fn delete_book_handler(
    State(state): State<AppState>,
    Json(request): Json<DeleteBookRequest>,
) -> Result<Json<serde_json::Value>, Error> {
    let pool = state.pool;
    let client = BookClient::new(&pool);
    client.delete_book(request.id).await?;
    Ok(json!({}).into())
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/createBook", post(create_book_handler))
        .route("/updateBook", post(update_book_handler))
        .route("/getBook", get(get_book_handler))
        .route("/listBooks", get(list_books_handler))
        .route("/deleteBook", delete(delete_book_handler))
}
