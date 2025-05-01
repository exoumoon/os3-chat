use crate::{models::Account, state::SharedState};
use argon2::{Argon2, PasswordHash, password_hash::SaltString};
use argon2::{PasswordHasher, PasswordVerifier};
use axum::{Json, debug_handler, extract::State, http::StatusCode};
use rand_core::OsRng;
use serde::Deserialize;
use std::borrow::Cow;
use tracing::instrument;

#[derive(Deserialize)]
#[serde(try_from = "UncheckedCredentials")]
pub struct Credentials {
    username: String,
    password: String,
}

impl Credentials {
    pub const MIN_USERNAME_LEN: usize = 2;
    pub const MIN_PASSWORD_LEN: usize = 8;
}

#[derive(Deserialize)]
pub struct UncheckedCredentials {
    username: String,
    password: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CredentialsError {
    #[error("The provided username is too short")]
    ShortUsername,
    #[error("The provided password is too short")]
    ShortPassword,
}

impl TryFrom<UncheckedCredentials> for Credentials {
    type Error = CredentialsError;

    fn try_from(unchecked_credentials: UncheckedCredentials) -> Result<Self, Self::Error> {
        let trimmed_username = unchecked_credentials.username.trim();
        if trimmed_username.len() < Self::MIN_USERNAME_LEN {
            return Err(CredentialsError::ShortUsername);
        }
        if unchecked_credentials.password.len() < Self::MIN_PASSWORD_LEN {
            return Err(CredentialsError::ShortPassword);
        }

        Ok(Self {
            username: trimmed_username.to_string(),
            password: unchecked_credentials.password,
        })
    }
}

#[instrument(skip(shared_state, credentials), fields(username = credentials.username))]
#[debug_handler]
pub async fn register(
    State(shared_state): State<SharedState>,
    Json(credentials): Json<Credentials>,
) -> Result<StatusCode, StatusCode> {
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(credentials.password.as_bytes(), &salt)
        .inspect_err(|error| tracing::error!(?error, "Failed to hash password"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let hash_string = password_hash.to_string();

    let _ = sqlx::query!(
        "INSERT INTO accounts (username, password_hash) VALUES (?, ?)",
        credentials.username,
        hash_string,
    )
    .execute(&shared_state.db_pool)
    .await
    .inspect(|_| tracing::info!("Registered new account"))
    .map_err(|error| {
        let code_non_unique = Cow::Borrowed("2067");
        match error {
            sqlx::Error::Database(error) if error.code() == Some(code_non_unique) => {
                tracing::warn!("Rejecting registration: username is taken");
                StatusCode::CONFLICT
            }
            _ => {
                tracing::error!(?error, "Failed to register account");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    })?;

    Ok(StatusCode::CREATED)
}

#[instrument(skip(shared_state, credentials), fields(username = credentials.username))]
#[debug_handler]
pub async fn login(
    State(shared_state): State<SharedState>,
    Json(credentials): Json<Credentials>,
) -> Result<StatusCode, StatusCode> {
    let account = sqlx::query_as!(
        Account,
        "SELECT * FROM accounts WHERE username = ?",
        credentials.username
    )
    .fetch_one(&shared_state.db_pool)
    .await
    .map_err(|error| match error {
        sqlx::Error::RowNotFound => {
            tracing::warn!("Login failed: no account with such username");
            StatusCode::UNAUTHORIZED
        }
        _ => {
            tracing::error!(?error, "Failed to fetch account from database");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    let stored_hash = PasswordHash::try_from(account.password_hash.as_str()).unwrap();
    match Argon2::default().verify_password(credentials.password.as_bytes(), &stored_hash) {
        Ok(()) => {
            tracing::debug!("Login successful");
            Ok(StatusCode::OK)
        }
        Err(error) => {
            tracing::warn!(%error, "Login failed");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
