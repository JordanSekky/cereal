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

INSERT INTO books(id, title, author, metadata, created_at, updated_at) 
VALUES(x'4066433f24ab4cfcab4ac98cb95682d1', 'He Who Fights With Monsters', 'Shirtaloon (Travis Deverell)', '{"RoyalRoad":{"book_id": 26294}}', '2022-12-26T04:50:42.879414Z', '2022-12-26T04:50:42.879414Z');

INSERT INTO books(id, title, author, metadata, created_at, updated_at) 
VALUES(x'e725e8bcc82b4f26a29c4ee2df932236', 'Beware Of Chicken', 'Casualfarmer', '{"RoyalRoad":{"book_id": 39408}}', '2022-12-26T04:50:42.879414Z', '2022-12-26T04:50:42.879414Z');

INSERT INTO books(id, title, author, metadata, created_at, updated_at) 
VALUES(x'2211075e9cf34605b1be1e772066b7cc', 'This Used to be About Dungeons', 'Alexander Wales', '{"RoyalRoad":{"book_id": 45534}}', '2022-12-26T04:50:42.879414Z', '2022-12-26T04:50:42.879414Z');

INSERT INTO books(id, title, author, metadata, created_at, updated_at) 
VALUES(x'7bc0a84802b14c788917264cd38860c4', 'Pale', 'wildbow', '"Pale"', '2022-12-26T04:50:42.879414Z', '2022-12-26T04:50:42.879414Z');

INSERT INTO books(id, title, author, metadata, created_at, updated_at) 
VALUES(x'a6aede4d9cdf44a3910e3e76da3737c1', 'The Wandering Inn', 'pireataba', '"TheWanderingInnPatreon"', '2022-12-26T04:50:42.879414Z', '2022-12-26T04:50:42.879414Z');

INSERT INTO books(id, title, author, metadata, created_at, updated_at) 
VALUES(x'ad382ba41f224c8b926cbcc51de08b11', 'Apparatus of Change', 'argusthecat', '"ApparatusOfChangePatreon"', '2022-12-26T04:50:42.879414Z', '2022-12-26T04:50:42.879414Z');

INSERT INTO books(id, title, author, metadata, created_at, updated_at) 
VALUES(x'6c24ba69819f43e494e21cb5b38d0689', 'The Daily Grind', 'argusthecat', '"TheDailyGrindPatreon"', '2022-12-26T04:50:42.879414Z', '2022-12-26T04:50:42.879414Z');

INSERT INTO subscribers(id, name, kindle_email, pushover_key, created_at, updated_at)
VALUES(x'bbb382d205264e02b5d8f47c30bb69c9', 'Jordan Sechler', 'i79KZi6RdyoCczQ@kindle.com', 'ucmnzvepd3mqy2rr5nj6exz49obtoj', '2022-12-26T04:50:42.879414Z', '2022-12-26T04:50:42.879414Z');

INSERT INTO subscribers(id, name, kindle_email, pushover_key, created_at, updated_at)
VALUES(x'f432e231aa32447a9894fd999b97da72', 'Brett Clark', 'clark1872_letsgo@kindle.com', NULL, '2022-12-26T04:50:42.879414Z', '2022-12-26T04:50:42.879414Z');