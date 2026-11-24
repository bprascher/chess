//! DD-ST-008 `PlayerRepo`: players, computer profiles, preferences, app settings and
//! `roster_indexes` (architecture 7.4 rows Register, Rename/archive/preferences, Roster batch).
//! The store re-checks no policy (`PlayerService` owns it) except the uniqueness backstops and
//! the archive guard.

use chess_domain::{Category, Personality, PlayerId, PlayerKind, RosterSeed, Track, WallMs};
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};

use crate::api::{ComputerProfile, NewComputer, NewPlayer, PlayerRecord};
use crate::error::{Entity, StoreError};
use crate::repo::{from_i64_i32, from_i64_u32, parse_text, parse_uuid, player_key, uuid_text};

/// Rating rows written for a new Player carry no game and no K (it is not a game result):
/// `k` is 0. SDD silent on the value: see evidence finding F-A04.
const INITIAL_POINT_K: i64 = 0;

const PLAYER_SELECT: &str = "SELECT p.uuid, p.kind, p.name, p.start_category, p.strict_elo, \
    p.training_elo, p.strict_rated_games, p.training_rated_games, p.reset_at, p.archived, \
    p.created_at, c.roster_seed, c.roster_index, c.skill_level, c.depth_limit, c.movetime_ms, \
    c.uci_elo, c.error_permille, c.error_multipv \
    FROM player p LEFT JOIN computer_profile c ON c.player_id = p.id";

fn record_from_row(row: &Row<'_>) -> Result<PlayerRecord, StoreError> {
    let uuid: String = row.get(0)?;
    let kind: PlayerKind = parse_text(&row.get::<_, String>(1)?)?;
    let start_category: Category = parse_text(&row.get::<_, String>(3)?)?;
    let computer = match row.get::<_, Option<i64>>(11)? {
        None => None,
        Some(seed) => {
            let personality = Personality::new(
                u8::try_from(row.get::<_, i64>(13)?).map_err(|_| StoreError::Corrupt)?,
                from_i64_u32(row.get(14)?)?,
                from_i64_u32(row.get(15)?)?,
                row.get::<_, Option<i64>>(16)?
                    .map(from_i64_u32)
                    .transpose()?,
                u16::try_from(row.get::<_, i64>(17)?).map_err(|_| StoreError::Corrupt)?,
                u8::try_from(row.get::<_, i64>(18)?).map_err(|_| StoreError::Corrupt)?,
            )
            .map_err(|_| StoreError::Corrupt)?;
            Some(ComputerProfile {
                roster_seed: RosterSeed::new(from_i64_u32(seed)?),
                roster_index: from_i64_u32(row.get(12)?)?,
                personality,
            })
        }
    };
    Ok(PlayerRecord {
        id: PlayerId::from_uuid(parse_uuid(&uuid)?),
        kind,
        name: row.get(2)?,
        start_category,
        strict_elo: from_i64_i32(row.get(4)?)?,
        training_elo: from_i64_i32(row.get(5)?)?,
        strict_rated_games: from_i64_u32(row.get(6)?)?,
        training_rated_games: from_i64_u32(row.get(7)?)?,
        reset_at: row.get::<_, Option<i64>>(8)?.map(WallMs),
        archived: row.get::<_, i64>(9)? != 0,
        created_at: WallMs(row.get(10)?),
        computer,
    })
}

pub(crate) fn get_player(conn: &Connection, id: PlayerId) -> Result<PlayerRecord, StoreError> {
    find_by_uuid(conn, id)?.ok_or(StoreError::NotFound {
        entity: Entity::Player,
        id: id.as_uuid(),
    })
}

fn find_by_uuid(conn: &Connection, id: PlayerId) -> Result<Option<PlayerRecord>, StoreError> {
    let sql = format!("{PLAYER_SELECT} WHERE p.uuid = ?1");
    conn.query_row(&sql, [uuid_text(id.as_uuid())], |row| {
        Ok(record_from_row(row))
    })
    .optional()?
    .transpose()
}

pub(crate) fn find_player_by_name(
    conn: &Connection,
    name: &str,
) -> Result<Option<PlayerRecord>, StoreError> {
    // `player.name` is declared COLLATE BINARY, so `=` is case-sensitive (PLY-6).
    let sql = format!("{PLAYER_SELECT} WHERE p.name = ?1");
    conn.query_row(&sql, [name], |row| Ok(record_from_row(row)))
        .optional()?
        .transpose()
}

pub(crate) fn list_players(
    conn: &Connection,
    kind: Option<PlayerKind>,
    include_archived: bool,
) -> Result<Vec<PlayerRecord>, StoreError> {
    let sql = format!(
        "{PLAYER_SELECT} WHERE (?1 IS NULL OR p.kind = ?1) AND (?2 = 1 OR p.archived = 0) \
         ORDER BY p.id"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(
        params![kind.map(PlayerKind::as_str), i64::from(include_archived)],
        |row| Ok(record_from_row(row)),
    )?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row??);
    }
    Ok(out)
}

fn insert_rating_points(
    conn: &Connection,
    key: i64,
    strict: i32,
    training: i32,
    now: i64,
) -> Result<(), StoreError> {
    for (track, elo) in [(Track::Strict, strict), (Track::Training, training)] {
        conn.execute(
            "INSERT INTO rating_history (player_id, track, game_id, rating_before, rating_after, k, at) \
             VALUES (?1, ?2, NULL, ?3, ?3, ?4, ?5)",
            params![key, track.as_str(), elo, INITIAL_POINT_K, now],
        )?;
    }
    Ok(())
}

fn name_in_use(conn: &Connection, name: &str, except: Option<i64>) -> Result<bool, StoreError> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM player WHERE name = ?1 AND (?2 IS NULL OR id <> ?2)",
            params![name, except],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

/// `PLAYER` + `PREFERENCES` + one rating row per track, in one `BEGIN IMMEDIATE`.
pub(crate) fn register_player(
    conn: &mut Connection,
    new: &NewPlayer,
    now: i64,
) -> Result<PlayerRecord, StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if name_in_use(&tx, &new.name, None)? {
        return Err(StoreError::NameTaken);
    }
    tx.execute(
        "INSERT INTO player (uuid, kind, name, start_category, strict_elo, training_elo, created_at) \
         VALUES (?1, 'human', ?2, ?3, ?4, ?5, ?6)",
        params![
            uuid_text(new.id.as_uuid()),
            new.name,
            new.start_category.as_str(),
            new.strict_elo,
            new.training_elo,
            now
        ],
    )?;
    let key = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO preferences (player_id, json) VALUES (?1, ?2)",
        params![key, new.preferences_json],
    )?;
    insert_rating_points(&tx, key, new.strict_elo, new.training_elo, now)?;
    let record = get_player(&tx, new.id)?;
    tx.commit()?;
    Ok(record)
}

/// The roster batch: N Players, N profiles and 2N rating rows, all or none. A repeated
/// `(roster_seed, roster_index)` is `RosterIndexTaken`; the caller repeats the read of
/// `roster_indexes` and the insert once (TR-9, UR-49). A repeated parameter set (the ENG-2
/// backstop index) is `InvariantViolated`.
pub(crate) fn insert_computers(
    conn: &mut Connection,
    batch: &[NewComputer],
    now: i64,
) -> Result<Vec<PlayerRecord>, StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let mut out = Vec::with_capacity(batch.len());
    for new in batch {
        let taken = tx
            .query_row(
                "SELECT 1 FROM computer_profile WHERE roster_seed = ?1 AND roster_index = ?2",
                params![new.roster_seed.get(), new.roster_index],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if taken {
            return Err(StoreError::RosterIndexTaken);
        }
        if name_in_use(&tx, &new.name, None)? {
            return Err(StoreError::NameTaken);
        }
        tx.execute(
            "INSERT INTO player (uuid, kind, name, start_category, strict_elo, training_elo, created_at) \
             VALUES (?1, 'computer', ?2, ?3, ?4, ?5, ?6)",
            params![
                uuid_text(new.id.as_uuid()),
                new.name,
                new.start_category.as_str(),
                new.strict_elo,
                new.training_elo,
                now
            ],
        )?;
        let key = tx.last_insert_rowid();
        let p = &new.personality;
        tx.execute(
            "INSERT INTO computer_profile (player_id, roster_seed, roster_index, skill_level, \
             depth_limit, movetime_ms, uci_elo, error_permille, error_multipv) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                key,
                new.roster_seed.get(),
                new.roster_index,
                p.skill_level,
                p.depth_limit,
                p.movetime_ms,
                p.uci_elo,
                p.error_permille,
                p.error_multipv
            ],
        )?;
        insert_rating_points(&tx, key, new.strict_elo, new.training_elo, now)?;
        out.push(get_player(&tx, new.id)?);
    }
    tx.commit()?;
    Ok(out)
}

/// Every stored `roster_index`, ascending, duplicates kept (two roster seeds may share an
/// index). The store knows nothing of the `5*k + class` scheme and has no class column (D-46).
pub(crate) fn roster_indexes(conn: &Connection) -> Result<Vec<u32>, StoreError> {
    let mut statement = conn
        .prepare("SELECT roster_index FROM computer_profile ORDER BY roster_index, roster_seed")?;
    let rows = statement.query_map([], |row| row.get::<_, i64>(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(from_i64_u32(row?)?);
    }
    Ok(out)
}

pub(crate) fn rename_player(
    conn: &mut Connection,
    id: PlayerId,
    new_name: &str,
) -> Result<PlayerRecord, StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let key = player_key(&tx, id)?;
    if name_in_use(&tx, new_name, Some(key))? {
        return Err(StoreError::NameTaken);
    }
    tx.execute(
        "UPDATE player SET name = ?1 WHERE id = ?2",
        params![new_name, key],
    )?;
    let record = get_player(&tx, id)?;
    tx.commit()?;
    Ok(record)
}

/// Archives a Player unless an `in_progress` game names it (the guard of SDD 3.7.2, checked in
/// the same transaction). Idempotent for an already archived Player.
pub(crate) fn archive_player(
    conn: &mut Connection,
    id: PlayerId,
) -> Result<PlayerRecord, StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let key = player_key(&tx, id)?;
    let ids = crate::repo::lifecycle::unfinished_game_ids(&tx, key)?;
    if !ids.is_empty() {
        return Err(StoreError::HasUnfinishedGames { ids });
    }
    tx.execute("UPDATE player SET archived = 1 WHERE id = ?1", [key])?;
    let record = get_player(&tx, id)?;
    tx.commit()?;
    Ok(record)
}

/// Replaces the preferences blob; `json` must be valid JSON (the table CHECK).
pub(crate) fn update_preferences(
    conn: &mut Connection,
    id: PlayerId,
    json: &str,
) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let key = player_key(&tx, id)?;
    tx.execute(
        "INSERT INTO preferences (player_id, json) VALUES (?1, ?2) \
         ON CONFLICT(player_id) DO UPDATE SET json = excluded.json",
        params![key, json],
    )?;
    tx.commit()?;
    Ok(())
}

/// `None` for an existing Player without a row (a computer).
pub(crate) fn get_preferences(
    conn: &Connection,
    id: PlayerId,
) -> Result<Option<String>, StoreError> {
    let key = player_key(conn, id)?;
    Ok(conn
        .query_row(
            "SELECT json FROM preferences WHERE player_id = ?1",
            [key],
            |row| row.get(0),
        )
        .optional()?)
}

pub(crate) fn get_app_setting(conn: &Connection, key: &str) -> Result<Option<String>, StoreError> {
    Ok(conn
        .query_row(
            "SELECT value FROM app_setting WHERE setting_key = ?1",
            [key],
            |row| row.get(0),
        )
        .optional()?)
}

pub(crate) fn set_app_setting(
    conn: &mut Connection,
    key: &str,
    value: &str,
) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute(
        "INSERT INTO app_setting (setting_key, value) VALUES (?1, ?2) \
         ON CONFLICT(setting_key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    tx.commit()?;
    Ok(())
}
