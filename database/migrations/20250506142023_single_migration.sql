-- Tables.
CREATE TABLE accounts (
    username TEXT NOT NULL UNIQUE PRIMARY KEY,
    password_hash TEXT NOT NULL,
    registered_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE sessions (
    token TEXT NOT NULL PRIMARY KEY,
    account TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expired BOOLEAN NOT NULL DEFAULT 0,
    FOREIGN KEY(account) REFERENCES accounts(username)
);

CREATE TABLE rooms (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE messages (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    sender TEXT NOT NULL,
    room_id INTEGER NOT NULL,
    text TEXT,
    sent_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    file_upload_uuid TEXT,

    FOREIGN KEY(sender) REFERENCES accounts(username),
    FOREIGN KEY(room_id) REFERENCES rooms(id),
    FOREIGN KEY(file_upload_uuid) REFERENCES file_uploads(uuid)
);

CREATE TABLE room_membership (
    member TEXT NOT NULL,
    room_id INTEGER NOT NULL,
    joined_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY(member, room_id),
    FOREIGN KEY(member) REFERENCES accounts(username),
    FOREIGN KEY(room_id) REFERENCES rooms(id)
);

CREATE TABLE file_uploads (
    uuid     TEXT NOT NULL PRIMARY KEY,
    filename TEXT NOT NULL
);

-- Create a public room for everyone to be in by default.
INSERT INTO rooms (id, name) VALUES (1, 'public');

CREATE TRIGGER auto_join_public_room
AFTER INSERT ON accounts
BEGIN
    INSERT INTO room_membership (member, room_id)
    VALUES (NEW.username, 1);
END;

-- An inaccessible "admin" account.
INSERT INTO accounts (username, password_hash) VALUES ('admin', '!');
INSERT INTO messages (sender, room_id, text) VALUES ('admin', 1, 'system diagnostic, database begins after this message');
