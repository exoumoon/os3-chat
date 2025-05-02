CREATE TABLE room_membership (
    account_id INTEGER NOT NULL,
    room_id INTEGER NOT NULL,
    joined_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    PRIMARY KEY(account_id, room_id),
    FOREIGN KEY(account_id) REFERENCES accounts(id),
    FOREIGN KEY(room_id) REFERENCES rooms(id)
);

