use axum::extract::{FromRef, FromRequestParts};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Redirect};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::Cookie;
use chrono::NaiveDateTime;
use sqlx::query;
use tracing::{Level, instrument};
use uuid::Uuid;

use crate::state::SharedState;

pub const SESSION_COOKIE_NAME: &str = "session-token";

#[derive(Debug)]
#[must_use]
pub struct Session(pub AuthorizedAccount);

#[derive(Debug)]
#[must_use]
pub struct AuthorizedAccount {
    pub username: String,
    pub registered_at: NaiveDateTime,
    pub session_token: Uuid,
}

impl<S> FromRequestParts<S> for Session
where
    SharedState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = RejectionCause;

    #[instrument(name = "auth_layer", skip_all, err(Debug, level = Level::WARN))]
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = SharedState::from_ref(state);
        let cookies = CookieJar::from_headers(&parts.headers);
        let token: Uuid = cookies
            .get(SESSION_COOKIE_NAME)
            .map(Cookie::value_trimmed)
            .and_then(|v| v.parse().ok())
            .ok_or(RejectionCause::InvalidSession)?;

        let token_string = token.to_string();
        let session = query!(
            "SELECT * FROM sessions WHERE token = ? AND expired = 0",
            token_string
        )
        .fetch_one(&state.db_pool)
        .await
        .map_err(|_| RejectionCause::ExpiredSession)?;

        let account_record = query!("SELECT * FROM accounts WHERE username = ?", session.account)
            .fetch_one(&state.db_pool)
            .await
            .map_err(|_| RejectionCause::InvalidSession)?;

        let authorized_account = AuthorizedAccount {
            username: account_record.username,
            registered_at: account_record.registered_at,
            session_token: token,
        };

        tracing::trace!(?authorized_account, "Cookie-based auth completed");

        Ok(Self(authorized_account))
    }
}

#[derive(Debug)]
#[must_use]
pub enum RejectionCause {
    NoSessionCookie,
    InvalidSession,
    ExpiredSession,
    InternalServerError,
}

impl IntoResponse for RejectionCause {
    fn into_response(self) -> axum::response::Response {
        let redirect = Redirect::to("/account");
        match self {
            Self::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            Self::InvalidSession | Self::NoSessionCookie | Self::ExpiredSession => {
                redirect.into_response()
            }
        }
    }
}
