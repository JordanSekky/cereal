use std::time::Duration;

use sqlx::{Pool, Sqlite};
use tokio::time::MissedTickBehavior;
use tracing::{error, info, instrument};

use crate::models::{Chapter, ChapterClient};

pub async fn check_for_bodiless_chap_loop(pool: Pool<Sqlite>) {
    // 10 sec check interval for all chapters.
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let client = ChapterClient::new(&pool);

    loop {
        // First tick completes immediately.
        interval.tick().await;
        let chapters = client.list_chapters_without_bodies().await;
        match chapters {
            Ok(chapters) => {
                for chapter in chapters {
                    fetch_chapter_body(&chapter, &pool).await
                }
            }
            Err(e) => error!("Error fetching chapters with empty bodies {}", e),
        }
    }
}

#[instrument(skip(pool))]
pub async fn fetch_chapter_body(chapter: &Chapter, pool: &Pool<Sqlite>) {
    let client = ChapterClient::new(pool);

    let chapter_provider = chapter.metadata.body_provider();
    let chapter_body = chapter_provider.fetch_chapter_body(chapter).await;
    let chapter_body = match chapter_body {
        Ok(x) => x,
        Err(e) => {
            error!("Error fetching chapters with empty bodies {}", e);
            return;
        }
    };

    info!("Found body with length {:?}", chapter_body.len());

    match client
        .update_chapter(&chapter.id, None, Some(&chapter_body), None, None)
        .await
    {
        Ok(x) => {
            info!("Created new chapter body for chapter {:?}", x.id);
        }
        Err(e) => {
            error!("Failed to body for chapter: {}", e)
        }
    };
}
