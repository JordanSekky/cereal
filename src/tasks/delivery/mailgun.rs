use anyhow::{bail, Error};
use reqwest::multipart::Part;
use std::env;

#[derive(Clone)]
struct Attachment {
    pub content_type: String,
    pub file_name: String,
    pub bytes: Vec<u8>,
}

impl std::fmt::Debug for Attachment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Attachment")
            .field("content_type", &self.content_type)
            .field("file_name", &self.file_name)
            .field("bytes_length", &self.bytes.len())
            .finish()
    }
}

#[derive(Clone)]
struct Message {
    to: String,
    subject: String,
    text: Option<String>,
    html: Option<String>,
    attachment: Option<Attachment>,
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Message")
            .field("to", &self.to)
            .field("subject", &self.subject)
            .field(
                "text_length",
                &self.text.as_ref().map(|x| x.as_bytes().len()),
            )
            .field(
                "html_length",
                &self.html.as_ref().map(|x| x.as_bytes().len()),
            )
            .field("attachment", &self.attachment)
            .finish()
    }
}

impl Message {
    fn new(
        to: &str,
        subject: &str,
        text: Option<&str>,
        html: Option<&str>,
        attachment: Option<Attachment>,
    ) -> Self {
        Self {
            to: to.into(),
            subject: subject.into(),
            text: text.map(String::from),
            html: html.map(String::from),
            attachment,
        }
    }
}

#[tracing::instrument(
name = "Sending an email",
err,
level = "info"
skip(message)
)]
async fn send_message(message: Message) -> Result<(), Error> {
    let client = reqwest::Client::new();
    let mut form = reqwest::multipart::Form::new()
        .text("to", message.to)
        .text("subject", message.subject)
        .text("from", env::var("CEREAL_FROM_EMAIL_ADDRESS").unwrap());
    if let Some(text) = message.text {
        form = form.text("text", text);
    }
    if let Some(html) = message.html {
        form = form.text("html", html);
    }
    if let Some(attachment) = message.attachment {
        form = form.part(
            "attachment",
            Part::bytes(attachment.bytes)
                .file_name(attachment.file_name)
                .mime_str(&attachment.content_type)?,
        );
    }
    let mailgun_api_key =
        env::var("CEREAL_MAILGUN_API_KEY").expect("Mailgun API key not provided.");
    let send_email_response = client
        .post(env::var("CEREAL_MAILGUN_API_ENDPOINT").unwrap())
        .basic_auth("api", Some(mailgun_api_key))
        .multipart(form)
        .send()
        .await?;
    if !send_email_response.status().is_success() {
        bail!(
            "Received unsuccessful status code from mailgun: {}",
            send_email_response.status()
        );
    };
    Ok(())
}

#[tracing::instrument(
name = "Sending a epub email",
err,
level = "info"
skip(bytes, email),
)]
pub async fn send_epub_file(
    bytes: &[u8],
    email: &str,
    chapter_title: &str,
    subject: &str,
) -> Result<(), Error> {
    let attachment = Attachment {
        content_type: "application/epub+zip".into(),
        file_name: sanitize_filename::sanitize(format!("{}.epub", &chapter_title)),
        bytes: Vec::from(bytes),
    };
    let message = Message::new(
        email,
        subject,
        Some(subject),
        Some(subject),
        Some(attachment),
    );
    send_message(message).await
}
