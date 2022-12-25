pub mod books;
use sqlx::{sqlite::SqliteRow, Row};
use uuid::Uuid;

fn decode_uuid(row: &SqliteRow, index: &str) -> core::result::Result<Uuid, sqlx::Error> {
    let id: &[u8] = row.try_get(index)?;
    let id: &[u8; 16] = id.try_into().map_err(|err| sqlx::Error::ColumnDecode {
        index: index.into(),
        source: Box::new(err),
    })?;
    Ok(*Uuid::from_bytes_ref(id))
}
