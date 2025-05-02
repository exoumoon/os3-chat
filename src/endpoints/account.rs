use crate::auth::{SESSION_COOKIE_NAME, Session};
use crate::repository::account::{LoginError, RegistrationError};
use crate::state::SharedState;
use askama::Template;
use axum::response::{Html, IntoResponse, Redirect};
use axum::{Form, debug_handler, extract::State, http::StatusCode};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_valid::Valid;
use serde::Deserialize;
use tracing::instrument;
use validator::Validate;

#[derive(Template)]
#[template(path = "account.html")]
pub struct AccountTemplate;

#[instrument(skip_all)]
#[debug_handler]
pub async fn page() -> Result<impl IntoResponse, StatusCode> {
    AccountTemplate
        .render()
        .map(Html)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize, Validate, Debug)]
#[must_use]
pub struct CredentialsForm {
    #[validate(length(min = 1, max = 64))]
    username: String,
    #[validate(length(min = 8, max = 64))]
    password: String,
    action: SubmitAction,
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
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
    if credentials.action == SubmitAction::Register {
        match state
            .repository
            .accounts
            .register(&credentials.username, &credentials.password)
            .await
        {
            Ok(_account) => { /* continue to automatic login */ }
            Err(RegistrationError::NameTaken) => return AuthResult::Error(StatusCode::CONFLICT),
            Err(_internal) => return AuthResult::Error(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    match state
        .repository
        .accounts
        .login(&credentials.username, &credentials.password)
        .await
    {
        Ok(session) => {
            let base_cookie = Cookie::new(SESSION_COOKIE_NAME, session.token);
            let cookie = Cookie::build(base_cookie)
                .path("/")
                .http_only(true)
                .secure(false)
                .same_site(SameSite::Lax);
            let jar = CookieJar::new().add(cookie);
            AuthResult::LoggedIn(jar, Redirect::to("/chat/1"))
        }

        Err(LoginError::InvalidCredentials) => AuthResult::Error(StatusCode::UNAUTHORIZED),
        Err(_) => AuthResult::Error(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[instrument(skip_all, fields(username = account.username))]
#[debug_handler]
pub async fn logout(
    State(state): State<SharedState>,
    Session(account): Session,
) -> Result<Redirect, StatusCode> {
    state
        .repository
        .accounts
        .expire_session(account.session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Redirect::to("/"))
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
