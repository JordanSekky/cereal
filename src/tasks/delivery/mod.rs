mod pushover;
use std::time::Duration;

use anyhow::anyhow;
use futures::future::join_all;
use sqlx::{Pool, Sqlite};
use tokio::time::MissedTickBehavior;
use tracing::{info, instrument};

use crate::{
    error,
    models::{
        Book, BookClient, Chapter, ChapterClient, Subscriber, SubscriberClient, Subscription,
        SubscriptionClient,
    },
};

pub async fn check_for_ready_delivery_loop(pool: Pool<Sqlite>) {
    // 10 sec check interval for all chapters.
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        // First tick completes immediately.
        interval.tick().await;
        let mut futures = Vec::new();
        let deliveries = find_ready_deliveries(&pool).await;
        match deliveries {
            Ok(deliveries) => {
                for delivery in deliveries {
                    let future =
                        deliver_subscription(delivery.0, delivery.1, delivery.2, delivery.3, &pool);
                    futures.push(future);
                }
            }
            Err(e) => error!("Error fetching chapters with empty epub fields {}", e),
        }
        join_all(futures).await;
    }
}

#[instrument(skip(pool), ret)]
async fn find_ready_deliveries(
    pool: &Pool<Sqlite>,
) -> anyhow::Result<Vec<(Subscriber, Subscription, Book, Vec<Chapter>)>> {
    let mut deliveries = Vec::new();

    let book_client = BookClient::new(pool);
    let chapter_client = ChapterClient::new(pool);
    let subscriber_client = SubscriberClient::new(pool);
    let subscription_client = SubscriptionClient::new(pool);

    let subscribers = subscriber_client.list_subscribers().await?;
    for subscriber in subscribers {
        let subscriptions = subscription_client
            .list_subscriptions(&subscriber.id)
            .await?;
        for subscription in subscriptions {
            let book = book_client
                .get_book(&subscription.book_id)
                .await?
                .ok_or_else(|| anyhow!("Book not found"))?;
            let chapters = chapter_client
                .list_chapters_with_epub(
                    &book.id,
                    subscription.last_delivered_chapter_created_at.as_ref(),
                )
                .await?;
            if chapters.len() >= subscription.chunk_size as usize {
                deliveries.push((subscriber.clone(), subscription, book, chapters));
            }
        }
    }

    Ok(deliveries)
}

async fn deliver_subscription(
    subscriber: Subscriber,
    subscription: Subscription,
    book: Book,
    chapters: Vec<Chapter>,
    pool: &Pool<Sqlite>,
) {
    let pushover_token = subscriber.pushover_key.clone();

    if let Some(pushover_token) = pushover_token {
        let message = match chapters.len() {
            1 => format!(
                "Delivered new chapter for {}: {}",
                book.title, chapters[0].title
            ),
            n => format!(
                "Delivered new chapters for {}. {} through {}",
                book.title,
                chapters[0].title,
                chapters[n - 1].title
            ),
        };
        match pushover::send_message(&pushover_token, &message).await {
            Ok(_) => (),
            Err(e) => {
                error!("Failed to send pushover message to subscriber {:?} for book {:?} and chapters {:?}: {}", subscriber, book, chapters, e);
                return;
            }
        };
    }

    let subscription_client = SubscriptionClient::new(pool);
    let latest_chapter = &chapters[chapters.len() - 1];
    let update_result = subscription_client
        .set_last_delivered_chapter(
            &subscription.id,
            &latest_chapter.id,
            &latest_chapter.created_at,
        )
        .await;
    match update_result {
        Ok(_) => info!(
            "Set subscription {} to have latest chapter {:?}",
            &subscription.id, latest_chapter
        ),
        Err(e) => info!(
            "A DB error occurred setting subscription {} to have latest chapter {:?}: {}",
            &subscription.id, latest_chapter, e
        ),
    }
}
