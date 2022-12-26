use axum::{
    extract::{Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;
use hyper::Body;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::Error,
    models::{Chapter, ChapterClient, ChapterMetadata},
    AppState,
};

#[derive(Debug, PartialEq, Clone, Deserialize)]
struct CreateChapterRequest {
    book_id: Uuid,
    title: String,
    metadata: ChapterMetadata,
}

async fn create_chapter_handler(
    State(state): State<AppState>,
    Json(request): Json<CreateChapterRequest>,
) -> Result<Json<Chapter>, Error> {
    let pool = state.pool;
    let client = ChapterClient::new(&pool);
    let chapter = client
        .create_chapter(
            &request.book_id,
            &request.title,
            &request.metadata,
            None,
            None,
        )
        .await?;
    Ok(chapter.into())
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
struct UpdateChapterRequest {
    id: Uuid,
    title: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
struct UpdateChapterResponse {
    id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    updated_at: chrono::DateTime<Utc>,
}

async fn update_chapter_handler(
    State(state): State<AppState>,
    Json(request): Json<UpdateChapterRequest>,
) -> Result<Json<UpdateChapterResponse>, Error> {
    let pool = state.pool;
    let client = ChapterClient::new(&pool);
    let chapter = client
        .update_chapter(&request.id, request.title.as_deref(), None, None)
        .await?;
    Ok(UpdateChapterResponse {
        id: chapter.id,
        title: request.title,
        updated_at: chapter.updated_at,
    }
    .into())
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
struct GetChapterRequest {
    id: Uuid,
}

async fn get_chapter_handler(
    State(state): State<AppState>,
    Query(request): Query<GetChapterRequest>,
) -> Result<Json<Chapter>, Error> {
    let pool = state.pool;
    let client = ChapterClient::new(&pool);
    let chapter = client.get_chapter(request.id).await?;
    match chapter {
        Some(x) => Ok(x.into()),
        None => Err(Error::ResourceNotFound {
            resource_type: String::from("chapter"),
            id: request.id.to_string(),
        }),
    }
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
struct ListChaptersRequest {
    book_id: Uuid,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
struct ListChaptersResult {
    books: Vec<Chapter>,
}

async fn list_chapters_handler(
    State(state): State<AppState>,
    Query(request): Query<ListChaptersRequest>,
) -> Result<Json<ListChaptersResult>, Error> {
    let pool = state.pool;
    let client = ChapterClient::new(&pool);
    let chapters = client.list_chapters(&request.book_id).await?;
    Ok(ListChaptersResult { books: chapters }.into())
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
struct DeleteChapterRequest {
    id: Uuid,
}

async fn delete_chapter_handler(
    State(state): State<AppState>,
    Json(request): Json<DeleteChapterRequest>,
) -> Result<Json<serde_json::Value>, Error> {
    let pool = state.pool;
    let client = ChapterClient::new(&pool);
    client.delete_chapter(request.id).await?;
    Ok(json!({}).into())
}

pub fn router() -> Router<AppState, Body> {
    Router::new()
        .route("/createChapter", post(create_chapter_handler))
        .route("/updateChapter", post(update_chapter_handler))
        .route("/getChapter", get(get_chapter_handler))
        .route("/listChapters", get(list_chapters_handler))
        .route("/deleteChapter", delete(delete_chapter_handler))
}
