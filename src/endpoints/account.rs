use crate::auth::SESSION_COOKIE_NAME;
use crate::repository::account::{LoginError, RegistrationError};
use crate::state::SharedState;
use axum::response::{IntoResponse, Redirect};
use axum::{Form, debug_handler, extract::State, http::StatusCode};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_valid::Valid;
use serde::Deserialize;
use tracing::instrument;
use validator::Validate;

#[derive(Deserialize, Validate, Debug)]
#[must_use]
pub struct CredentialsForm {
    username: String,
    password: String,
    action: SubmitAction,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
#[must_use]
pub enum SubmitAction {
    Register,
    Login,
}

#[derive(Debug)]
#[must_use]
pub enum AuthResult {
    Registered(Redirect),
    LoggedIn(CookieJar, Redirect),
    Error(StatusCode),
}

#[instrument(skip_all, fields(username = credentials.username))]
#[debug_handler]
pub async fn submit(
    State(state): State<SharedState>,
    Valid(credentials): Valid<Form<CredentialsForm>>,
) -> AuthResult {
    match credentials.action {
        SubmitAction::Register => {
            let result = state
                .repository
                .accounts
                .register(&credentials.username, &credentials.password)
                .await;
            match result {
                Ok(_account) => AuthResult::Registered(Redirect::to("/")),
                Err(RegistrationError::UsernameTaken) => AuthResult::Error(StatusCode::CONFLICT),
                Err(_internal) => AuthResult::Error(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }

        SubmitAction::Login => {
            let result = state
                .repository
                .accounts
                .login(&credentials.username, &credentials.password)
                .await;
            match result {
                Ok(session) => {
                    let base_cookie = Cookie::new(SESSION_COOKIE_NAME, session.token);
                    let cookie = Cookie::build(base_cookie)
                        .path("/")
                        .http_only(true)
                        .secure(false)
                        .same_site(SameSite::Lax);
                    let jar = CookieJar::new().add(cookie);
                    AuthResult::LoggedIn(jar, Redirect::to("/chat"))
                }

                Err(login_error) => match login_error {
                    LoginError::InvalidCredentials => AuthResult::Error(StatusCode::UNAUTHORIZED),
                    _ => AuthResult::Error(StatusCode::INTERNAL_SERVER_ERROR),
                },
            }
        }
    }
}

impl IntoResponse for AuthResult {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Registered(redirect) => redirect.into_response(),
            Self::LoggedIn(cookie_jar, redirect) => (cookie_jar, redirect).into_response(),
            Self::Error(status_code) => status_code.into_response(),
        }
    }
}
