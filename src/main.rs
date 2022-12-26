mod controllers;
mod error;
mod logging;
mod models;
mod util;

use controllers::{books, chapters, subscribers, subscriptions};
use error::Result;

use axum::Router;
use logging::configure_tracing;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool, Sqlite,
};
use std::str::FromStr;
use std::{fs, net::SocketAddr};
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pool: Pool<Sqlite>,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() -> Result<()> {
    configure_tracing();

    let _ = fs::remove_file("data.db");
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(SqliteConnectOptions::from_str("sqlite:data.db")?.create_if_missing(true))
        .await?;

    new_db(pool.clone()).await?;

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

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;
    Ok(())
}

async fn new_db(pool: Pool<Sqlite>) -> Result<()> {
    sqlx::query(&String::from_utf8_lossy(include_bytes!(
        "../create_tables.sql"
    )))
    .execute(&pool)
    .await
    .unwrap();

    Ok(())
}
