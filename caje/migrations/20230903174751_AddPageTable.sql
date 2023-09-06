-- Add migration script here
CREATE TABLE
  Pages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    method TEXT NOT NULL,
    url TEXT NOT NULL
  );
