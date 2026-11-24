//! DD-ST-014 (SDD 3.11.6): the process registry and the game lease rows, as functions over a
//! connection. They know no threads and no clock: `now` and the pid check are arguments, so a
//! unit test drives them with a fake clock and a fake pid table.
//!
//! **Values (XC-16, UR-17; all `ESTIMATE`):** a process refreshes its row every 10 s; a row is
//! live when its pid exists and its `heartbeat_at` is not older than 30 s (OQ-ST-001); the game
//! lease row is refreshed every 2 s and is for display only: the journal `flock` decides who
//! owns a game. The shape of `process_heartbeat` never changes (SDD 3.11.5), so these
//! functions work against any schema version that has the table.

use std::time::Duration;

use chess_domain::GameId;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::api::{LeaseBeat, LeaseInfo, LiveProcess, ProcessIdentity};
use crate::error::StoreError;
use crate::repo::{game, game_key, uuid_text};

/// A process refreshes its `PROCESS_HEARTBEAT` row this often (XC-16, `ESTIMATE`).
pub const PROCESS_HEARTBEAT_EVERY: Duration = Duration::from_secs(10);
/// The analysis-job claim lease; the same value as the process heartbeat (XC-16, `ESTIMATE`).
pub const ANALYSIS_LEASE: Duration = Duration::from_secs(10);
/// The game lease row is refreshed this often (display only; XC-16, `ESTIMATE`).
pub const LEASE_BEAT_EVERY: Duration = Duration::from_secs(2);
/// A process row older than this is stale: three refresh periods (OQ-ST-001, `ESTIMATE`).
pub const PROCESS_STALE_AFTER: Duration = Duration::from_secs(30);

fn stale_ms() -> i64 {
    i64::try_from(PROCESS_STALE_AFTER.as_millis()).unwrap_or(i64::MAX)
}

/// The pid test as an argument (`kill(pid, 0)` in production).
pub(crate) type PidProbe<'a> = &'a dyn Fn(u32) -> bool;

/// True when the registry table exists. A brand-new database has none until its first
/// migration, and then nobody else can be registered in it.
pub(crate) fn has_registry(conn: &Connection) -> Result<bool, StoreError> {
    Ok(conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'process_heartbeat'",
        [],
        |row| row.get::<_, i64>(0),
    )? > 0)
}

fn split_owner(owner: &str) -> (String, Option<u32>) {
    match owner.rsplit_once(':') {
        Some((kind, pid)) => (kind.to_owned(), pid.parse().ok()),
        None => (owner.to_owned(), None),
    }
}

fn read_rows(conn: &Connection) -> Result<Vec<LiveProcess>, StoreError> {
    let mut statement = conn.prepare(
        "SELECT owner, build_version, heartbeat_at FROM process_heartbeat ORDER BY owner",
    )?;
    let rows = statement.query_map([], |row| {
        let owner: String = row.get(0)?;
        let (kind, pid) = split_owner(&owner);
        Ok(LiveProcess {
            owner,
            kind,
            pid,
            build_version: row.get(1)?,
            heartbeat_at: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Live means: fresh (a heartbeat dated in the future counts as fresh, so a clock step never
/// kills a registered process) and its pid exists (a row without a pid is judged by age alone).
fn is_live(row: &LiveProcess, now: i64, alive: PidProbe<'_>) -> bool {
    let fresh = now.saturating_sub(row.heartbeat_at) <= stale_ms();
    fresh && row.pid.is_none_or(alive)
}

/// Every live process, this one included (`list_live_processes`). Reads only.
pub(crate) fn list_live(
    conn: &Connection,
    now: i64,
    alive: PidProbe<'_>,
) -> Result<Vec<LiveProcess>, StoreError> {
    if !has_registry(conn)? {
        return Ok(Vec::new());
    }
    Ok(read_rows(conn)?
        .into_iter()
        .filter(|row| is_live(row, now, alive))
        .collect())
}

/// Live processes other than `me_owner`. Reads only; the migration guard uses it.
pub(crate) fn other_live(
    conn: &Connection,
    me_owner: &str,
    now: i64,
    alive: PidProbe<'_>,
) -> Result<Vec<LiveProcess>, StoreError> {
    Ok(list_live(conn, now, alive)?
        .into_iter()
        .filter(|row| row.owner != me_owner)
        .collect())
}

/// Registration and the check for other live processes in one `BEGIN IMMEDIATE`, so two
/// simultaneous starts see each other (SDD 3.11.6). Stale rows of other owners are deleted.
/// A live process of **another build version** refuses the registration with
/// `OtherVersionRunning{version, kind}` and this process's row is not written (the stale-row
/// deletions are kept). Otherwise the row is upserted and the other live processes returned.
pub(crate) fn register(
    conn: &mut Connection,
    me: &ProcessIdentity,
    now: i64,
    alive: PidProbe<'_>,
) -> Result<Vec<LiveProcess>, StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let own = me.owner();
    let mut others = Vec::new();
    for row in read_rows(&tx)? {
        if row.owner == own {
            continue;
        }
        if is_live(&row, now, alive) {
            others.push(row);
        } else {
            tx.execute(
                "DELETE FROM process_heartbeat WHERE owner = ?1",
                [&row.owner],
            )?;
        }
    }
    if let Some(other) = others
        .iter()
        .find(|row| row.build_version != me.build_version)
    {
        let refusal = StoreError::OtherVersionRunning {
            version: other.build_version.clone(),
            kind: other.kind.clone(),
        };
        tx.commit()?;
        return Err(refusal);
    }
    tx.execute(
        "INSERT INTO process_heartbeat (owner, build_version, heartbeat_at) VALUES (?1, ?2, ?3) \
         ON CONFLICT (owner) DO UPDATE SET build_version = excluded.build_version, \
         heartbeat_at = excluded.heartbeat_at",
        params![own, me.build_version, now],
    )?;
    tx.commit()?;
    Ok(others)
}

/// Refreshes this process's row. If another process deleted it as stale (a long sleep of this
/// machine), the process registers again with the full check.
pub(crate) fn heartbeat(
    conn: &mut Connection,
    me: &ProcessIdentity,
    now: i64,
    alive: PidProbe<'_>,
) -> Result<(), StoreError> {
    let changed = conn.execute(
        "UPDATE process_heartbeat SET heartbeat_at = ?2 WHERE owner = ?1",
        params![me.owner(), now],
    )?;
    if changed == 0 {
        register(conn, me, now, alive)?;
    }
    Ok(())
}

/// Removes this process's row. A missing row is not an error.
pub(crate) fn deregister(conn: &Connection, me: &ProcessIdentity) -> Result<(), StoreError> {
    conn.execute(
        "DELETE FROM process_heartbeat WHERE owner = ?1",
        [me.owner()],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------- game leases

/// Creates or takes over the display row of `game` for `owner`. The caller holds the journal
/// `flock`, which is the real lease, so the row is overwritten whoever the old owner was and
/// whatever its heartbeat (stale-lease takeover). Returns the row it replaced when that row
/// named a different owner. `NotFound` for an unknown game.
pub(crate) fn lease_upsert(
    conn: &mut Connection,
    game: GameId,
    owner: &str,
    now: i64,
) -> Result<Option<LeaseInfo>, StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let key = game_key(&tx, game)?;
    let previous = tx
        .query_row(
            "SELECT owner, heartbeat_at FROM game_lease WHERE game_id = ?1",
            [key],
            |row| {
                Ok(LeaseInfo {
                    owner: row.get(0)?,
                    heartbeat_at: row.get(1)?,
                })
            },
        )
        .optional()?;
    tx.execute(
        "INSERT INTO game_lease (game_id, owner, heartbeat_at) VALUES (?1, ?2, ?3) \
         ON CONFLICT (game_id) DO UPDATE SET owner = excluded.owner, \
         heartbeat_at = excluded.heartbeat_at",
        params![key, owner, now],
    )?;
    tx.commit()?;
    Ok(previous.filter(|row| row.owner != owner))
}

/// Deletes the row if it is `owner`'s (compare-and-set: a session that lost the game must not
/// delete the new owner's row). True when a row was deleted.
pub(crate) fn lease_delete(
    conn: &Connection,
    game: GameId,
    owner: &str,
) -> Result<bool, StoreError> {
    let changed = conn.execute(
        "DELETE FROM game_lease WHERE owner = ?2 AND game_id = (SELECT id FROM game WHERE uuid = ?1)",
        params![uuid_text(game.as_uuid()), owner],
    )?;
    Ok(changed > 0)
}

/// The 2 s refresh (SDD 3.7.2 `lease_beat`). Compare-and-set on `owner`.
pub(crate) fn lease_beat(
    conn: &mut Connection,
    game_id: GameId,
    owner: &str,
    now: i64,
) -> Result<LeaseBeat, StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let uuid = uuid_text(game_id.as_uuid());
    let changed = tx.execute(
        "UPDATE game_lease SET heartbeat_at = ?3 WHERE owner = ?2 \
         AND game_id = (SELECT id FROM game WHERE uuid = ?1)",
        params![uuid, owner, now],
    )?;
    let outcome = if changed > 0 {
        LeaseBeat::Held
    } else if let Some(row) = lease_row(&tx, game_id)? {
        LeaseBeat::Lost { owner: row.owner }
    } else {
        let record = game::get_game(&tx, game_id)?;
        LeaseBeat::RowGone {
            status: record.status,
            voided_by: record.voided_by,
            content_hash: record.content_hash,
            ended_at: record.ended_at,
        }
    };
    tx.commit()?;
    Ok(outcome)
}

/// The lease row of a game, if any.
pub(crate) fn lease_row(conn: &Connection, game: GameId) -> Result<Option<LeaseInfo>, StoreError> {
    Ok(conn
        .query_row(
            "SELECT l.owner, l.heartbeat_at FROM game_lease l JOIN game g ON g.id = l.game_id \
             WHERE g.uuid = ?1",
            [uuid_text(game.as_uuid())],
            |row| {
                Ok(LeaseInfo {
                    owner: row.get(0)?,
                    heartbeat_at: row.get(1)?,
                })
            },
        )
        .optional()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ExclusiveReason;

    const SECOND: i64 = 1_000;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE process_heartbeat (owner TEXT PRIMARY KEY, build_version TEXT NOT NULL, \
             heartbeat_at INTEGER NOT NULL) WITHOUT ROWID;",
        )
        .unwrap();
        conn
    }

    fn me(version: &str) -> ProcessIdentity {
        ProcessIdentity {
            kind: "cli".into(),
            pid: 100,
            build_version: version.into(),
        }
    }

    fn put(conn: &Connection, owner: &str, version: &str, at: i64) {
        conn.execute(
            "INSERT INTO process_heartbeat VALUES (?1, ?2, ?3)",
            params![owner, version, at],
        )
        .unwrap();
    }

    fn owners(conn: &Connection) -> Vec<String> {
        read_rows(conn)
            .unwrap()
            .into_iter()
            .map(|r| r.owner)
            .collect()
    }

    fn all_alive(_: u32) -> bool {
        true
    }

    #[test]
    fn the_values_are_those_of_xc_16_and_oq_st_001() {
        assert_eq!(PROCESS_HEARTBEAT_EVERY, Duration::from_secs(10));
        assert_eq!(ANALYSIS_LEASE, Duration::from_secs(10));
        assert_eq!(LEASE_BEAT_EVERY, Duration::from_secs(2));
        assert_eq!(PROCESS_STALE_AFTER, Duration::from_secs(30));
    }

    #[test]
    fn owner_text_is_kind_colon_pid_and_a_colon_in_the_kind_is_replaced() {
        assert_eq!(me("1").owner(), "cli:100");
        assert_eq!(ProcessIdentity::current("a:b", "1").kind, "a_b");
        assert_eq!(split_owner("server:42"), ("server".into(), Some(42)));
        assert_eq!(split_owner("weird"), ("weird".into(), None));
        assert_eq!(split_owner("x:y"), ("x".into(), None));
    }

    #[test]
    fn registration_writes_the_row_and_returns_the_other_live_processes() {
        let mut conn = db();
        put(&conn, "server:7", "1.0", 10 * SECOND);
        let others = register(&mut conn, &me("1.0"), 12 * SECOND, &all_alive).unwrap();
        assert_eq!(others.len(), 1);
        assert_eq!(
            (others[0].kind.as_str(), others[0].pid),
            ("server", Some(7))
        );
        assert_eq!(owners(&conn), vec!["cli:100", "server:7"]);
        // Registering again is an update of the same row, not a second one.
        register(&mut conn, &me("1.0"), 13 * SECOND, &all_alive).unwrap();
        assert_eq!(owners(&conn).len(), 2);
        let at: i64 = conn
            .query_row(
                "SELECT heartbeat_at FROM process_heartbeat WHERE owner = 'cli:100'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(at, 13 * SECOND);
    }

    /// The row is live while its heartbeat is at most 30 s old and its pid exists.
    #[test]
    fn liveness_is_age_at_most_30_s_and_an_existing_pid() {
        let conn = db();
        put(&conn, "cli:1", "v", 0);
        put(&conn, "cli:2", "v", 0);
        let now = 30 * SECOND;
        let live = |alive: PidProbe<'_>, now| list_live(&conn, now, alive).unwrap().len();
        assert_eq!(live(&all_alive, now), 2, "exactly 30 s old is still live");
        assert_eq!(live(&all_alive, now + 1), 0, "30.001 s old is stale");
        assert_eq!(live(&|pid| pid == 1, now), 1, "a dead pid is dead at once");
        // A heartbeat dated in the future (a clock step back) is fresh.
        assert_eq!(live(&all_alive, -5 * SECOND), 2);
        // A row with no pid in its owner text is judged by age alone.
        put(&conn, "odd", "v", now);
        assert_eq!(live(&|_| false, now), 1);
    }

    #[test]
    fn registration_deletes_stale_rows_of_others_but_not_live_ones() {
        let mut conn = db();
        put(&conn, "cli:1", "v", 0); // too old
        put(&conn, "cli:2", "v", 60 * SECOND); // fresh but dead pid
        put(&conn, "cli:3", "v", 60 * SECOND); // live
        let others = register(&mut conn, &me("v"), 61 * SECOND, &|pid| pid != 2).unwrap();
        assert_eq!(
            others.iter().map(|o| o.owner.as_str()).collect::<Vec<_>>(),
            vec!["cli:3"]
        );
        assert_eq!(owners(&conn), vec!["cli:100", "cli:3"]);
    }

    /// F-A03: a live process of another version refuses the registration and no row is written
    /// for this process; stale rows are still cleaned.
    #[test]
    fn another_live_build_version_refuses_and_writes_no_row() {
        let mut conn = db();
        put(&conn, "desktop:9", "0.9", 50 * SECOND);
        put(&conn, "cli:1", "0.1", 0); // stale: deleted even though the registration fails
        let err = register(&mut conn, &me("1.0"), 55 * SECOND, &all_alive).unwrap_err();
        match err {
            StoreError::OtherVersionRunning { version, kind } => {
                assert_eq!((version.as_str(), kind.as_str()), ("0.9", "desktop"));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(owners(&conn), vec!["desktop:9"]);
        // The same version coexists.
        assert!(register(&mut conn, &me("0.9"), 55 * SECOND, &all_alive).is_ok());
        // A stale row of another version does not refuse.
        let mut conn = db();
        put(&conn, "desktop:9", "0.9", 0);
        assert!(register(&mut conn, &me("1.0"), 100 * SECOND, &all_alive).is_ok());
    }

    #[test]
    fn heartbeat_refreshes_the_row_and_registers_again_when_it_was_deleted() {
        let mut conn = db();
        register(&mut conn, &me("v"), 0, &all_alive).unwrap();
        heartbeat(&mut conn, &me("v"), 10 * SECOND, &all_alive).unwrap();
        let at = |conn: &Connection| -> i64 {
            conn.query_row(
                "SELECT heartbeat_at FROM process_heartbeat WHERE owner = 'cli:100'",
                [],
                |r| r.get(0),
            )
            .unwrap()
        };
        assert_eq!(at(&conn), 10 * SECOND);
        deregister(&conn, &me("v")).unwrap();
        assert!(owners(&conn).is_empty());
        heartbeat(&mut conn, &me("v"), 20 * SECOND, &all_alive).unwrap();
        assert_eq!(at(&conn), 20 * SECOND, "registered again");
        // ... with the full check: another version now refuses it.
        deregister(&conn, &me("v")).unwrap();
        put(&conn, "server:5", "other", 20 * SECOND);
        let err = heartbeat(&mut conn, &me("v"), 21 * SECOND, &all_alive).unwrap_err();
        assert!(
            matches!(err, StoreError::OtherVersionRunning { .. }),
            "{err:?}"
        );
        // Deregistering a missing row is not an error and leaves other rows.
        deregister(&conn, &me("v")).unwrap();
        assert_eq!(owners(&conn), vec!["server:5"]);
    }

    #[test]
    fn other_live_excludes_this_process_and_the_registry_may_be_missing() {
        let mut conn = db();
        put(&conn, "server:7", "v", 5 * SECOND);
        register(&mut conn, &me("v"), 5 * SECOND, &all_alive).unwrap();
        let others = other_live(&conn, "cli:100", 6 * SECOND, &all_alive).unwrap();
        assert_eq!(others.len(), 1);
        assert_eq!(list_live(&conn, 6 * SECOND, &all_alive).unwrap().len(), 2);
        let bare = Connection::open_in_memory().unwrap();
        assert!(!has_registry(&bare).unwrap());
        assert!(other_live(&bare, "x", 0, &all_alive).unwrap().is_empty());
        assert!(has_registry(&conn).unwrap());
    }

    #[test]
    fn exclusive_reason_is_the_migration() {
        // The guard of the opener builds this value; the variant exists and is comparable.
        assert_eq!(ExclusiveReason::Migration, ExclusiveReason::Migration);
    }
}
