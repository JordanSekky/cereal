use chrono::Utc;
use serde::Serialize;
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use tracing::{info_span, instrument, Instrument};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

use super::{decode_optional_uuid, decode_uuid, BookClient, ChapterClient, SubscriberClient};

pub struct SubscriptionClient {
    pool: Pool<Sqlite>,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Subscription {
    pub id: Uuid,
    #[serde(rename = "subscriberId")]
    pub subscriber_id: Uuid,
    #[serde(rename = "bookId")]
    pub book_id: Uuid,
    #[serde(rename = "chunkSize")]
    pub chunk_size: i32,
    #[serde(rename = "lastDeliveredChapterId")]
    pub last_delivered_chapter_id: Option<Uuid>,
    #[serde(rename = "lastDeliveredChapterCreatedAt")]
    pub last_delivered_chapter_created_at: Option<chrono::DateTime<Utc>>,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<Utc>,
}

impl<'r> sqlx::FromRow<'r, SqliteRow> for Subscription {
    fn from_row(row: &'r SqliteRow) -> core::result::Result<Self, sqlx::Error> {
        Ok(Subscription {
            id: decode_uuid(row, "id")?,
            book_id: decode_uuid(row, "book_id")?,
            subscriber_id: decode_uuid(row, "subscriber_id")?,
            last_delivered_chapter_id: decode_optional_uuid(row, "last_delivered_chapter_id")?,
            last_delivered_chapter_created_at: row.try_get("last_delivered_chapter_created_at")?,
            chunk_size: row.try_get("chunk_size")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl SubscriptionClient {
    pub fn new(pool: &Pool<Sqlite>) -> SubscriptionClient {
        SubscriptionClient { pool: pool.clone() }
    }

    pub async fn create_subscription(
        &self,
        subscriber_id: &Uuid,
        book_id: &Uuid,
        chunk_size: Option<&i32>,
        last_delivered_chapter_id: Option<&Uuid>,
    ) -> ApiResult<Subscription> {
        // Sqlite doesn't tell us _which_ foreign key causes an error, so we must do some checks
        let book_client = BookClient::new(&self.pool);
        let chapter_client = ChapterClient::new(&self.pool);
        let subscriber_client = SubscriberClient::new(&self.pool);
        if book_client
            .get_book(book_id)
            .instrument(info_span!("Querying db"))
            .await?
            .is_none()
        {
            return Err(ApiError::ResourceNotFound {
                resource_type: String::from("book"),
                id: book_id.to_string(),
            });
        }

        let mut chapter_created_at = None;
        if let Some(chapter_id) = last_delivered_chapter_id {
            let chapter = chapter_client
                .get_chapter(*chapter_id)
                .instrument(info_span!("Querying db"))
                .await?;
            match chapter {
                Some(chapter) => chapter_created_at = Some(chapter.created_at),
                None => {
                    return Err(ApiError::ResourceNotFound {
                        resource_type: String::from("chapter"),
                        id: chapter_id.to_string(),
                    });
                }
            }
        }

        if subscriber_client
            .get_subscriber(*subscriber_id)
            .instrument(info_span!("Querying db"))
            .await?
            .is_none()
        {
            return Err(ApiError::ResourceNotFound {
                resource_type: "subscriber".to_owned(),
                id: subscriber_id.to_string(),
            });
        }

        let subscription = sqlx::query_as::<_, Subscription>(
            "INSERT INTO subscriptions(id, book_id, subscriber_id, chunk_size, last_delivered_chapter_id,
                last_delivered_chapter_created_at, created_at, updated_at) 
            VALUES(?, ?, ?, coalesce(?, 1), ?, ?, ?, ?) 
            RETURNING *;",
        )
        .bind(Uuid::new_v4().as_bytes().as_slice())
        .bind(book_id.as_bytes().as_slice())
        .bind(subscriber_id.as_bytes().as_slice())
        .bind(chunk_size)
        .bind(last_delivered_chapter_id.map(|x| x.as_bytes().as_slice()))
        .bind(chapter_created_at)
        .bind(Utc::now())
        .bind(Utc::now())
        .fetch_one(&self.pool)
            .instrument(info_span!("Querying db"))
        .await?;
        Ok(subscription)
    }

    #[instrument(skip(self))]
    pub async fn update_subscription(
        &self,
        id: &Uuid,
        chunk_size: Option<i32>,
    ) -> ApiResult<Subscription> {
        let subscription = sqlx::query_as::<_, Subscription>(
            "UPDATE subscriptions
                 SET chunk_size = coalesce(?, chunk_size),
                  updated_at = ?
                 WHERE id = ? 
                 RETURNING *;",
        )
        .bind(chunk_size)
        .bind(Utc::now())
        .bind(id.as_bytes().as_slice())
        .fetch_optional(&self.pool)
        .instrument(info_span!("Querying db"))
        .await?;
        match subscription {
            Some(x) => Ok(x),
            None => Err(ApiError::ResourceNotFound {
                id: id.to_string(),
                resource_type: String::from("subscription"),
            }),
        }
    }

    #[instrument(skip(self))]
    pub async fn get_subscription(&self, id: Uuid) -> ApiResult<Option<Subscription>> {
        let subscription =
            sqlx::query_as::<_, Subscription>("SELECT * FROM subscriptions WHERE id = ?")
                .bind(id.as_bytes().as_slice())
                .fetch_optional(&self.pool)
                .instrument(info_span!("Querying db"))
                .await?;
        Ok(subscription)
    }

    #[instrument(skip(self))]
    pub async fn list_subscriptions(&self, subscriber_id: &Uuid) -> ApiResult<Vec<Subscription>> {
        let subscriptions = sqlx::query_as::<_, Subscription>(
            "
        SELECT * FROM subscriptions 
        WHERE subscriber_id = ?",
        )
        .bind(subscriber_id.as_bytes().as_slice())
        .fetch_all(&self.pool)
        .instrument(info_span!("Querying db"))
        .await?;
        Ok(subscriptions)
    }

    #[instrument(skip(self))]
    pub async fn delete_subscription(&self, id: Uuid) -> ApiResult<()> {
        sqlx::query("DELETE FROM subscriptions WHERE id = ?")
            .bind(id.as_bytes().as_slice())
            .execute(&self.pool)
            .instrument(info_span!("Querying db"))
            .await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn set_last_delivered_chapter(
        &self,
        id: &Uuid,
        chapter_id: &Uuid,
        chapter_created_at: &chrono::DateTime<Utc>,
    ) -> ApiResult<Subscription> {
        let subscription = sqlx::query_as::<_, Subscription>(
            "UPDATE subscriptions
                 SET last_delivered_chapter_id = ?,
                  last_delivered_chapter_created_at = ?,
                  updated_at = ?
                 WHERE id = ? 
                 RETURNING *;",
        )
        .bind(chapter_id.as_bytes().as_slice())
        .bind(chapter_created_at)
        .bind(Utc::now())
        .bind(id.as_bytes().as_slice())
        .fetch_optional(&self.pool)
        .instrument(info_span!("Querying db"))
        .await?;
        match subscription {
            Some(x) => Ok(x),
            None => Err(ApiError::ResourceNotFound {
                id: id.to_string(),
                resource_type: String::from("subscription"),
            }),
        }
    }
}
