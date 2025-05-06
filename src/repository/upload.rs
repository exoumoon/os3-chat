use std::path::PathBuf;

use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(sqlx::FromRow, Debug)]
#[must_use]
pub struct Upload {
    pub uuid: String,
    pub filename: PathBuf,
}

impl Upload {
    pub fn store_path(&self) -> Result<PathBuf, std::io::Error> {
        let path_str = format!(
            "./database/file_uploads/{}_{}",
            self.uuid,
            self.filename.to_string_lossy()
        );
        PathBuf::from(path_str).canonicalize()
    }
}

#[derive(Debug, Clone)]
#[must_use]
pub struct UploadRepository {
    pub(super) connection: SqlitePool,
}

impl UploadRepository {
    pub async fn find(&self, uuid: Uuid) -> Result<Option<Upload>, sqlx::Error> {
        let uuid_str = uuid.to_string();
        sqlx::query_as!(
            Upload,
            "SELECT * FROM file_uploads WHERE uuid = ?",
            uuid_str
        )
        .fetch_optional(&self.connection)
        .await
    }
}
