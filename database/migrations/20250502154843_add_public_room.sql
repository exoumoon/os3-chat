INSERT INTO rooms (name) VALUES ('public');

CREATE TRIGGER auto_join_public_room
AFTER INSERT ON accounts
BEGIN
    INSERT INTO room_membership (account_id, room_id)
    VALUES (NEW.id, 1);
END;
