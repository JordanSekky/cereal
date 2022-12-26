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
    models::{Subscriber, SubscriberClient},
    AppState,
};

#[derive(Debug, PartialEq, Clone, Deserialize)]
struct CreateSubscriberRequest {
    name: String,
    kindle_email: Option<String>,
    pushover_key: Option<String>,
}

async fn create_subscriber_handler(
    State(state): State<AppState>,
    Json(request): Json<CreateSubscriberRequest>,
) -> Result<Json<Subscriber>, Error> {
    let pool = state.pool;
    let client = SubscriberClient::new(&pool);
    let subscriber = client
        .create_subscriber(
            &request.name,
            request.pushover_key.as_deref(),
            request.kindle_email.as_deref(),
        )
        .await?;
    Ok(subscriber.into())
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
struct UpdateSubscriberRequest {
    id: Uuid,
    name: Option<String>,
    kindle_email: Option<String>,
    pushover_key: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
struct UpdateSubscriberResponse {
    id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(rename = "pushoverKey")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pushover_key: Option<String>,
    #[serde(rename = "kindleEmail")]
    #[serde(skip_serializing_if = "Option::is_none")]
    kindle_email: Option<String>,
    updated_at: chrono::DateTime<Utc>,
}

async fn update_subscriber_handler(
    State(state): State<AppState>,
    Json(request): Json<UpdateSubscriberRequest>,
) -> Result<Json<UpdateSubscriberResponse>, Error> {
    let pool = state.pool;
    let client = SubscriberClient::new(&pool);
    let subscriber = client
        .update_subscriber(
            &request.id,
            request.name.as_deref(),
            request.kindle_email.as_deref(),
            request.pushover_key.as_deref(),
        )
        .await?;
    Ok(UpdateSubscriberResponse {
        id: subscriber.id,
        name: request.name,
        pushover_key: request.pushover_key,
        kindle_email: request.kindle_email,
        updated_at: subscriber.updated_at,
    }
    .into())
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
struct GetSubscriberRequest {
    id: Uuid,
}

async fn get_subscriber_handler(
    State(state): State<AppState>,
    Query(request): Query<GetSubscriberRequest>,
) -> Result<Json<Subscriber>, Error> {
    let pool = state.pool;
    let client = SubscriberClient::new(&pool);
    let subscriber = client.get_subscriber(request.id).await?;
    match subscriber {
        Some(x) => Ok(x.into()),
        None => Err(Error::ResourceNotFound {
            resource_type: String::from("subscriber"),
            id: request.id.to_string(),
        }),
    }
}

#[derive(Debug, PartialEq, Clone, Serialize)]
struct ListSubscribersResult {
    subscribers: Vec<Subscriber>,
}

async fn list_subscribers_handler(
    State(state): State<AppState>,
) -> Result<Json<ListSubscribersResult>, Error> {
    let pool = state.pool;
    let client = SubscriberClient::new(&pool);
    let subscribers = client.list_subscribers().await?;
    Ok(ListSubscribersResult { subscribers }.into())
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
struct DeleteSubscriberRequest {
    id: Uuid,
}

async fn delete_subscriber_handler(
    State(state): State<AppState>,
    Json(request): Json<DeleteSubscriberRequest>,
) -> Result<Json<serde_json::Value>, Error> {
    let pool = state.pool;
    let client = SubscriberClient::new(&pool);
    client.delete_subscriber(request.id).await?;
    Ok(json!({}).into())
}

pub fn router() -> Router<AppState, Body> {
    Router::new()
        .route("/createSubscriber", post(create_subscriber_handler))
        .route("/updateSubscriber", post(update_subscriber_handler))
        .route("/getSubscriber", get(get_subscriber_handler))
        .route("/listSubscribers", get(list_subscribers_handler))
        .route("/deleteSubscriber", delete(delete_subscriber_handler))
}
