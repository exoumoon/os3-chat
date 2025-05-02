use std::path::PathBuf;

use chrono::NaiveDateTime;

#[derive(sqlx::FromRow, Debug)]
#[must_use]
pub struct Upload {
    pub id: i64,
    pub uploader_account_id: i64,
    pub room_id: i64,
    pub original_filename: PathBuf,
    pub store_path: PathBuf,
    pub uploaded_at: NaiveDateTime,
}
