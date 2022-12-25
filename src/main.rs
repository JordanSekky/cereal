use axum::{
    routing::{delete, get, post},
    Router,
};
use error::Result;
mod models;
use std::str::FromStr;
use std::{fs, net::SocketAddr};
mod controllers;
use controllers::books;
mod error;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool, Sqlite,
};

#[derive(Clone)]
pub struct AppState {
    pool: Pool<Sqlite>,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() -> Result<()> {
    let _ = fs::remove_file("data.db");
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(SqliteConnectOptions::from_str("sqlite:data.db")?.create_if_missing(true))
        .await?;

    new_db(pool.clone()).await?;

    let state = AppState { pool };

    let app = Router::new()
        .route("/createBook", post(books::create_book_handler))
        .route("/updateBook", post(books::update_book_handler))
        .route("/getBook", get(books::get_book_handler))
        .route("/listBooks", get(books::list_books_handler))
        .route("/deleteBook", delete(books::delete_book_handler))
        .with_state(state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn new_db(pool: Pool<Sqlite>) -> Result<()> {
    // Make a simple query to return the given parameter (use a question mark `?` instead of `$1` for MySQL)
    sqlx::query(&String::from_utf8_lossy(include_bytes!(
        "../create_tables.sql"
    )))
    .execute(&pool)
    .await
    .unwrap();

    Ok(())
}
