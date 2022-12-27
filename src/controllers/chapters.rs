use axum::{
    extract::{Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    error::ApiError,
    models::{Chapter, ChapterClient, ChapterMetadata},
    AppState,
};

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateChapterRequest {
    #[serde(rename = "bookId")]
    book_id: Uuid,
    title: String,
    metadata: ChapterMetadata,
    #[serde(rename = "publishedAt")]
    published_at: Option<DateTime<Utc>>,
}

#[instrument(skip(state))]
async fn create_chapter_handler(
    State(state): State<AppState>,
    Json(request): Json<CreateChapterRequest>,
) -> Result<Json<Chapter>, ApiError> {
    let pool = state.pool;
    let client = ChapterClient::new(&pool);
    let chapter = client
        .create_chapter(
            &request.book_id,
            &request.title,
            &request.metadata,
            None,
            None,
            request.published_at,
        )
        .await?;
    Ok(chapter.into())
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateChapterRequest {
    id: Uuid,
    title: Option<String>,
    #[serde(rename = "publishedAt")]
    published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
struct UpdateChapterResponse {
    id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(rename = "updatedAt")]
    updated_at: chrono::DateTime<Utc>,
}

#[instrument(skip(state))]
async fn update_chapter_handler(
    State(state): State<AppState>,
    Json(request): Json<UpdateChapterRequest>,
) -> Result<Json<UpdateChapterResponse>, ApiError> {
    let pool = state.pool;
    let client = ChapterClient::new(&pool);
    let chapter = client
        .update_chapter(
            &request.id,
            request.title.as_deref(),
            None,
            None,
            request.published_at.as_ref(),
        )
        .await?;
    Ok(UpdateChapterResponse {
        id: chapter.id,
        title: request.title,
        updated_at: chapter.updated_at,
    }
    .into())
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct GetChapterRequest {
    id: Uuid,
}

#[instrument(skip(state))]
async fn get_chapter_handler(
    State(state): State<AppState>,
    Query(request): Query<GetChapterRequest>,
) -> Result<Json<Chapter>, ApiError> {
    let pool = state.pool;
    let client = ChapterClient::new(&pool);
    let chapter = client.get_chapter(request.id).await?;
    match chapter {
        Some(x) => Ok(x.into()),
        None => Err(ApiError::ResourceNotFound {
            resource_type: String::from("chapter"),
            id: request.id.to_string(),
        }),
    }
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListChaptersRequest {
    #[serde(rename = "bookId")]
    book_id: Uuid,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
struct ListChaptersResult {
    chapters: Vec<Chapter>,
}

#[instrument(skip(state))]
async fn list_chapters_handler(
    State(state): State<AppState>,
    Query(request): Query<ListChaptersRequest>,
) -> Result<Json<ListChaptersResult>, ApiError> {
    let pool = state.pool;
    let client = ChapterClient::new(&pool);
    let chapters = client.list_chapters(&request.book_id).await?;
    Ok(ListChaptersResult { chapters }.into())
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeleteChapterRequest {
    id: Uuid,
}

#[instrument(skip(state))]
async fn delete_chapter_handler(
    State(state): State<AppState>,
    Json(request): Json<DeleteChapterRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = state.pool;
    let client = ChapterClient::new(&pool);
    client.delete_chapter(&request.id).await?;
    Ok(json!({}).into())
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/createChapter", post(create_chapter_handler))
        .route("/updateChapter", post(update_chapter_handler))
        .route("/getChapter", get(get_chapter_handler))
        .route("/listChapters", get(list_chapters_handler))
        .route("/deleteChapter", delete(delete_chapter_handler))
}
