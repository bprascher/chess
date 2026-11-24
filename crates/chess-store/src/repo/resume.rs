//! Part D4: what the resume and the startup recovery read from the database about one game, in
//! one place (SDD 3.7.3 `open_for_resume`, 3.11.3). Repository functions over `&Connection`:
//! no thread, no file. The `MOVE` rows come back in the shape of the reconcile function
//! (`StoredMove`, one-based `ply`), and the seat kinds come back so that a `Start` record can be
//! synthesized from the row (3.9.2, RR-A-1 (a)).

use chess_domain::{GameId, PlayerId, PlayerKind};
use rusqlite::{Connection, OptionalExtension};

use crate::api::GameRecord;
use crate::error::StoreError;
use crate::journal::StoredMove;
use crate::repo::{game, parse_text, uuid_text};

/// A `GAME` row, its `MOVE` rows and the two seats' kinds.
#[derive(Debug, Clone)]
pub(crate) struct ResumeRow {
    pub record: GameRecord,
    pub moves: Vec<StoredMove>,
    pub white_kind: Option<PlayerKind>,
    pub black_kind: Option<PlayerKind>,
}

fn kind_of(conn: &Connection, player: Option<PlayerId>) -> Result<Option<PlayerKind>, StoreError> {
    let Some(player) = player else {
        return Ok(None);
    };
    let text: Option<String> = conn
        .query_row(
            "SELECT kind FROM player WHERE uuid = ?1",
            [uuid_text(player.as_uuid())],
            |row| row.get(0),
        )
        .optional()?;
    text.map(|t| parse_text::<PlayerKind>(&t)).transpose()
}

/// `Ok(None)` when the game has no row.
pub(crate) fn load(conn: &Connection, id: GameId) -> Result<Option<ResumeRow>, StoreError> {
    let record = match game::get_game(conn, id) {
        Ok(record) => record,
        Err(StoreError::NotFound { .. }) => return Ok(None),
        Err(other) => return Err(other),
    };
    let moves = game::game_moves(conn, id)?
        .into_iter()
        .map(|row| StoredMove {
            ply: row.ply,
            uci: row.uci,
            san: row.san,
            fen_after: row.fen_after,
            think_ms: row.think_ms,
            clocks: row.clocks_ms,
            played_at: row.played_at.map(|at| at.0),
        })
        .collect();
    let white_kind = kind_of(conn, record.white)?;
    let black_kind = kind_of(conn, record.black)?;
    Ok(Some(ResumeRow {
        record,
        moves,
        white_kind,
        black_kind,
    }))
}
