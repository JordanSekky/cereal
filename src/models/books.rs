use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use tracing::{info_span, instrument, Instrument};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

use super::decode_uuid;

pub struct BookClient {
    pool: Pool<Sqlite>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum BookMetadata {
    RoyalRoad(u64),
    Pale,
    TheWanderingInn,
    TheWanderingInnPatreon,
    TheDailyGrindPatreon,
    ApparatusOfChangePatreon,
}

impl TryFrom<(&SqliteRow, &str)> for BookMetadata {
    type Error = sqlx::Error;

    fn try_from(value: (&SqliteRow, &str)) -> core::result::Result<Self, Self::Error> {
        let (row, index) = value;
        let metadata: String = row.try_get(index)?;
        let metadata =
            serde_json::from_str(&metadata).map_err(|err| sqlx::Error::ColumnDecode {
                index: index.into(),
                source: Box::new(err),
            })?;
        Ok(metadata)
    }
}

impl BookMetadata {
    pub fn json(&self) -> ApiResult<String> {
        let json = serde_json::to_string(self)?;
        Ok(json)
    }
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Book {
    pub id: Uuid,
    pub title: String,
    pub author: String,
    pub metadata: BookMetadata,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<Utc>,
}

impl<'r> sqlx::FromRow<'r, SqliteRow> for Book {
    fn from_row(row: &'r SqliteRow) -> core::result::Result<Self, sqlx::Error> {
        Ok(Book {
            id: decode_uuid(row, "id")?,
            title: row.try_get("title")?,
            author: row.try_get("author")?,
            metadata: (row, "metadata").try_into()?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl BookClient {
    pub fn new(pool: &Pool<Sqlite>) -> BookClient {
        BookClient { pool: pool.clone() }
    }

    #[instrument(skip(self))]
    pub async fn create_book(
        &self,
        title: &str,
        author: &str,
        metadata: &BookMetadata,
    ) -> ApiResult<Book> {
        let book = sqlx::query_as::<_, Book>(
            "INSERT INTO books(id, title, author, metadata, created_at, updated_at) 
            VALUES(?, ?, ?, ?, ?, ?) 
            RETURNING *;",
        )
        .bind(Uuid::new_v4().as_bytes().as_slice())
        .bind(title)
        .bind(author)
        .bind(metadata.json()?)
        .bind(Utc::now())
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .instrument(info_span!("Querying db"))
        .await?;
        Ok(book)
    }

    #[instrument(skip(self))]
    pub async fn update_book(
        &self,
        id: &Uuid,
        title: Option<&str>,
        author: Option<&str>,
    ) -> ApiResult<Book> {
        let book = sqlx::query_as::<_, Book>(
            "UPDATE books
                 SET title = coalesce(?, title),
                  author = coalesce(?, author), 
                  updated_at = ?
                 WHERE id = ? 
                 RETURNING *;",
        )
        .bind(title)
        .bind(author)
        .bind(Utc::now())
        .bind(id.as_bytes().as_slice())
        .fetch_optional(&self.pool)
        .instrument(info_span!("Querying db"))
        .await?;
        match book {
            Some(x) => Ok(x),
            None => Err(ApiError::ResourceNotFound {
                id: id.to_string(),
                resource_type: String::from("book"),
            }),
        }
    }

    #[instrument(skip(self))]
    pub async fn get_book(&self, id: &Uuid) -> ApiResult<Option<Book>> {
        let book = sqlx::query_as::<_, Book>("SELECT * FROM books WHERE id = ?")
            .bind(id.as_bytes().as_slice())
            .fetch_optional(&self.pool)
            .instrument(info_span!("Querying db"))
            .await?;
        Ok(book)
    }

    #[instrument(skip(self))]
    pub async fn list_books(&self) -> ApiResult<Vec<Book>> {
        let books = sqlx::query_as::<_, Book>("SELECT * FROM books")
            .fetch_all(&self.pool)
            .instrument(info_span!("Querying db"))
            .await?;
        Ok(books)
    }

    #[instrument(skip(self))]
    pub async fn delete_book(&self, id: &Uuid) -> ApiResult<()> {
        sqlx::query("DELETE FROM books WHERE id = ?")
            .bind(id.as_bytes().as_slice())
            .execute(&self.pool)
            .instrument(info_span!("Querying db"))
            .await?;
        Ok(())
    }
}
