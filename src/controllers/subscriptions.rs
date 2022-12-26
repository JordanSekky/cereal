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
    models::{Subscription, SubscriptionClient},
    AppState,
};

#[derive(Debug, PartialEq, Clone, Deserialize)]
struct CreateSubscriptionRequest {
    #[serde(rename = "subscriberId")]
    subscriber_id: Uuid,
    #[serde(rename = "bookId")]
    book_id: Uuid,
    #[serde(rename = "chunkSize")]
    chunk_size: Option<i32>,
}

async fn create_subscription_handler(
    State(state): State<AppState>,
    Json(request): Json<CreateSubscriptionRequest>,
) -> Result<Json<Subscription>, Error> {
    let pool = state.pool;
    let client = SubscriptionClient::new(&pool);
    let subscription = client
        .create_subscription(
            &request.subscriber_id,
            &request.book_id,
            request.chunk_size.as_ref(),
        )
        .await?;

    Ok(subscription.into())
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
struct UpdateSubscriptionRequest {
    id: Uuid,
    #[serde(rename = "chunkSize")]
    chunk_size: Option<i32>,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
struct UpdateSubscriptionResponse {
    id: Uuid,
    #[serde(rename = "chunkSize")]
    chunk_size: Option<i32>,
    updated_at: chrono::DateTime<Utc>,
}

async fn update_subscription_handler(
    State(state): State<AppState>,
    Json(request): Json<UpdateSubscriptionRequest>,
) -> Result<Json<UpdateSubscriptionResponse>, Error> {
    if request.chunk_size.is_none() {
        return Err(Error::InvalidRequest(String::from(
            "Expected one of [chunk_size] to be set but none were.",
        )));
    }
    let pool = state.pool;
    let client = SubscriptionClient::new(&pool);
    let subscriber = client
        .update_subscription(&request.id, request.chunk_size)
        .await?;
    Ok(UpdateSubscriptionResponse {
        id: subscriber.id,
        updated_at: subscriber.updated_at,
        chunk_size: request.chunk_size,
    }
    .into())
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
struct GetSubscriptionRequest {
    id: Uuid,
}

async fn get_subscription_handler(
    State(state): State<AppState>,
    Query(request): Query<GetSubscriptionRequest>,
) -> Result<Json<Subscription>, Error> {
    let pool = state.pool;
    let client = SubscriptionClient::new(&pool);
    let subscriber = client.get_subscription(request.id).await?;
    match subscriber {
        Some(x) => Ok(x.into()),
        None => Err(Error::ResourceNotFound {
            resource_type: String::from("subscription"),
            id: request.id.to_string(),
        }),
    }
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
struct ListSubscriptionsRequest {
    subscriber_id: Uuid,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
struct ListSubscriptionsResult {
    subscriptions: Vec<Subscription>,
}

async fn list_subscriptions_handler(
    State(state): State<AppState>,
    Query(request): Query<ListSubscriptionsRequest>,
) -> Result<Json<ListSubscriptionsResult>, Error> {
    let pool = state.pool;
    let client = SubscriptionClient::new(&pool);
    let subscriptions = client.list_subscriptions(&request.subscriber_id).await?;
    Ok(ListSubscriptionsResult { subscriptions }.into())
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
struct DeleteSubscriptionRequest {
    id: Uuid,
}

async fn delete_subscription_handler(
    State(state): State<AppState>,
    Json(request): Json<DeleteSubscriptionRequest>,
) -> Result<Json<serde_json::Value>, Error> {
    let pool = state.pool;
    let client = SubscriptionClient::new(&pool);
    client.delete_subscription(request.id).await?;
    Ok(json!({}).into())
}

pub fn router() -> Router<AppState, Body> {
    Router::new()
        .route("/createSubscription", post(create_subscription_handler))
        .route("/updateSubscription", post(update_subscription_handler))
        .route("/getSubscription", get(get_subscription_handler))
        .route("/listSubscriptions", get(list_subscriptions_handler))
        .route("/deleteSubscription", delete(delete_subscription_handler))
}
