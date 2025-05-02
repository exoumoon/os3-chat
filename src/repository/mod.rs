pub const CODE_NON_UNIQUE: &str = "2067";

pub mod account;
pub mod message;

#[derive(Debug, Clone)]
#[must_use]
pub struct Repository {
    pub accounts: account::AccountRepository,
}

impl Repository {
    pub const fn new(connection: sqlx::SqlitePool) -> Self {
        let accounts = account::AccountRepository { connection };
        Self { accounts }
    }
}
