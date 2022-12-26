CREATE TABLE books (
  id BLOB PRIMARY KEY NOT NULL,
  title TEXT NOT NULL,
  author TEXT NOT NULL,
  metadata TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE chapters (
  id BLOB PRIMARY KEY NOT NULL,
  book_id BLOB NOT NULL,
  title TEXT NOT NULL,
  metadata TEXT NOT NULL,
  html BLOB,
  epub BLOB,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,

  CONSTRAINT fk_book_id FOREIGN KEY(book_id) REFERENCES books(id) ON DELETE CASCADE
);

CREATE TABLE subscribers (
  id BLOB PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  kindle_email TEXT,
  pushover_key TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE subscriptions (
  id BLOB PRIMARY KEY NOT NULL,
  subscriber_id BLOB NOT NULL,
  chunk_size NUMBER NOT NULL DEFAULT 1,
  book_id BLOB NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,

  CONSTRAINT fk_book_id FOREIGN KEY(book_id) REFERENCES books(id) ON DELETE CASCADE
  CONSTRAINT fk_subscriber_id FOREIGN KEY(subscriber_id) REFERENCES subscribers(id) ON DELETE CASCADE
);

