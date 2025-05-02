CREATE TABLE file_uploads (
    id                  INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    uploader_account_id INTEGER NOT NULL,
    room_id             INTEGER NOT NULL,
    original_filename   TEXT NOT NULL,
    store_path          TEXT NOT NULL,
    uploaded_at         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY(uploader_account_id) REFERENCES accounts(id),
    FOREIGN KEY(room_id)             REFERENCES rooms(id)
);
