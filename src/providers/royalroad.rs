extern crate futures;
extern crate reqwest;

use crate::models::Chapter;
use crate::models::ChapterMetadata;
use crate::models::NewChapter;

use anyhow::anyhow;
use anyhow::Context;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use scraper::{Html, Selector};
use tracing::instrument;
use uuid::Uuid;

use anyhow::Result;

use super::ChapterBodyProvider;
use super::NewChapterProvider;

pub struct RoyalroadNewChapterProvider {
    pub royalroad_book_id: u64,
}

#[async_trait]
impl NewChapterProvider for RoyalroadNewChapterProvider {
    #[instrument(skip(self), level = "info", ret)]
    async fn fetch_new_chapters(
        &self,
        book_id: &Uuid,
        last_publish_date: Option<&DateTime<Utc>>,
    ) -> anyhow::Result<Vec<NewChapter>> {
        return get_chapters(self.royalroad_book_id, book_id, last_publish_date).await;
    }
}

#[derive(Clone)]
pub struct RoyalroadChapterBodyProvider {
    pub royalroad_chapter_id: u64,
}

#[async_trait]
impl ChapterBodyProvider for RoyalroadChapterBodyProvider {
    #[instrument(skip(self))]
    async fn fetch_chapter_body(&self, _chapter: &Chapter) -> anyhow::Result<Vec<u8>> {
        Ok(get_chapter_body(&self.royalroad_chapter_id).await?)
    }
}

pub async fn get_chapter_body(royalroad_chapter_id: &u64) -> Result<Vec<u8>> {
    let link = format!(
        "https://www.royalroad.com/fiction/chapter/{}",
        royalroad_chapter_id
    );
    let res = reqwest::get(&link).await?.text().await?;
    let doc = Html::parse_document(&res);
    let chapter_body_selector = Selector::parse("div.chapter-inner").unwrap();

    let body = doc
        .select(&chapter_body_selector)
        .next()
        .ok_or_else(|| anyhow!("Failed to find body in {}", link))?
        .html();
    Ok(body.into_bytes())
}

pub async fn get_chapters(
    royalroad_book_id: u64,
    book_uuid: &Uuid,
    last_publish_date: Option<&DateTime<Utc>>,
) -> Result<Vec<NewChapter>> {
    let content = reqwest::get(format!(
        "https://www.royalroad.com/syndication/{}",
        royalroad_book_id
    ))
    .await?
    .bytes()
    .await?;
    let channel = rss::Channel::read_from(&content[..])?;
    channel
        .items()
        .iter()
        .map(|item| {
            Ok(NewChapter {
                book_id: *book_uuid,
                metadata: ChapterMetadata::RoyalRoad {
                    royalroad_book_id,
                    royalroad_chapter_id: get_chapter_id_from_link(item.link())?,
                },
                html: None,
                epub: None,
                title: item
                    .title()
                    .and_then(|x| x.split_once(" - "))
                    .map(|x| x.1)
                    .ok_or_else(|| anyhow!("No chapter title in RSS item. Item {:?}", &item))?
                    .into(),
                published_at: Some(
                    item.pub_date()
                        .ok_or_else(|| anyhow!("No publish date in RSS item. Item {:?}", &item))
                        .and_then(|x| {
                            DateTime::parse_from_rfc2822(x).with_context(|| {
                                format!(
                                    "Failed to parse publish date in RSS item. Item {:?}",
                                    &item
                                )
                            })
                        })?
                        .with_timezone(&Utc),
                ),
            })
        })
        .filter(|x| match x {
            Ok(y) => y.published_at.as_ref() > last_publish_date,
            Err(_) => true,
        })
        .collect()
}

fn get_chapter_id_from_link(link: Option<&str>) -> Result<u64> {
    link.and_then(|link| {
        link.rsplit_once('/')
            .map(|(_left, right)| right)
            .and_then(|x| x.parse().ok())
    })
    .ok_or_else(|| anyhow!("No valid royalroad chapter link in RSS Item."))
}
