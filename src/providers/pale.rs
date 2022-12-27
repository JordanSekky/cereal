use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use itertools::Itertools;
use scraper::{Html, Selector};
use uuid::Uuid;

use crate::models::Chapter;
use crate::models::ChapterMetadata;
use crate::models::NewChapter;
use tracing::instrument;

use super::ChapterBodyProvider;
use super::NewChapterProvider;

pub struct PaleNewChapterProvider;

#[async_trait]
impl NewChapterProvider for PaleNewChapterProvider {
    #[instrument(skip(self), level = "info", ret)]
    async fn fetch_new_chapters(
        &self,
        book_id: &Uuid,
        last_publish_date: Option<&DateTime<Utc>>,
    ) -> anyhow::Result<Vec<NewChapter>> {
        return get_chapters(book_id, last_publish_date).await;
    }
}

#[derive(Clone)]
pub struct PaleChapterBodyProvider {
    pub url: String,
}

#[async_trait]
impl ChapterBodyProvider for PaleChapterBodyProvider {
    #[instrument(skip(self))]
    async fn fetch_chapter_body(&self, _chapter: &Chapter) -> anyhow::Result<Vec<u8>> {
        let url = self.url.clone();
        Ok(get_chapter_body(&url).await?)
    }
}

#[instrument(ret)]
pub async fn get_chapters(
    book_uuid: &Uuid,
    last_publish_date: Option<&DateTime<Utc>>,
) -> anyhow::Result<Vec<NewChapter>> {
    let content = reqwest::get("https://palewebserial.wordpress.com/feed/")
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
                metadata: ChapterMetadata::Pale {
                    url: item
                        .link()
                        .ok_or_else(|| anyhow!("No chapter link in RSS item. Item {:?}", &item))?
                        .into(),
                },
                html: None,
                epub: None,
                title: item
                    .title()
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

#[instrument]
pub async fn get_chapter_body(link: &str) -> Result<Vec<u8>, anyhow::Error> {
    let res = reqwest::get(link).await?.text().await?;
    let doc = Html::parse_document(&res);
    let chapter_body_elem_selector = Selector::parse("div.entry-content > *").unwrap();

    let body = doc
        .select(&chapter_body_elem_selector)
        .filter(|x| x.value().id() != Some("jp-post-flair"))
        .filter(|x| !x.text().any(|t| t == "Next Chapter"))
        .filter(|x| !x.text().any(|t| t == "Previous Chapter"))
        .map(|x| x.html())
        .join("\n");
    if body.trim().is_empty() {
        bail!("Failed to find chapter body.");
    }
    Ok(body.as_bytes().to_vec())
}
