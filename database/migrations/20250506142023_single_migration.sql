-- Tables.
CREATE TABLE accounts (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE CHECK (length(username) > 0),
    password_hash TEXT NOT NULL,
    registered_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE sessions (
    token BYTES NOT NULL PRIMARY KEY,
    account_id INTEGER NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expired BOOLEAN NOT NULL DEFAULT 0,
    FOREIGN KEY(account_id) REFERENCES accounts(id)
);

CREATE TABLE rooms (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE CHECK (length(name) > 0),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE messages (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    sender_id INTEGER NOT NULL,
    room_id INTEGER NOT NULL,
    content TEXT,
    sent_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    file_upload_id INTEGER,

    FOREIGN KEY(sender_id) REFERENCES accounts(id),
    FOREIGN KEY(room_id) REFERENCES rooms(id),
    FOREIGN KEY(file_upload_id) REFERENCES file_uploads(id)
);

CREATE TABLE room_membership (
    member_id INTEGER NOT NULL,
    room_id INTEGER NOT NULL,
    joined_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY(member_id, room_id),
    FOREIGN KEY(member_id) REFERENCES accounts(id),
    FOREIGN KEY(room_id) REFERENCES rooms(id)
);

CREATE TABLE file_uploads (
    uuid     BLOB NOT NULL PRIMARY KEY,
    filename TEXT NOT NULL
);

-- Create a public room for everyone to be in by default.
INSERT INTO rooms (name) VALUES ('public');

CREATE TRIGGER auto_join_public_room
AFTER INSERT ON accounts
BEGIN
    INSERT INTO room_membership (account_id, room_id)
    VALUES (NEW.id, 1);
END;

-- Show a message when somebody uploads a file.
CREATE TRIGGER insert_message_on_file_upload
AFTER INSERT ON file_uploads
BEGIN
    INSERT INTO messages (sender_account_id, room_id, content, sent_at, file_upload_id)
    VALUES (
        NEW.uploader_account_id,
        NEW.room_id,
        'Uploaded a file:',
        CURRENT_TIMESTAMP,
        NEW.id
    );
END;
