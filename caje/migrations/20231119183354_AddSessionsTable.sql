-- Add migration script here
CREATE TABLE
  Sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    session_id TEXT NOT NULL
  );

CREATE UNIQUE INDEX idx_session_id ON Sessions (session_id);
