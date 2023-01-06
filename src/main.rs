mod controllers;
mod error;
mod logging;
mod models;
mod providers;
mod tasks;
mod util;

use controllers::{books, chapters, subscribers, subscriptions};
use error::ApiResult;

use axum::Router;
use futures::Future;
use logging::configure_tracing;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool, Sqlite,
};
use std::{fs, net::SocketAddr};
use std::{path::Path, str::FromStr};
use tokio::signal;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct AppState {
    pool: Pool<Sqlite>,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ApiResult<()> {
    configure_tracing();

    let create_db = !Path::new("./data.db").try_exists()?;

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(SqliteConnectOptions::from_str("sqlite:data.db")?.create_if_missing(true))
        .await?;

    if create_db {
        new_db(pool.clone()).await?;
    }

    let cancel = tokio::spawn(signal::ctrl_c());
    tokio::pin!(cancel);
    let mut server = Box::pin(tokio::spawn(get_server_future(pool.clone())));
    let mut check_for_new_chapters = Box::pin(tokio::spawn(
        tasks::chapter_discovery::check_for_new_chap_loop(pool.clone()),
    ));
    let mut chapter_body_fetcher = Box::pin(tokio::spawn(
        tasks::chapter_body_hydration::check_for_bodiless_chap_loop(pool.clone()),
    ));
    let mut chapter_epub_converter = Box::pin(tokio::spawn(
        tasks::chapter_body_conversion::check_for_epubless_chap_loop(pool.clone()),
    ));
    let mut mailman = Box::pin(tokio::spawn(
        tasks::delivery::check_for_ready_delivery_loop(pool.clone()),
    ));
    loop {
        tokio::select! {
            x = &mut server => {
                error!("API server thread failed. Restarting the thread.");
                match x {
                    Ok(_) => error!("API Server returned OK. This should not be possible."),
                    Err(err) => error!(?err, "API Server has paniced. This should not be possible."),
                };
                server.set(tokio::spawn(get_server_future(pool.clone())));

            },
            x = &mut check_for_new_chapters => {
                error!("New chapter check thread failed. Restarting the thread.");
                match x {
                    Ok(_) => error!("New chapter check returned OK. This should not be possible."),
                    Err(err) => error!(?err, "New chapter check has paniced. This should not be possible."),
                };
                check_for_new_chapters.set(tokio::spawn(tasks::chapter_discovery::check_for_new_chap_loop(pool.clone())));

            }
            x = &mut chapter_body_fetcher => {
                error!("Chapter Body fetch thread failed. Restarting the thread.");
                match x {
                    Ok(_) => error!("Chapter body fetch returned OK. This should not be possible."),
                    Err(err) => error!(?err, "Chapter body fetch has paniced. This should not be possible."),
                };
                chapter_body_fetcher.set(tokio::spawn(tasks::chapter_body_hydration::check_for_bodiless_chap_loop(pool.clone())));

            }
            x = &mut chapter_epub_converter => {
                error!("Chapter epub converter thread failed. Restarting the thread.");
                match x {
                    Ok(_) => error!("Chapter epub converter thread returned OK. This should not be possible."),
                    Err(err) => error!(?err, "Chapter epub converter thread has paniced. This should not be possible."),
                };
                chapter_epub_converter.set(tokio::spawn(tasks::chapter_body_conversion::check_for_epubless_chap_loop(pool.clone())));
            }
            x = &mut mailman => {
                error!("Mailman thread failed. Restarting the thread.");
                match x {
                    Ok(_) => error!("Mailman thread returned OK. This should not be possible."),
                    Err(err) => error!(?err, "Mailman thread has paniced. This should not be possible."),
                };
                mailman.set(tokio::spawn(tasks::delivery::check_for_ready_delivery_loop(pool.clone())));
            }
            _ = &mut cancel => {
                println!("Received exit signal, exiting.");
                break;
            }
        }
    }
    Ok(())
}

fn get_server_future(pool: Pool<Sqlite>) -> impl Future<Output = Result<(), hyper::Error>> {
    let state = AppState { pool };

    let subscribers = subscribers::router();
    let books = books::router();
    let chapters = chapters::router();
    let subscriptions = subscriptions::router();

    let app = Router::new()
        .merge(subscribers)
        .merge(chapters)
        .merge(books)
        .merge(subscriptions)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr).serve(app.into_make_service_with_connect_info::<SocketAddr>())
}

async fn new_db(pool: Pool<Sqlite>) -> ApiResult<()> {
    warn!("Running schema setup script");
    sqlx::query(&String::from_utf8_lossy(include_bytes!(
        "../create_tables.sql"
    )))
    .execute(&pool)
    .await
    .unwrap();

    Ok(())
}
