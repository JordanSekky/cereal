use chrono::Utc;
use serde::Serialize;
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use tracing::{info_span, instrument, Instrument};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

use super::decode_uuid;

pub struct SubscriberClient {
    pool: Pool<Sqlite>,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Subscriber {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "kindleEmail")]
    pub kindle_email: Option<String>,
    #[serde(rename = "pushoverKey")]
    pub pushover_key: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<Utc>,
}

impl<'r> sqlx::FromRow<'r, SqliteRow> for Subscriber {
    fn from_row(row: &'r SqliteRow) -> core::result::Result<Self, sqlx::Error> {
        Ok(Subscriber {
            id: decode_uuid(row, "id")?,
            name: row.try_get("name")?,
            kindle_email: row.try_get("kindle_email")?,
            pushover_key: row.try_get("pushover_key")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl SubscriberClient {
    pub fn new(pool: &Pool<Sqlite>) -> SubscriberClient {
        SubscriberClient { pool: pool.clone() }
    }

    #[instrument(skip(self))]
    pub async fn create_subscriber(
        &self,
        name: &str,
        pushover_key: Option<&str>,
        kindle_email: Option<&str>,
    ) -> ApiResult<Subscriber> {
        let subscriber = sqlx::query_as::<_, Subscriber>(
            "INSERT INTO subscribers(id, name, kindle_email, pushover_key, created_at, updated_at) 
            VALUES(?, ?, ?, ?, ?, ?) 
            RETURNING *;",
        )
        .bind(Uuid::new_v4().as_bytes().as_slice())
        .bind(name)
        .bind(kindle_email)
        .bind(pushover_key)
        .bind(Utc::now())
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .instrument(info_span!("Querying db"))
        .await?;
        Ok(subscriber)
    }

    #[instrument(skip(self))]
    pub async fn update_subscriber(
        &self,
        id: &Uuid,
        name: Option<&str>,
        kindle_email: Option<&str>,
        pushover_key: Option<&str>,
    ) -> ApiResult<Subscriber> {
        let subscriber = sqlx::query_as::<_, Subscriber>(
            "UPDATE subscribers
                 SET kindle_email = coalesce(?, kindle_email),
                  pushover_key = coalesce(?, pushover_key), 
                  name = coalesce(?, name),
                  updated_at = ?
                 WHERE id = ? 
                 RETURNING *;",
        )
        .bind(kindle_email)
        .bind(pushover_key)
        .bind(name)
        .bind(Utc::now())
        .bind(id.as_bytes().as_slice())
        .fetch_optional(&self.pool)
        .instrument(info_span!("Querying db"))
        .await?;
        match subscriber {
            Some(x) => Ok(x),
            None => Err(ApiError::ResourceNotFound {
                id: id.to_string(),
                resource_type: String::from("subscriber"),
            }),
        }
    }

    #[instrument(skip(self))]
    pub async fn get_subscriber(&self, id: Uuid) -> ApiResult<Option<Subscriber>> {
        let subscriber = sqlx::query_as::<_, Subscriber>("SELECT * FROM subscribers WHERE id = ?")
            .bind(id.as_bytes().as_slice())
            .fetch_optional(&self.pool)
            .instrument(info_span!("Querying db"))
            .await?;
        Ok(subscriber)
    }

    #[instrument(skip(self))]
    pub async fn list_subscribers(&self) -> ApiResult<Vec<Subscriber>> {
        let subscribers = sqlx::query_as::<_, Subscriber>("SELECT * FROM subscribers")
            .fetch_all(&self.pool)
            .instrument(info_span!("Querying db"))
            .await?;
        Ok(subscribers)
    }

    #[instrument(skip(self))]
    pub async fn delete_subscriber(&self, id: Uuid) -> ApiResult<()> {
        sqlx::query("DELETE FROM subscribers WHERE id = ?")
            .bind(id.as_bytes().as_slice())
            .execute(&self.pool)
            .instrument(info_span!("Querying db"))
            .await?;
        Ok(())
    }
}
