-- Add migration script here
CREATE TABle
  Pages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    method TEXT NOT NULL,
    url TEXT NOT NULL
  );
