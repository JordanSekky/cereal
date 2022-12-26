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
  published_at TEXT,
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
  last_delivered_chapter_id BLOB,
  last_delivered_chapter_created_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,

  CONSTRAINT fk_book_id FOREIGN KEY(book_id) REFERENCES books(id) ON DELETE CASCADE
  CONSTRAINT fk_subscriber_id FOREIGN KEY(subscriber_id) REFERENCES subscribers(id) ON DELETE CASCADE
  CONSTRAINT fk_chapter_id FOREIGN KEY(last_delivered_chapter_id) REFERENCES chapters(id) ON DELETE SET NULL
);

-- INSERT INTO books(id, title, author, metadata, created_at, updated_at) 
-- VALUES(x'7bc0a84802b14c788917264cd38860c4', 'Pale', 'wildbow', '"Pale"', '2022-12-26T04:50:42.879414Z', '2022-12-26T04:50:42.879414Z');

INSERT INTO books(id, title, author, metadata, created_at, updated_at) 
VALUES(x'a6aede4d9cdf44a3910e3e76da3737c1', 'The Wandering Inn (Patreon)', 'pireataba', '"TheWanderingInnPatreon"', '2022-12-26T04:50:42.879414Z', '2022-12-26T04:50:42.879414Z');