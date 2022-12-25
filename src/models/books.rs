use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use uuid::Uuid;

use crate::error::{Error, Result};

use super::decode_uuid;

pub struct BooksClient {
    pool: Pool<Sqlite>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum BookMetadata {
    RoyalRoad(u64),
    Pale,
    APracticalGuideToEvil,
    TheWanderingInn,
    TheWanderingInnPatreon,
    TheDailyGrindPatreon,
    ApparatusOfChangePatreon,
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
            metadata: decode_book_metadata(row, "metadata")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

fn decode_book_metadata(
    row: &SqliteRow,
    index: &str,
) -> core::result::Result<BookMetadata, sqlx::Error> {
    let book_metadata: String = row.try_get(index)?;
    let book_metadata =
        serde_json::from_str(&book_metadata).map_err(|err| sqlx::Error::ColumnDecode {
            index: index.into(),
            source: Box::new(err),
        })?;
    Ok(book_metadata)
}

fn encode_book_metadata(metadata: &BookMetadata) -> Result<String> {
    let book_metadata = serde_json::to_string(metadata)?;
    Ok(book_metadata)
}

impl BooksClient {
    pub fn new(pool: &Pool<Sqlite>) -> BooksClient {
        BooksClient { pool: pool.clone() }
    }

    pub async fn create_book(
        &self,
        title: &str,
        author: &str,
        metadata: &BookMetadata,
    ) -> Result<Book> {
        let book = sqlx::query_as::<_, Book>(
            "INSERT INTO books(id, title, author, metadata, created_at, updated_at) 
            VALUES(?, ?, ?, ?, ?, ?) 
            RETURNING *;",
        )
        .bind(Uuid::new_v4().as_bytes().as_slice())
        .bind(title)
        .bind(author)
        .bind(encode_book_metadata(metadata)?)
        .bind(Utc::now())
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?;
        Ok(book)
    }

    pub async fn update_book(
        &self,
        id: &Uuid,
        title: Option<&str>,
        author: Option<&str>,
    ) -> Result<Book> {
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
        .await?;
        match book {
            Some(x) => Ok(x),
            None => Err(Error::ResourceNotFound()),
        }
    }

    pub async fn get_book(&self, id: Uuid) -> Result<Option<Book>> {
        let book = sqlx::query_as::<_, Book>("SELECT * FROM books WHERE id = ?")
            .bind(id.as_bytes().as_slice())
            .fetch_optional(&self.pool)
            .await?;
        Ok(book)
    }

    pub async fn list_books(&self) -> Result<Vec<Book>> {
        let books = sqlx::query_as::<_, Book>("SELECT * FROM books")
            .fetch_all(&self.pool)
            .await?;
        Ok(books)
    }

    pub async fn delete_book(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM books WHERE id = ?")
            .bind(id.as_bytes().as_slice())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
