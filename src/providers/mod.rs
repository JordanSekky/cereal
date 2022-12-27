mod apparatus_of_change_patreon;
mod daily_grind_patreon;
mod pale;
mod royalroad;
mod wandering_inn_patreon;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;
pub use wandering_inn_patreon::WanderingInnPatreonNewChapterProvider;

use crate::models::{BookMetadata, Chapter, ChapterMetadata, NewChapter};

use self::{
    apparatus_of_change_patreon::ApparatusOfChangePatreonNewChapterProvider,
    daily_grind_patreon::DailyGrindPatreonNewChapterProvider,
    pale::{PaleChapterBodyProvider, PaleNewChapterProvider},
    royalroad::{RoyalroadChapterBodyProvider, RoyalroadNewChapterProvider},
    wandering_inn_patreon::WanderingInnPatreonChapterBodyProvider,
};

#[async_trait]
pub trait ChapterBodyProvider {
    async fn fetch_chapter_body(&self, chapter: &Chapter) -> anyhow::Result<Vec<u8>>;
}

#[async_trait]
pub trait NewChapterProvider {
    async fn fetch_new_chapters(
        &self,
        book_id: &Uuid,
        last_publish_date: Option<&DateTime<Utc>>,
    ) -> anyhow::Result<Vec<NewChapter>>;
}

impl BookMetadata {
    pub fn chapter_provider(&self) -> Box<dyn NewChapterProvider + Send + Sync> {
        match self {
            BookMetadata::TheWanderingInnPatreon => Box::new(WanderingInnPatreonNewChapterProvider),
            BookMetadata::TheDailyGrindPatreon => Box::new(DailyGrindPatreonNewChapterProvider),
            BookMetadata::ApparatusOfChangePatreon => {
                Box::new(ApparatusOfChangePatreonNewChapterProvider)
            }
            BookMetadata::RoyalRoad { book_id } => Box::new(RoyalroadNewChapterProvider {
                royalroad_book_id: *book_id,
            }),
            BookMetadata::Pale => Box::new(PaleNewChapterProvider),
        }
    }
}

impl ChapterMetadata {
    pub fn body_provider(&self) -> Option<Box<dyn ChapterBodyProvider + Send + Sync>> {
        match self {
            ChapterMetadata::TheWanderingInnPatreon { url, password } => {
                Some(Box::new(WanderingInnPatreonChapterBodyProvider {
                    url: url.clone(),
                    password: password.clone(),
                }))
            }
            ChapterMetadata::RoyalRoad {
                royalroad_book_id: _,
                royalroad_chapter_id,
            } => Some(Box::new(RoyalroadChapterBodyProvider {
                royalroad_chapter_id: *royalroad_chapter_id,
            })),
            ChapterMetadata::Pale { url } => {
                Some(Box::new(PaleChapterBodyProvider { url: url.clone() }))
            }
            ChapterMetadata::TheDailyGrindPatreon => None,
            ChapterMetadata::ApparatusOfChangePatreon => None,
        }
    }
}
