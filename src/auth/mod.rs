use crate::state::SharedState;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Redirect};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::Cookie;
use chrono::NaiveDateTime;
use sqlx::query;
use tracing::{Level, instrument};

pub const SESSION_COOKIE_NAME: &str = "session-token";

#[derive(Debug)]
#[must_use]
pub struct Session(pub AuthorizedAccount);

#[derive(Debug)]
#[must_use]
pub struct AuthorizedAccount {
    pub id: i64,
    pub username: String,
    pub registered_at: NaiveDateTime,
    pub session_id: i64,
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
        let session_token = cookies.get(SESSION_COOKIE_NAME).map(Cookie::value_trimmed);

        if let Some(token) = session_token {
            let session_query = query!(
                "SELECT * FROM sessions WHERE token = ? AND expired = 0",
                token
            );
            let session = session_query
                .fetch_one(&state.db_pool)
                .await
                .map_err(|_| RejectionCause::ExpiredSession)?;
            let account_record = query!("SELECT * FROM accounts WHERE id = ?", session.account_id)
                .fetch_one(&state.db_pool)
                .await
                .map_err(|_| RejectionCause::InvalidSession)?;

            let authorized_account = AuthorizedAccount {
                id: account_record.id,
                username: account_record.username,
                registered_at: account_record.registered_at,
                session_id: session.id,
            };

            tracing::trace!(?authorized_account, "Cookie authorization successful");
            Ok(Self(authorized_account))
        } else {
            Err(RejectionCause::NoSessionCookie)
        }
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
