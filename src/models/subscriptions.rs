use chrono::Utc;
use serde::Serialize;
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use uuid::Uuid;

use crate::error::{Error, Result};

use super::{decode_uuid, BookClient, SubscriberClient};

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
    ) -> Result<Subscription> {
        // Sqlite doesn't tell us _which_ foreign key causes an error, so we must do some checks
        let book_client = BookClient::new(&self.pool);
        let subscriber_client = SubscriberClient::new(&self.pool);
        if book_client.get_book(*book_id).await?.is_none() {
            return Err(Error::ResourceNotFound {
                resource_type: String::from("book"),
                id: book_id.to_string(),
            });
        }

        if subscriber_client
            .get_subscriber(*subscriber_id)
            .await?
            .is_none()
        {
            return Err(Error::ResourceNotFound {
                resource_type: "subscriber".to_owned(),
                id: subscriber_id.to_string(),
            });
        }

        let subscription = sqlx::query_as::<_, Subscription>(
            "INSERT INTO subscriptions(id, book_id, subscriber_id, chunk_size, created_at, updated_at) 
            VALUES(?, ?, ?, coalesce(?, 1), ?, ?) 
            RETURNING *;",
        )
        .bind(Uuid::new_v4().as_bytes().as_slice())
        .bind(book_id.as_bytes().as_slice())
        .bind(subscriber_id.as_bytes().as_slice())
        .bind(chunk_size)
        .bind(Utc::now())
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?;
        Ok(subscription)
    }

    pub async fn update_subscription(
        &self,
        id: &Uuid,
        chunk_size: Option<i32>,
    ) -> Result<Subscription> {
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
        .await?;
        match subscription {
            Some(x) => Ok(x),
            None => Err(Error::ResourceNotFound {
                id: id.to_string(),
                resource_type: String::from("subscription"),
            }),
        }
    }

    pub async fn get_subscription(&self, id: Uuid) -> Result<Option<Subscription>> {
        let subscription =
            sqlx::query_as::<_, Subscription>("SELECT * FROM subscriptions WHERE id = ?")
                .bind(id.as_bytes().as_slice())
                .fetch_optional(&self.pool)
                .await?;
        Ok(subscription)
    }

    pub async fn list_subscriptions(&self, subscriber_id: &Uuid) -> Result<Vec<Subscription>> {
        let subscriptions = sqlx::query_as::<_, Subscription>(
            "
        SELECT * FROM subscribers 
        WHERE subscriber_id = ?",
        )
        .bind(subscriber_id.as_bytes().as_slice())
        .fetch_all(&self.pool)
        .await?;
        Ok(subscriptions)
    }

    pub async fn delete_subscription(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM subscriptions WHERE id = ?")
            .bind(id.as_bytes().as_slice())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
