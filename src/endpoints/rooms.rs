use axum::extract::State;
use axum::http::StatusCode;
use axum::{Json, debug_handler};
use serde::Serialize;
use tracing::instrument;

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
pub async fn handle_room_request(
    State(state): State<SharedState>,
    Session(requester): Session,
) -> Result<RoomResponse, StatusCode> {
    let rooms = state
        .repository
        .rooms
        .find_by_member(&requester.username)
        .await
        .inspect(|rooms| tracing::debug!(?rooms, "Returning list of rooms via API"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|db_room| RoomResponseEntry {
            room_id: db_room.id,
            room_name: db_room.name,
        })
        .collect();

    Ok(Json(rooms))
}
