pub const CODE_NON_UNIQUE: &str = "2067";

pub mod account;
pub mod message;
pub mod room;
pub mod upload;

#[derive(Debug, Clone)]
#[must_use]
pub struct Repository {
    pub accounts: account::AccountRepository,
    pub rooms: room::RoomRepository,
}

impl Repository {
    pub fn new(connection: sqlx::SqlitePool) -> Self {
        let accounts = account::AccountRepository {
            connection: connection.clone(),
        };
        let rooms = room::RoomRepository { connection };
        Self { accounts, rooms }
    }
}
