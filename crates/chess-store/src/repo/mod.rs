//! DD-ST-008 to DD-ST-012 (part A: players, ratings read, games): repository functions over
//! `&Connection` / `&mut Connection`. They know nothing of threads, tasks or channels, so the
//! same code runs on the store thread, on a read connection and in tests (SDD 3.6, 3.8). All SQL
//! of the crate's normal operation lives here; integer row keys never leave this module (D-40).

pub(crate) mod analysis;
pub(crate) mod apply;
pub(crate) mod coord;
pub(crate) mod finalize;
pub(crate) mod game;
pub(crate) mod invariant;
pub(crate) mod jobs;
pub(crate) mod lifecycle;
pub(crate) mod player;
pub(crate) mod rating;
pub(crate) mod stats;

use chess_domain::{GameId, PlayerId};
use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::error::{Entity, StoreError};

pub(crate) fn uuid_text(id: Uuid) -> String {
    // Hyphenated, lower case, 36 characters: what the `uuid` CHECK demands (SF-12).
    id.hyphenated().to_string()
}

pub(crate) fn parse_uuid(text: &str) -> Result<Uuid, StoreError> {
    Uuid::try_parse(text).map_err(|_| StoreError::Corrupt)
}

pub(crate) fn player_key(conn: &Connection, id: PlayerId) -> Result<i64, StoreError> {
    conn.query_row(
        "SELECT id FROM player WHERE uuid = ?1",
        [uuid_text(id.as_uuid())],
        |row| row.get(0),
    )
    .optional()?
    .ok_or(StoreError::NotFound {
        entity: Entity::Player,
        id: id.as_uuid(),
    })
}

pub(crate) fn game_key(conn: &Connection, id: GameId) -> Result<i64, StoreError> {
    conn.query_row(
        "SELECT id FROM game WHERE uuid = ?1",
        [uuid_text(id.as_uuid())],
        |row| row.get(0),
    )
    .optional()?
    .ok_or(StoreError::NotFound {
        entity: Entity::Game,
        id: id.as_uuid(),
    })
}

/// A value from the caller that does not fit SQLite's signed 64-bit integer is a caller bug.
pub(crate) fn to_i64(value: u64) -> Result<i64, StoreError> {
    i64::try_from(value).map_err(|_| StoreError::InvariantViolated)
}

/// A stored value outside the type's range means the file was not written by this program.
pub(crate) fn from_i64_u64(value: i64) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::Corrupt)
}

pub(crate) fn from_i64_u32(value: i64) -> Result<u32, StoreError> {
    u32::try_from(value).map_err(|_| StoreError::Corrupt)
}

pub(crate) fn from_i64_i32(value: i64) -> Result<i32, StoreError> {
    i32::try_from(value).map_err(|_| StoreError::Corrupt)
}

pub(crate) fn opt_u64(value: Option<i64>) -> Result<Option<u64>, StoreError> {
    value.map(from_i64_u64).transpose()
}

pub(crate) fn opt_i32(value: Option<i64>) -> Result<Option<i32>, StoreError> {
    value.map(from_i64_i32).transpose()
}

/// Parses a stored text enum; text that no variant accepts is `Corrupt`.
pub(crate) fn parse_text<T: std::str::FromStr>(text: &str) -> Result<T, StoreError> {
    text.parse().map_err(|_| StoreError::Corrupt)
}
