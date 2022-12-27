use std::time::Duration;

use futures::future::join_all;
use sqlx::{Pool, Sqlite};
use tokio::time::MissedTickBehavior;
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::models::{BookClient, ChapterClient};

pub async fn check_for_new_chap_loop(pool: Pool<Sqlite>) {
    // 5 min check interval for all book.
    let mut interval = tokio::time::interval(Duration::from_secs(5 * 60));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let client = BookClient::new(&pool);

    loop {
        // First tick completes immediately.
        interval.tick().await;
        let books = client.list_books().await;
        let mut futures = Vec::new();
        match books {
            Ok(books) => {
                for book in books {
                    futures.push(check_for_new_chapters_in_book(book.id, &pool));
                }
            }
            Err(e) => error!("Error fetching books {}", e),
        }
        join_all(futures).await;
    }
}

#[instrument(skip(pool))]
pub async fn check_for_new_chapters_in_book(book_id: Uuid, pool: &Pool<Sqlite>) {
    let client = ChapterClient::new(pool);
    let most_recent_chapter = match client.most_recent_chapter_by_created_at(&book_id).await {
        Ok(x) => x,
        Err(e) => {
            error!(
                "Error fetching most recent chapter for book {}: {}",
                book_id, e
            );
            return;
        }
    };
    let most_recent_chapter_created_at = most_recent_chapter.map(|x| x.created_at);
    let book = match BookClient::new(pool).get_book(&book_id).await {
        Ok(book) => match book {
            Some(book) => book,
            None => {
                error!("Book with id {} not found", book_id);
                return;
            }
        },
        Err(e) => {
            error!("DB error occurred fetching book with id {}: {}", book_id, e);
            return;
        }
    };

    let chapter_provider = book.metadata.chapter_provider();
    let new_chapters = chapter_provider
        .fetch_new_chapters(&book_id, most_recent_chapter_created_at.as_ref())
        .await;

    let new_chapters = match new_chapters {
        Ok(chapters) => chapters,
        Err(e) => {
            error!(
                "Error occurred fetching chapters for book id {}: {}",
                book_id, e
            );
            return;
        }
    };

    match client.create_chapters(&new_chapters).await {
        Ok(x) => {
            info!("Created new chapters {:?}", x);
        }
        Err(e) => {
            error!("Failed to create new chapters: {}", e)
        }
    };
}
