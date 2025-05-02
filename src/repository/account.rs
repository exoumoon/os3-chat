use super::CODE_NON_UNIQUE;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::NaiveDateTime;
use rand_core::OsRng;
use sqlx::SqlitePool;
use tracing::instrument;
use uuid::Uuid;

#[derive(sqlx::FromRow, Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub registered_at: NaiveDateTime,
}

#[derive(sqlx::FromRow, Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub id: i64,
    pub token: String,
    pub account_id: i64,
    pub created_at: NaiveDateTime,
    pub expired: bool,
}

#[derive(Debug, Clone)]
#[must_use]
pub struct AccountRepository {
    pub(super) connection: SqlitePool,
}

impl AccountRepository {
    #[instrument(skip(self))]
    pub async fn find_by_id(&self, account_id: i64) -> Result<Option<Account>, sqlx::Error> {
        tracing::trace!("Searching for account in the repository");
        sqlx::query_as!(Account, "SELECT * FROM accounts WHERE id = ?", account_id)
            .fetch_optional(&self.connection)
            .await
    }

    #[instrument(skip(self, password))]
    pub async fn register(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Account, RegistrationError> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .inspect_err(|error| tracing::error!(?error, "Failed to hash password"))
            .map_err(RegistrationError::Hash)?;

        let password_hash_str = password_hash.to_string();
        let query = sqlx::query_as!(
            Account,
            "INSERT INTO accounts (username, password_hash) VALUES (?, ?) RETURNING id, username, password_hash, registered_at",
            username,
            password_hash_str,
        );

        query
            .fetch_one(&self.connection)
            .await
            .inspect(|_| tracing::debug!("Sucessfully registered new account"))
            .map_err(|error| match error {
                sqlx::Error::Database(error)
                    if error.code().is_some_and(|code| CODE_NON_UNIQUE == code) =>
                {
                    tracing::debug!(?error, "Rejecting registration: username is taken");
                    RegistrationError::NameTaken
                }
                _ => {
                    tracing::error!(?error, "Database error during registration");
                    RegistrationError::Database(error)
                }
            })
    }

    #[instrument(skip(self, password))]
    pub async fn login(&self, username: &str, password: &str) -> Result<Session, LoginError> {
        let query = sqlx::query_as!(
            Account,
            "SELECT * FROM accounts WHERE username = ?",
            username
        );

        let account = query
            .fetch_one(&self.connection)
            .await
            .map_err(|error| match error {
                sqlx::Error::RowNotFound => {
                    tracing::debug!("Rejecting login attempt: invalid credentials");
                    LoginError::InvalidCredentials
                }
                _ => {
                    tracing::error!(?error, "Database error during login");
                    LoginError::Database(error)
                }
            })?;

        let stored_hash =
            PasswordHash::try_from(account.password_hash.as_str()).map_err(LoginError::Hash)?;
        Argon2::default()
            .verify_password(password.as_bytes(), &stored_hash)
            .map_err(|error| match error {
                argon2::password_hash::Error::Password => {
                    tracing::debug!("Rejecting login attempt: invalid credentials");
                    LoginError::InvalidCredentials
                }
                _ => {
                    tracing::error!(?error, "Failed to verify password hash");
                    LoginError::Hash(error)
                }
            })?;

        let session_token = Uuid::new_v4();
        let session_token_string = session_token.to_string();
        let created_session = sqlx::query_as!(
            Session,
            "INSERT INTO sessions (token, account_id) VALUES (?, ?) RETURNING *",
            session_token_string,
            account.id
        )
        .fetch_one(&self.connection)
        .await?;

        tracing::debug!("Login successful, created new session");
        Ok(created_session)
    }

    #[instrument(skip(self), err(Debug))]
    pub async fn expire_session(&self, session_id: i64) -> Result<(), sqlx::Error> {
        let update_query = sqlx::query!(
            "UPDATE sessions SET expired = 1 WHERE id = ? RETURNING account_id",
            session_id
        );
        let account_id = update_query.fetch_one(&self.connection).await?.account_id;
        let owner = sqlx::query!("SELECT username FROM accounts WHERE id = ?", account_id)
            .fetch_one(&self.connection)
            .await?;
        tracing::debug!(owner = owner.username, "Expired session");
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RegistrationError {
    #[error("An account with this username already exists")]
    NameTaken,

    #[error("Failed to hash the password")]
    Hash(argon2::password_hash::Error),

    #[error("Internal database error")]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("An account with this username already exists")]
    InvalidCredentials,

    #[error("Failed to hash the password")]
    Hash(argon2::password_hash::Error),

    #[error("Internal database error")]
    Database(#[from] sqlx::Error),
}
