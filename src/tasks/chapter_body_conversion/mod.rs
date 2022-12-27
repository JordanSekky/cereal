use std::time::Duration;

use anyhow::bail;
use itertools::Itertools;
use sqlx::{Pool, Sqlite};
use tokio::time::MissedTickBehavior;
use tracing::{info, instrument};

use crate::{
    error,
    models::{Book, BookClient, Chapter, ChapterClient},
};

mod calibre;

pub async fn check_for_epubless_chap_loop(pool: Pool<Sqlite>) {
    // 10 sec check interval for all chapters.
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let client = ChapterClient::new(&pool);

    loop {
        // First tick completes immediately.
        interval.tick().await;
        let chapters = client.list_chapters_ready_for_epub_conversion().await;
        match chapters {
            Ok(chapters) => {
                for chapter in chapters {
                    generate_chapter_epub(chapter, &pool).await
                }
            }
            Err(e) => error!("Error fetching chapters with empty epub fields {}", e),
        }
    }
}

#[instrument(skip(pool))]
pub async fn generate_chapter_epub(chapter: Chapter, pool: &Pool<Sqlite>) {
    let client = ChapterClient::new(pool);

    let book_id = chapter.book_id;
    let chapter_id = chapter.id;
    let chapter_body = match chapter.html {
        Some(body) => body,
        None => {
            error!("Chapter id {} had no html body", &chapter_id);
            return;
        }
    };

    let book = match BookClient::new(pool).get_book(&book_id).await {
        Ok(Some(book)) => book,
        Ok(None) => {
            error!(
                "Book with id {} not found for chapter {}",
                &book_id, &chapter_id
            );
            return;
        }
        Err(e) => {
            error!(
                "A database error occurred looking up book with id {} for chapter {}: {}",
                &book_id, &chapter_id, e
            );
            return;
        }
    };

    let cover_title = &format!("{}: {}", &book.title, &chapter.title);

    let epub_bytes = calibre::generate_epub(
        ".html",
        chapter_body.as_slice(),
        cover_title,
        &book.title,
        &book.author,
    )
    .await;

    let epub_bytes = match epub_bytes {
        Ok(x) => x,
        Err(e) => {
            error!(
                "A database error occurred converting body to epub for chapter {}: {}",
                &chapter_id, e
            );
            return;
        }
    };

    info!("Generated epub body with length {:?}", epub_bytes.len());

    match client
        .update_chapter(&chapter.id, None, None, Some(&epub_bytes), None)
        .await
    {
        Ok(x) => {
            info!("Created new epub chapter body for chapter {:?}", x.id);
        }
        Err(e) => {
            error!("Failed to body for chapter: {}", e)
        }
    };
}

#[instrument]
pub async fn generate_multichapter_epub(
    cover_title: &str,
    chapters: &[Chapter],
    book: &Book,
) -> anyhow::Result<Vec<u8>> {
    if chapters.is_empty() {
        bail!("Provided chapters slice is empty.");
    }

    if !chapters.iter().all(|x| x.book_id.eq(&book.id)) {
        bail!("Some chapters were not related to provided book.");
    }

    if !chapters.iter().all(|x| x.html.is_some()) {
        bail!("Not every chapter has an html body.");
    }

    // Ensure chapters are in publication order.
    let chapters = chapters
        .iter()
        .sorted_by(|a, b| {
            let a = a.published_at.unwrap_or(a.created_at);
            let b = b.published_at.unwrap_or(b.created_at);
            a.cmp(&b)
        })
        .collect_vec();

    let html_body: Vec<u8> = chapters
        .iter()
        .flat_map(|x| {
            let mut bytes = format!("<h1>{}</h1>", x.title).as_bytes().to_vec();
            bytes.append(&mut x.html.clone().unwrap());
            bytes
        })
        .collect();

    let epub_bytes = calibre::generate_epub(
        ".html",
        html_body.as_slice(),
        cover_title,
        &book.title,
        &book.author,
    )
    .await;

    let epub_bytes = match epub_bytes {
        Ok(x) => x,
        Err(e) => {
            bail!(
                "A database error occurred converting body to epub for chapters {:?}: {}",
                &chapters,
                e
            );
        }
    };

    info!("Generated epub body with length {:?}", epub_bytes.len());
    Ok(epub_bytes)
}
