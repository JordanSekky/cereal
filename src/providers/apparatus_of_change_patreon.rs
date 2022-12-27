use std::env;

use anyhow::anyhow;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use futures::future::try_join_all;
use itertools::Itertools;
use mailparse::MailHeaderMap;
use rusoto_core::credential::StaticProvider;
use rusoto_core::HttpClient;
use rusoto_core::Region;
use rusoto_s3::GetObjectRequest;
use rusoto_s3::ListObjectsV2Request;
use rusoto_s3::Object;
use rusoto_s3::S3Client;
use rusoto_s3::S3;
use scraper::{Html, Selector};
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use crate::models::ChapterMetadata;

use super::NewChapter;
use super::NewChapterProvider;

pub struct ApparatusOfChangePatreonNewChapterProvider;

#[async_trait]
impl NewChapterProvider for ApparatusOfChangePatreonNewChapterProvider {
    #[tracing::instrument(skip(self), level = "info", ret)]
    async fn fetch_new_chapters(
        &self,
        book_id: &Uuid,
        last_publish_date: Option<&DateTime<Utc>>,
    ) -> anyhow::Result<Vec<NewChapter>> {
        return get_chapters(book_id, last_publish_date).await;
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
    match &subject {
        Some(x) => {
            if !x.to_lowercase().contains("apparatus") {
                // Not an apparatus of change email, return zero chapters.
                return Ok(Vec::with_capacity(0));
            }
        }
        // No subject, return zero chapters.
        None => return Ok(Vec::with_capacity(0)),
    }
    tracing::info!(
        "Found apparatus of change patreon email with subject: {}",
        subject.as_ref().unwrap()
    );

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
    let doc = Html::parse_document(&body);
    tracing::info!("Parsed body",);
    let body = doc
        .select(&Selector::parse("td > div > span > div > div > div > div + div").unwrap())
        .map(|x| x.html())
        .next()
        .ok_or_else(|| anyhow!("No matching body in html."))?;
    let chapter = NewChapter {
        title: chapter_title_from_subject(&subject.unwrap())
            .ok_or_else(|| anyhow!("Failed to find chapter title from email subject"))?
            .into(),
        book_id: *book_id,
        html: Some(body.into_bytes()),
        epub: None,
        published_at,
        metadata: ChapterMetadata::ApparatusOfChangePatreon,
    };
    Ok(Vec::from([chapter]))
}

fn chapter_title_from_subject(subject: &str) -> Option<&str> {
    subject
        .split('"')
        .nth(1)
        .map(|x| x.trim_start_matches("Apparatus Of Change - "))
}
