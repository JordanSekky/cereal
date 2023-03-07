mod books;
mod chapters;
mod subscribers;
mod subscriptions;
use sqlx::{sqlite::SqliteRow, Row};
use uuid::Uuid;

pub use books::{Book, BookClient, BookMetadata};
pub use chapters::{Chapter, ChapterClient, ChapterMetadata, NewChapter, ShallowChapter};
pub use subscribers::{Subscriber, SubscriberClient};
pub use subscriptions::{Subscription, SubscriptionClient};

fn decode_uuid(row: &SqliteRow, index: &str) -> core::result::Result<Uuid, sqlx::Error> {
    let id: &[u8] = row.try_get(index)?;
    let id: &[u8; 16] = id.try_into().map_err(|err| sqlx::Error::ColumnDecode {
        index: index.into(),
        source: Box::new(err),
    })?;
    Ok(*Uuid::from_bytes_ref(id))
}

fn decode_optional_uuid(
    row: &SqliteRow,
    index: &str,
) -> core::result::Result<Option<Uuid>, sqlx::Error> {
    let id: Option<&[u8]> = row.try_get(index)?;
    if id.is_none() {
        return Ok(None);
    }
    Ok(Some(decode_uuid(row, index)?))
}
