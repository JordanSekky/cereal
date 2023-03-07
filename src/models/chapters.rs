use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use tracing::{error, info_span, instrument, Instrument};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    util::is_foreign_key_error,
};

use super::decode_uuid;

#[derive(PartialEq, Clone, Eq)]
pub struct NewChapter {
    pub title: String,
    pub metadata: ChapterMetadata,
    pub book_id: Uuid,
    pub html: Option<Vec<u8>>,
    pub epub: Option<Vec<u8>>,
    pub published_at: Option<chrono::DateTime<Utc>>,
}

impl std::fmt::Debug for NewChapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NewChapter")
            .field("title", &self.title)
            .field("metadata", &self.metadata)
            .field("book_id", &self.book_id)
            .field("html_bytes", &self.html.as_ref().map(|x| x.len()))
            .field("epub_bytes", &self.epub.as_ref().map(|x| x.len()))
            .field("published_at", &self.published_at)
            .finish()
    }
}

pub struct ChapterClient {
    pool: Pool<Sqlite>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum ChapterMetadata {
    RoyalRoad {
        royalroad_book_id: u64,
        royalroad_chapter_id: u64,
    },
    Pale {
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
    pub fn json(&self) -> ApiResult<String> {
        let json = serde_json::to_string(self)?;
        Ok(json)
    }
}

#[derive(PartialEq, Clone, Serialize)]
pub struct Chapter {
    pub id: Uuid,
    pub title: String,
    pub metadata: ChapterMetadata,
    #[serde(rename = "bookId")]
    pub book_id: Uuid,
    pub html: Option<Vec<u8>>,
    pub epub: Option<Vec<u8>>,
    #[serde(rename = "publishedAt")]
    pub published_at: Option<chrono::DateTime<Utc>>,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<Utc>,
}

impl std::fmt::Debug for Chapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chapter")
            .field("id", &self.id)
            .field("title", &self.title)
            .field("metadata", &self.metadata)
            .field("book_id", &self.book_id)
            .field("html_bytes", &self.html.as_ref().map(|x| x.len()))
            .field("epub_bytes", &self.epub.as_ref().map(|x| x.len()))
            .field("published_at", &self.published_at)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
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
            published_at: row.try_get("published_at")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[derive(PartialEq, Clone, Serialize)]
pub struct ShallowChapter {
    pub id: Uuid,
    pub title: String,
    pub metadata: ChapterMetadata,
    #[serde(rename = "bookId")]
    pub book_id: Uuid,
    pub html_bytes: Option<i64>,
    pub epub_bytes: Option<i64>,
    #[serde(rename = "publishedAt")]
    pub published_at: Option<chrono::DateTime<Utc>>,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<Utc>,
}

impl std::fmt::Debug for ShallowChapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShallowChapter")
            .field("id", &self.id)
            .field("title", &self.title)
            .field("metadata", &self.metadata)
            .field("book_id", &self.book_id)
            .field("html_bytes", &self.html_bytes)
            .field("epub_bytes", &self.epub_bytes)
            .field("published_at", &self.published_at)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl<'r> sqlx::FromRow<'r, SqliteRow> for ShallowChapter {
    fn from_row(row: &'r SqliteRow) -> core::result::Result<Self, sqlx::Error> {
        Ok(ShallowChapter {
            id: decode_uuid(row, "id")?,
            book_id: decode_uuid(row, "book_id")?,
            title: row.try_get("title")?,
            html_bytes: row.try_get("html_bytes")?,
            epub_bytes: row.try_get("epub_bytes")?,
            metadata: (row, "metadata").try_into()?,
            published_at: row.try_get("published_at")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl ChapterClient {
    pub fn new(pool: &Pool<Sqlite>) -> ChapterClient {
        ChapterClient { pool: pool.clone() }
    }

    #[instrument(skip(self))]
    pub async fn create_chapter(
        &self,
        book_id: &Uuid,
        title: &str,
        metadata: &ChapterMetadata,
        html: Option<&Vec<u8>>,
        epub: Option<&Vec<u8>>,
        published_at: Option<chrono::DateTime<Utc>>,
    ) -> ApiResult<Chapter> {
        let chapter = sqlx::query_as::<_, Chapter>(
            "INSERT INTO chapters(id, book_id, title, metadata, html, epub, published_at, created_at, updated_at) 
            VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?) 
            RETURNING *;",
        )
        .bind(Uuid::new_v4().as_bytes().as_slice())
        .bind(book_id.as_bytes().as_slice())
        .bind(title)
        .bind(metadata.json()?)
        .bind(html)
        .bind(epub)
        .bind(published_at)
        .bind(Utc::now())
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .instrument(info_span!("Querying db"))
        .await;
        match chapter {
            Ok(chapter) => Ok(chapter),
            Err(e) => match is_foreign_key_error(&e) {
                true => Err(ApiError::ResourceNotFound {
                    id: book_id.to_string(),
                    resource_type: String::from("book"),
                }),
                false => Err(e.into()),
            },
        }
    }

    pub async fn create_chapters(&self, chapters: &Vec<NewChapter>) -> ApiResult<Vec<Chapter>> {
        let transaction = self.pool.begin().await?;
        let mut inserted_chapters = Vec::with_capacity(chapters.len());
        for chapter in chapters {
            let inserted_chapter = sqlx::query_as::<_, Chapter>(
            "INSERT INTO chapters(id, book_id, title, metadata, html, epub, published_at, created_at, updated_at) 
            VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?) 
            RETURNING *;",
                )
                .bind(Uuid::new_v4().as_bytes().as_slice())
                .bind(chapter.book_id.as_bytes().as_slice())
                .bind(&chapter.title)
                .bind(chapter.metadata.json()?)
                .bind(chapter.html.as_ref())
                .bind(chapter.epub.as_ref())
                .bind(chapter.published_at)
                .bind(Utc::now())
                .bind(Utc::now())
                .fetch_one(&self.pool)
                .instrument(info_span!("Querying db"))
                .await;
            match inserted_chapter {
                Ok(chapter) => inserted_chapters.push(chapter),
                Err(e) => {
                    error!("Error occurred, cancelling transaction: {}", e);
                    transaction.rollback().await?;
                    return Err(e.into());
                }
            }
        }
        transaction.commit().await?;
        Ok(inserted_chapters)
    }

    #[instrument(skip(self))]
    pub async fn update_chapter(
        &self,
        id: &Uuid,
        title: Option<&str>,
        html: Option<&Vec<u8>>,
        epub: Option<&Vec<u8>>,
        published_at: Option<&chrono::DateTime<Utc>>,
    ) -> ApiResult<Chapter> {
        let chapter = sqlx::query_as::<_, Chapter>(
            "UPDATE chapters
                 SET title = coalesce(?, title),
                  html = coalesce(?, html), 
                  epub = coalesce(?, epub), 
                  published_at = coalesce(?, published_at),
                  updated_at = ?
                 WHERE id = ? 
                 RETURNING *;",
        )
        .bind(title)
        .bind(html)
        .bind(epub)
        .bind(published_at)
        .bind(Utc::now())
        .bind(id.as_bytes().as_slice())
        .fetch_optional(&self.pool)
        .instrument(info_span!("Querying db"))
        .await?;
        match chapter {
            Some(x) => Ok(x),
            None => Err(ApiError::ResourceNotFound {
                resource_type: String::from("chapter"),
                id: id.to_string(),
            }),
        }
    }

    #[instrument(skip(self))]
    pub async fn get_chapter(&self, id: Uuid) -> ApiResult<Option<Chapter>> {
        let book = sqlx::query_as::<_, Chapter>("SELECT * FROM chapters WHERE id = ?")
            .bind(id.as_bytes().as_slice())
            .fetch_optional(&self.pool)
            .instrument(info_span!("Querying db"))
            .await?;
        Ok(book)
    }

    #[instrument(skip(self))]
    pub async fn list_chapters(&self, book_id: &Uuid) -> ApiResult<Vec<Chapter>> {
        let chapters =
            sqlx::query_as::<_, Chapter>("SELECT * FROM chapters where book_id = ? ORDER BY coalesce(published_at, created_at) DESC")
                .bind(book_id.as_bytes().as_slice())
                .fetch_all(&self.pool)
                .instrument(info_span!("Querying db"))
                .await?;
        Ok(chapters)
    }

    #[instrument(skip(self))]
    pub async fn list_chapters_shallow(&self, book_id: &Uuid) -> ApiResult<Vec<ShallowChapter>> {
        let chapters =
            sqlx::query_as::<_, ShallowChapter>("SELECT id, book_id, title, metadata, length(html) as html_bytes, length(epub) as epub_bytes, published_at, created_at, updated_at FROM chapters where book_id = ? ORDER BY coalesce(published_at, created_at) DESC")
                .bind(book_id.as_bytes().as_slice())
                .fetch_all(&self.pool)
                .instrument(info_span!("Querying db"))
                .await?;
        Ok(chapters)
    }

    #[instrument(skip(self))]
    pub async fn delete_chapter(&self, id: &Uuid) -> ApiResult<()> {
        sqlx::query("DELETE FROM chapters WHERE id = ?")
            .bind(id.as_bytes().as_slice())
            .execute(&self.pool)
            .instrument(info_span!("Querying db"))
            .await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn most_recent_chapter_by_published_at(
        &self,
        book_id: &Uuid,
    ) -> ApiResult<Option<Chapter>> {
        let book = sqlx::query_as::<_, Chapter>("SELECT * FROM chapters WHERE book_id = ? ORDER BY coalesce(published_at, created_at) DESC LIMIT 1")
            .bind(book_id.as_bytes().as_slice())
            .fetch_optional(&self.pool)
            .instrument(info_span!("Querying db"))
            .await?;
        Ok(book)
    }

    #[instrument(skip(self))]
    pub async fn most_recent_chapter_by_created_at(
        &self,
        book_id: &Uuid,
    ) -> ApiResult<Option<Chapter>> {
        let book = sqlx::query_as::<_, Chapter>(
            "SELECT * FROM chapters WHERE book_id = ? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(book_id.as_bytes().as_slice())
        .fetch_optional(&self.pool)
        .instrument(info_span!("Querying db"))
        .await?;
        Ok(book)
    }

    #[instrument(skip(self))]
    pub async fn list_chapters_without_bodies(&self) -> ApiResult<Vec<Chapter>> {
        let chapters =
            sqlx::query_as::<_, Chapter>("SELECT * FROM chapters where html IS NULL ORDER BY coalesce(published_at, created_at) DESC")
                .fetch_all(&self.pool)
                .instrument(info_span!("Querying db"))
                .await?;
        Ok(chapters)
    }

    #[instrument(skip(self))]
    pub async fn list_chapters_ready_for_epub_conversion(&self) -> ApiResult<Vec<Chapter>> {
        let chapters =
            sqlx::query_as::<_, Chapter>("SELECT * FROM chapters WHERE html IS NOT NULL AND epub IS NULL ORDER BY coalesce(published_at, created_at) DESC")
                .fetch_all(&self.pool)
                .instrument(info_span!("Querying db"))
                .await?;
        Ok(chapters)
    }

    #[instrument(skip(self))]
    pub async fn list_chapters_with_epub(
        &self,
        book_id: &Uuid,
        datetime: Option<&DateTime<Utc>>,
    ) -> ApiResult<Vec<Chapter>> {
        let chapters =
            sqlx::query_as::<_, Chapter>("SELECT * FROM chapters WHERE epub IS NOT NULL AND coalesce(created_at > ?,  true) AND book_id = ? ORDER BY coalesce(published_at, created_at) ASC")
            .bind(datetime)
            .bind(book_id.as_bytes().as_slice())
                .fetch_all(&self.pool)
                .instrument(info_span!("Querying db"))
                .await?;
        Ok(chapters)
    }
}
