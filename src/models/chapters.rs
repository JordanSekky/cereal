use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    util::is_foreign_key_error,
};

use super::decode_uuid;

pub struct ChapterClient {
    pool: Pool<Sqlite>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum ChapterMetadata {
    RoyalRoad {
        id: u64,
    },
    Pale {
        url: String,
    },
    APracticalGuideToEvil {
        url: String,
    },
    TheWanderingInn {
        url: String,
    },
    TheWanderingInnPatreon {
        url: String,
        password: Option<String>,
    },
    TheDailyGrindPatreon,
    ApparatusOfChangePatreon,
}

impl TryFrom<(&SqliteRow, &str)> for ChapterMetadata {
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

impl ChapterMetadata {
    pub fn json(&self) -> Result<String> {
        let json = serde_json::to_string(self)?;
        Ok(json)
    }
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Chapter {
    pub id: Uuid,
    pub title: String,
    pub metadata: ChapterMetadata,
    #[serde(rename = "bookId")]
    pub book_id: Uuid,
    pub html: Option<Vec<u8>>,
    pub epub: Option<Vec<u8>>,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<Utc>,
}

impl<'r> sqlx::FromRow<'r, SqliteRow> for Chapter {
    fn from_row(row: &'r SqliteRow) -> core::result::Result<Self, sqlx::Error> {
        Ok(Chapter {
            id: decode_uuid(row, "id")?,
            book_id: decode_uuid(row, "book_id")?,
            title: row.try_get("title")?,
            html: row.try_get("html")?,
            epub: row.try_get("epub")?,
            metadata: (row, "metadata").try_into()?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl ChapterClient {
    pub fn new(pool: &Pool<Sqlite>) -> ChapterClient {
        ChapterClient { pool: pool.clone() }
    }

    pub async fn create_chapter(
        &self,
        book_id: &Uuid,
        title: &str,
        metadata: &ChapterMetadata,
        html: Option<&Vec<u8>>,
        epub: Option<&Vec<u8>>,
    ) -> Result<Chapter> {
        let chapter = sqlx::query_as::<_, Chapter>(
            "INSERT INTO chapters(id, book_id, title, metadata, html, epub, created_at, updated_at) 
            VALUES(?, ?, ?, ?, ?, ?, ?, ?) 
            RETURNING *;",
        )
        .bind(Uuid::new_v4().as_bytes().as_slice())
        .bind(book_id.as_bytes().as_slice())
        .bind(title)
        .bind(metadata.json()?)
        .bind(html)
        .bind(epub)
        .bind(Utc::now())
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await;
        match chapter {
            Ok(chapter) => Ok(chapter),
            Err(e) => match is_foreign_key_error(&e) {
                true => Err(Error::ResourceNotFound {
                    id: book_id.to_string(),
                    resource_type: String::from("book"),
                }),
                false => Err(e.into()),
            },
        }
    }

    pub async fn update_chapter(
        &self,
        id: &Uuid,
        title: Option<&str>,
        html: Option<&Vec<u8>>,
        epub: Option<&Vec<u8>>,
    ) -> Result<Chapter> {
        let chapter = sqlx::query_as::<_, Chapter>(
            "UPDATE chapters
                 SET title = coalesce(?, title),
                  html = coalesce(?, html), 
                  epub = coalesce(?, epub), 
                  updated_at = ?
                 WHERE id = ? 
                 RETURNING *;",
        )
        .bind(title)
        .bind(html)
        .bind(epub)
        .bind(Utc::now())
        .bind(id.as_bytes().as_slice())
        .fetch_optional(&self.pool)
        .await?;
        match chapter {
            Some(x) => Ok(x),
            None => Err(Error::ResourceNotFound {
                resource_type: String::from("chapter"),
                id: id.to_string(),
            }),
        }
    }

    pub async fn get_chapter(&self, id: Uuid) -> Result<Option<Chapter>> {
        let book = sqlx::query_as::<_, Chapter>("SELECT * FROM chapters WHERE id = ?")
            .bind(id.as_bytes().as_slice())
            .fetch_optional(&self.pool)
            .await?;
        Ok(book)
    }

    pub async fn list_chapters(&self, book_id: &Uuid) -> Result<Vec<Chapter>> {
        let chapters = sqlx::query_as::<_, Chapter>("SELECT * FROM chapters where book_id = ?")
            .bind(book_id.as_bytes().as_slice())
            .fetch_all(&self.pool)
            .await?;
        Ok(chapters)
    }

    pub async fn delete_chapter(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM chapters WHERE id = ?")
            .bind(id.as_bytes().as_slice())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
