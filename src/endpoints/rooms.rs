use axum::extract::State;
use axum::http::StatusCode;
use axum::{Form, Json, debug_handler};
use axum_valid::Valid;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use validator::Validate;

use crate::auth::Session;
use crate::state::SharedState;

#[derive(Serialize, Debug)]
#[must_use]
pub struct RoomResponseEntry {
    pub room_id: i64,
    pub room_name: String,
}

pub type RoomResponse = Json<Vec<RoomResponseEntry>>;

#[instrument(skip_all, fields(requester.username = requester.username), err(Debug))]
#[debug_handler]
pub async fn list(
    State(state): State<SharedState>,
    Session(requester): Session,
) -> Result<RoomResponse, StatusCode> {
    let rooms = state
        .repository
        .rooms
        .find_by_member(&requester.username)
        .await
        .inspect(|rooms| tracing::debug!(count = rooms.len(), "Returning list of rooms"))
        .inspect_err(|error| tracing::error!(?error, "Failed to get user's rooms"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|db_room| RoomResponseEntry {
            room_id: db_room.id,
            room_name: db_room.name,
        })
        .collect();

    Ok(Json(rooms))
}

#[derive(Deserialize, Validate, Debug)]
#[must_use]
pub struct CreateRoomForm {
    #[validate(length(min = 1, max = 64))]
    room_name: String,
}

#[instrument(skip_all, fields(requester.username = requester.username, form = ?form))]
#[debug_handler]
pub async fn create(
    State(state): State<SharedState>,
    Session(requester): Session,
    Valid(form): Valid<Form<CreateRoomForm>>,
) -> Result<StatusCode, StatusCode> {
    let room = state
        .repository
        .rooms
        .create(&form.room_name)
        .await
        .inspect(|room| tracing::debug!(?room, "Created new room"))
        .inspect_err(|error| tracing::error!(?error, "Failed to create room"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    room.add_member(&state.db_pool, &requester.username)
        .await
        .inspect(|()| tracing::debug!("Added member to room"))
        .inspect_err(|error| tracing::error!(?error, "Failed to add member to room"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::CREATED)
}

#[derive(Deserialize, Validate, Debug)]
#[must_use]
pub struct MemberModificationForm {
    #[validate(length(min = 1, max = 64))]
    username: String,
    room_id: i64,
}

#[instrument(skip_all, fields(requester.username = requester.username, form = ?form))]
#[debug_handler]
pub async fn invite(
    State(state): State<SharedState>,
    Session(requester): Session,
    Valid(form): Valid<Form<MemberModificationForm>>,
) -> Result<StatusCode, StatusCode> {
    let room = state
        .repository
        .rooms
        .find_by_id(form.room_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    room.add_member(&state.db_pool, &form.username)
        .await
        .inspect(|()| tracing::debug!("Added member to room"))
        .inspect_err(|error| tracing::error!(?error, "Failed to add member to room"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::CREATED)
}

#[instrument(skip_all, fields(requester.username = requester.username, form = ?form))]
#[debug_handler]
pub async fn kick_out(
    State(state): State<SharedState>,
    Session(requester): Session,
    Valid(form): Valid<Form<MemberModificationForm>>,
) -> Result<StatusCode, StatusCode> {
    let room = state
        .repository
        .rooms
        .find_by_id(form.room_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    room.remove_member(&state.db_pool, &form.username)
        .await
        .inspect(|()| tracing::debug!("Deleted member from room"))
        .inspect_err(|error| tracing::error!(?error, "Failed to delete user from room"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}
