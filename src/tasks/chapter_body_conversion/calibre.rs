use anyhow::{bail, Context, Result};
use rand::Rng;
use std::fs;
use tokio::process::Command;
use tracing::{info, info_span, instrument, Instrument};

#[instrument(
name = "Converting to mobi",
err,
level = "info"
skip(chapter_body),
)]
pub async fn generate_epub(
    input_extension: &str,
    chapter_body: &[u8],
    cover_title: &str,
    book_title: &str,
    author: &str,
) -> Result<Vec<u8>> {
    let file_name: String = rand::thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();
    let in_path = format!("/tmp/{}.{}", file_name, input_extension);
    let out_path = format!("/tmp/{}.epub", file_name);
    fs::write(&in_path, chapter_body)?;
    let output = Command::new("ebook-convert")
        .arg(&in_path)
        .arg(&out_path)
        .arg("--filter-css")
        .arg(r#""font-family,color,background""#)
        .arg("--authors")
        .arg(author)
        .arg("--title")
        .arg(cover_title)
        .arg("--series")
        .arg(book_title)
        .arg("--output-profile")
        .arg("kindle_oasis")
        .output()
        .instrument(info_span!(
            "Converting file from {} to {}",
            in_path,
            out_path
        ))
        .await
        .with_context(|| "Failed to spawn ebook-convert. Perhaps calibre is not installed?")?;
    info!(
        stdout = ?String::from_utf8_lossy(&output.stdout),
        stderr = ?String::from_utf8_lossy(&output.stderr),
        status_code = ?output.status
    );
    if !output.status.success() {
        bail!("Calibre conversion failed with status {:?}", output.status);
    }
    let bytes = fs::read(&out_path)?;
    fs::remove_file(&in_path)?;
    fs::remove_file(&out_path)?;
    Ok(bytes)
}
