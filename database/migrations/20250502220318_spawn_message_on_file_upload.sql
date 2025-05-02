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
