use std::collections::HashMap;
use std::env;

use anyhow::anyhow;
use anyhow::bail;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use futures::future::try_join_all;
use itertools::Chunk;
use itertools::Itertools;
use mailparse::MailHeaderMap;
use reqwest::Method;
use rusoto_core::credential::StaticProvider;
use rusoto_core::HttpClient;
use rusoto_core::Region;
use rusoto_s3::GetObjectRequest;
use rusoto_s3::ListObjectsV2Request;
use rusoto_s3::Object;
use rusoto_s3::S3Client;
use rusoto_s3::S3;
use scraper::{Html, Selector};
use selectors::Element;
use tokio::io::AsyncReadExt;
use tracing::info;
use tracing::instrument;
use uuid::Uuid;

use crate::models::Chapter;
use crate::models::ChapterMetadata;

use super::ChapterBodyProvider;
use super::NewChapter;
use super::NewChapterProvider;

pub struct WanderingInnPatreonNewChapterProvider;

#[async_trait]
impl NewChapterProvider for WanderingInnPatreonNewChapterProvider {
    #[tracing::instrument(skip(self), level = "info", ret)]
    async fn fetch_new_chapters(
        &self,
        book_id: &Uuid,
        last_publish_date: Option<&DateTime<Utc>>,
    ) -> anyhow::Result<Vec<NewChapter>> {
        return get_chapters(book_id, last_publish_date).await;
    }
}

#[derive(Clone)]
pub struct WanderingInnPatreonChapterBodyProvider {
    pub url: String,
    pub password: Option<String>,
}

#[async_trait]
impl ChapterBodyProvider for WanderingInnPatreonChapterBodyProvider {
    #[instrument(skip(self))]
    async fn fetch_chapter_body(&self, _chapter: &Chapter) -> anyhow::Result<Vec<u8>> {
        let url = self.url.clone();
        let password = self.password.clone();
        Ok(get_chapter_body(&url, password.as_deref())
            .await?
            .as_bytes()
            .into())
    }
}

#[tracing::instrument(name = "Listing S3 objects for new emails", level = "info", ret)]
pub async fn get_chapters(
    book_id: &Uuid,
    last_publish_date: Option<&DateTime<Utc>>,
) -> anyhow::Result<Vec<NewChapter>> {
    let s3 = S3Client::new_with(
        HttpClient::new().expect("failed to create request dispatcher"),
        StaticProvider::new_minimal(
            env::var("AWS_ACCESS_KEY")?,
            env::var("AWS_SECRET_ACCESS_KEY")?,
        ),
        Region::default(),
    );
    let bucket = env::var("AWS_EMAIL_BUCKET")?;
    let objects = s3
        .list_objects_v2(ListObjectsV2Request {
            bucket: bucket.clone(),
            ..Default::default()
        })
        .await?;
    info!("List objects results: {:?}", objects.contents);
    let chapter_objects = objects
        .contents
        .unwrap_or_else(|| Vec::with_capacity(0))
        .into_iter()
        // Object must be newer than the most recent delivered chapter.
        .filter(|x| match x.last_modified.as_ref() {
            Some(lm) => match DateTime::parse_from_rfc3339(lm) {
                Ok(published_at) => {
                    if let Some(last_publish_date) = last_publish_date {
                        published_at > *last_publish_date
                    } else {
                        // No published date provided for book, all objects are valid
                        true
                    }
                }
                // Object publish date failed to parse.
                Err(_) => false,
            },
            None => false,
        })
        .collect_vec();
    let chapter_futures = chapter_objects
        .into_iter()
        .map(|obj| get_new_chapter_from_email(obj, &bucket, &s3, book_id));
    let chapters = try_join_all(chapter_futures)
        .await?
        .into_iter()
        .flatten()
        .collect();
    Ok(chapters)
}

#[tracing::instrument(
    name = "Getting chapter metadata from email.",
    level = "info",
    skip(s3),
    ret
)]
async fn get_new_chapter_from_email(
    s3_obj: Object,
    bucket_name: &str,
    s3: &S3Client,
    book_id: &Uuid,
) -> anyhow::Result<Vec<NewChapter>> {
    let chapter_object = s3
        .get_object(GetObjectRequest {
            bucket: bucket_name.to_owned(),
            key: s3_obj
                .key
                .ok_or_else(|| anyhow!("No key found on s3 object."))?,
            ..Default::default()
        })
        .await?;
    let published_at = chapter_object.last_modified.and_then(|lm| {
        DateTime::parse_from_rfc2822(&lm)
            .ok()
            .map(|x| x.with_timezone(&Utc))
    });
    tracing::info!("Published at {:?}", published_at);
    let mut chapter_bytes = Vec::new();
    chapter_object
        .body
        .ok_or_else(|| anyhow!("No body on s3 object."))?
        .into_async_read()
        .read_to_end(&mut chapter_bytes)
        .await?;
    let chapter_email = mailparse::parse_mail(&chapter_bytes)?;
    let subject = chapter_email.headers.get_first_value("Subject");
    info!("Subject is {:?}", subject);
    match subject {
        Some(x) => {
            if !x.to_lowercase().contains("pirateaba") {
                // Not from pirataba, return zero chapters.
                return Ok(Vec::with_capacity(0));
            }
        }
        // No subject, return zero chapters.
        None => return Ok(Vec::with_capacity(0)),
    }

    let singlepart_email_body = chapter_email.get_body().ok();
    let multipart_email_body = chapter_email
        .subparts
        .iter()
        .last()
        .and_then(|x| x.get_body().ok());
    let body = match singlepart_email_body.or(multipart_email_body) {
        Some(b) => b,
        // No body, return zero chapters.
        None => return Ok(Vec::with_capacity(0)),
    };
    tracing::info!("Found wandering inn patreon email with body: {}", body);
    let doc = Html::parse_document(&body);
    let para_tags_selector = Selector::parse("div > p").unwrap();

    let password = doc
        .select(&para_tags_selector)
        .filter(|x| x.text().any(|t| t.to_lowercase().contains("password")))
        .map(|x| x.next_sibling_element().map(|sib| sib.text().join("")))
        .exactly_one()
        .ok()
        .flatten()
        .or_else(|| {
            doc.select(&para_tags_selector)
                .flat_map(|x| x.text())
                .skip_while(|x| !x.to_lowercase().contains("password"))
                .nth(1)
                .map(|x| x.to_owned())
        });
    tracing::info!("Found password {:?}", password);

    let links_selector = Selector::parse("div > p a").unwrap();

    let chapters = doc
        .select(&links_selector)
        .filter_map(|x| x.value().attr("href").map(|y| (y, x.text().join(""))))
        .filter_map(|(href, link_text)| {
            Some(NewChapter {
                title: chapter_title_from_link(&link_text)?.to_owned(),
                book_id: *book_id,
                metadata: ChapterMetadata::TheWanderingInnPatreon {
                    url: href.to_owned(),
                    password: password.clone(),
                },
                published_at,
                html: None,
                epub: None,
            })
        })
        .collect();

    Ok(chapters)
}

#[tracing::instrument(
    name = "Getting chapter name from link.",
    level = "info"
    ret
)]
fn chapter_title_from_link(link: &str) -> Option<&str> {
    link.split('/').filter(|x| !x.trim().is_empty()).last()
}

#[tracing::instrument(name = "Fetching chapter text from link.", level = "info")]
pub async fn get_chapter_body(url: &str, password: Option<&str>) -> anyhow::Result<String> {
    let reqwest_client = reqwest::Client::builder().cookie_store(true).build()?;
    if let Some(password) = password {
        let mut form_data = HashMap::with_capacity(2);
        form_data.insert("post_password", password);
        form_data.insert("Submit", "Enter");
        let password_submit_result = reqwest_client
            .request(
                Method::POST,
                "https://wanderinginn.com/wp-login.php?action=postpass",
            )
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:109.0) Gecko/20100101 Firefox/110.0")
            .form(&form_data)
            .send()
            .await?;
        tracing::info!("Submitted password: {:?}", password_submit_result);
    }
    let res = reqwest_client.get(url).send().await?.text().await?;
    let doc = Html::parse_document(&res);
    let chapter_body_elem_selector = Selector::parse("div.entry-content > *").unwrap();

    let body = doc
        .select(&chapter_body_elem_selector)
        .filter(|x| !x.text().any(|t| t == "Next Chapter"))
        .filter(|x| !x.text().any(|t| t == "Previous Chapter"))
        .map(|x| x.html())
        .join("\n");
    if body.trim().is_empty() {
        bail!("Failed to find chapter body.");
    }
    Ok(body)
}
