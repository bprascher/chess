//! DD-ST-018 `Reconciler`: the pure core of the journal-wins rule (SDD 3.11.3, R-15, D-66).
//! No files, no store, no clock: a function of the folded journal and the stored rows.
//!
//! `fold` turns the intact records into the moves in force: `Move{ply}` sets ply `ply + 1`
//! (`MOVE.ply` is one-based, D-92) and drops every later ply, `Takeback{to_ply}` drops the plies
//! above `to_ply`; draw, pause, resume and suspend records change nothing. `reconcile` compares
//! them with the stored `MOVE` rows and returns a `Plan`.
//!
//! | case | rule |
//! |---|---|
//! | no damage and `db_seq <= last_intact_seq` | the journal wins from the first ply where the two differ; stored rows beyond the journal's end go |
//! | damaged (middle, or `db_seq > last_intact_seq`, D-66), the two agree on the common plies | the longer run stays; a longer store is kept (`source = Store`) |
//! | damaged, the two differ at ply `d` | the journal wins from `d` |
//!
//! **Choices (flagged for the reviewer):** (1) a stored row `matches` a journal move when `uci`,
//! `san` and `fen_after` are equal and every stored value that is not NULL (`think_ms`, the
//! clock pair, `played_at`) is equal; a NULL stored value matches anything, so a row that the
//! rebase builder synthesized from a `MOVE` row with NULL columns does not make the plan
//! non-empty (the SDD asks for an empty plan after a rebase, UT-ST-088). (2) The function is
//! total for gappy maps too: whatever the plies, applying the plan and reconciling again gives
//! an empty plan (property test UT-ST-012).
//! "Nothing readable in either" (SDD last row) is not decided here: it is a scan error plus an
//! empty store, and the caller offers `VoidUnreadableGame`.

use std::collections::BTreeMap;

use crate::journal::format::{JournalRecord, Payload};

/// Both clocks, or `None` for an untimed game.
pub type ClockPair = Option<(u64, u64)>;

/// A `MOVE` row, or a journal move in the same shape. `ply` is one-based.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredMove {
    pub ply: u32,
    pub uci: String,
    pub san: String,
    pub fen_after: String,
    pub think_ms: Option<u64>,
    pub clocks: ClockPair,
    pub played_at: Option<i64>,
}

/// The intact journal folded to the moves in force and the last clocks.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Folded {
    pub moves: BTreeMap<u32, StoredMove>,
    /// The clocks of the last `Move` or `Clock` record; `None` when there is none.
    pub clocks: Option<ClockPair>,
}

fn pair(white: Option<u64>, black: Option<u64>) -> ClockPair {
    white.zip(black)
}

/// Folds intact records (SDD 3.11.3 "Input").
pub fn fold(records: &[JournalRecord]) -> Folded {
    let mut folded = Folded::default();
    for record in records {
        match &record.payload {
            Payload::Move(m) => {
                let ply = m.ply.saturating_add(1);
                let _ = folded.moves.split_off(&ply.saturating_add(1));
                let clocks = pair(m.white_ms, m.black_ms);
                folded.moves.insert(
                    ply,
                    StoredMove {
                        ply,
                        uci: m.uci.clone(),
                        san: m.san.clone(),
                        fen_after: m.fen_after.clone(),
                        think_ms: Some(m.think_ms),
                        clocks,
                        played_at: Some(record.wall_ms),
                    },
                );
                folded.clocks = Some(clocks);
            }
            Payload::Takeback(t) => {
                let _ = folded.moves.split_off(&t.to_ply.saturating_add(1));
            }
            Payload::Clock(c) => folded.clocks = Some(pair(c.white_ms, c.black_ms)),
            _ => {}
        }
    }
    folded
}

/// Which run resumes the game (`RecoveryReport.source`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunSource {
    Journal,
    Store,
    Both,
}

pub struct ReconcileInput<'a> {
    pub journal: &'a Folded,
    /// The stored `MOVE` rows.
    pub stored: &'a [StoredMove],
    /// The scan found damage: a damaged middle or an undecodable CRC-valid tail
    /// (`scan.damaged.is_some()`; CH-5 B).
    pub scan_damaged: bool,
    /// `GAME.journal_seq`.
    pub db_seq: u64,
    pub last_intact_seq: u64,
    /// The `GAME` clock columns.
    pub stored_clocks: ClockPair,
}

/// What to do to the stored rows and clocks. Applying order: delete every stored row with
/// `ply >= delete_from()`, then upsert `rows_to_write`, then set the clocks if `clocks` is
/// `Some`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// First ply replaced: stored rows from here on are deleted and rewritten.
    pub replace_from: Option<u32>,
    /// Stored rows beyond the journal's end that go (the journal is shorter).
    pub rows_to_delete_from: Option<u32>,
    pub rows_to_write: Vec<StoredMove>,
    /// `Some(pair)` sets the `GAME` clocks; `None` leaves them.
    pub clocks: Option<ClockPair>,
    pub source: RunSource,
    /// Damage in the sense of the rule: a damaged middle or `db_seq > last_intact_seq`.
    pub damaged: bool,
}

impl Plan {
    /// No row and no clock changes.
    pub fn is_empty(&self) -> bool {
        self.replace_from.is_none()
            && self.rows_to_delete_from.is_none()
            && self.rows_to_write.is_empty()
            && self.clocks.is_none()
    }

    /// The ply from which stored rows are deleted before the writes, if any.
    pub fn delete_from(&self) -> Option<u32> {
        match (self.replace_from, self.rows_to_delete_from) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }
}

fn matches(stored: &StoredMove, journal: &StoredMove) -> bool {
    stored.uci == journal.uci
        && stored.san == journal.san
        && stored.fen_after == journal.fen_after
        && stored.think_ms.is_none_or(|v| Some(v) == journal.think_ms)
        && stored.clocks.is_none_or(|v| Some(v) == journal.clocks)
        && stored
            .played_at
            .is_none_or(|v| Some(v) == journal.played_at)
}

/// The reconcile function (DD-ST-018): total, deterministic and idempotent.
pub fn reconcile(input: &ReconcileInput<'_>) -> Plan {
    let j = &input.journal.moves;
    let s: BTreeMap<u32, &StoredMove> = input.stored.iter().map(|m| (m.ply, m)).collect();
    let damaged = input.scan_damaged || input.db_seq > input.last_intact_seq;
    let jmax = j.keys().next_back().copied().unwrap_or(0);
    let smax = s.keys().next_back().copied().unwrap_or(0);
    let store_beyond = smax > jmax;
    let in_sync = |ply: u32| match (s.get(&ply), j.get(&ply)) {
        (Some(stored), Some(journal)) => matches(stored, journal),
        _ => false,
    };

    let mut plan = Plan {
        replace_from: None,
        rows_to_delete_from: None,
        rows_to_write: Vec::new(),
        clocks: None,
        source: RunSource::Journal,
        damaged,
    };
    if damaged {
        let mismatch = j
            .iter()
            .find(|(ply, journal)| s.get(ply).is_some_and(|stored| !matches(stored, journal)))
            .map(|(ply, _)| *ply);
        match mismatch {
            Some(d) => {
                plan.replace_from = Some(d);
                plan.rows_to_delete_from = store_beyond.then(|| jmax.saturating_add(1));
                plan.rows_to_write = j
                    .values()
                    .filter(|m| m.ply >= d || !s.contains_key(&m.ply))
                    .cloned()
                    .collect();
            }
            None => {
                plan.rows_to_write = j
                    .values()
                    .filter(|m| !s.contains_key(&m.ply))
                    .cloned()
                    .collect();
                plan.source = match jmax.cmp(&smax) {
                    std::cmp::Ordering::Less => RunSource::Store,
                    std::cmp::Ordering::Greater => RunSource::Journal,
                    std::cmp::Ordering::Equal => RunSource::Both,
                };
            }
        }
    } else {
        let first_difference = j
            .keys()
            .chain(s.keys())
            .copied()
            .collect::<std::collections::BTreeSet<u32>>()
            .into_iter()
            .find(|ply| !in_sync(*ply));
        if let Some(d) = first_difference {
            plan.replace_from = Some(d);
            plan.rows_to_delete_from = store_beyond.then(|| jmax.saturating_add(1));
            plan.rows_to_write = j.values().filter(|m| m.ply >= d).cloned().collect();
        }
    }
    // Clocks: from the journal, unless the store holds a later record (D-66).
    if input.db_seq <= input.last_intact_seq {
        plan.clocks = input
            .journal
            .clocks
            .filter(|clocks| *clocks != input.stored_clocks);
    }
    plan
}

/// The model of applying a plan to stored rows and clocks (what `apply_records` does with SQL
/// in part D3). Used by the property test and by tests of the rebase.
pub fn apply_plan(
    stored: &[StoredMove],
    stored_clocks: ClockPair,
    plan: &Plan,
) -> (Vec<StoredMove>, ClockPair) {
    let mut rows: BTreeMap<u32, StoredMove> = stored.iter().map(|m| (m.ply, m.clone())).collect();
    if let Some(from) = plan.delete_from() {
        let _ = rows.split_off(&from);
    }
    for row in &plan.rows_to_write {
        rows.insert(row.ply, row.clone());
    }
    (
        rows.into_values().collect(),
        plan.clocks.unwrap_or(stored_clocks),
    )
}

/// The `GAME` clocks after the plan: the plan's, else the stored ones (D-66 case).
pub fn reconciled_clocks(plan: &Plan, stored_clocks: ClockPair) -> ClockPair {
    plan.clocks.unwrap_or(stored_clocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::format::{ClockPayload, DrawByPayload, MovePayload, Side, TakebackPayload};
    use proptest::prelude::*;

    fn mv(seq: u64, ply: u32, uci: &str, clocks: ClockPair) -> JournalRecord {
        JournalRecord {
            seq,
            wall_ms: 100 + i64::try_from(seq).unwrap(),
            payload: Payload::Move(MovePayload {
                ply,
                uci: uci.into(),
                think_ms: 5,
                white_ms: clocks.map(|c| c.0),
                black_ms: clocks.map(|c| c.1),
                san: format!("san-{uci}"),
                fen_after: format!("fen-{ply}-{uci}"),
            }),
        }
    }

    fn row(ply: u32, uci: &str) -> StoredMove {
        StoredMove {
            ply,
            uci: uci.into(),
            san: format!("san-{uci}"),
            fen_after: format!("fen-{}-{uci}", ply - 1),
            think_ms: Some(5),
            clocks: Some((1, 2)),
            played_at: None,
        }
    }

    fn input<'a>(
        j: &'a Folded,
        s: &'a [StoredMove],
        scan_damaged: bool,
        db_seq: u64,
        last: u64,
    ) -> ReconcileInput<'a> {
        ReconcileInput {
            journal: j,
            stored: s,
            scan_damaged,
            db_seq,
            last_intact_seq: last,
            stored_clocks: Some((1, 2)),
        }
    }

    fn journal3() -> Folded {
        fold(&[
            mv(2, 0, "e2e4", Some((1, 2))),
            mv(3, 1, "e7e5", Some((1, 2))),
            mv(4, 2, "g1f3", Some((1, 2))),
        ])
    }

    /// UT-ST-014: `Move` sets ply `ply + 1` and drops later plies; `Takeback` drops above
    /// `to_ply`; other kinds change nothing; the journal ply 0 is `MOVE.ply` 1 (D-92).
    #[test]
    fn ut_st_014_fold_handles_move_takeback_and_ply_base() {
        let f = journal3();
        assert_eq!(f.moves.keys().copied().collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(f.moves[&1].uci, "e2e4", "journal ply 0 is MOVE.ply 1");
        let taken = fold(&[
            mv(1, 0, "a", None),
            mv(2, 1, "b", None),
            mv(3, 2, "c", None),
            JournalRecord {
                seq: 4,
                wall_ms: 0,
                payload: Payload::Takeback(TakebackPayload { to_ply: 1 }),
            },
            JournalRecord {
                seq: 5,
                wall_ms: 0,
                payload: Payload::DrawOffer(DrawByPayload { by: Side::White }),
            },
            JournalRecord {
                seq: 6,
                wall_ms: 0,
                payload: Payload::Pause,
            },
        ]);
        assert_eq!(taken.moves.keys().copied().collect::<Vec<_>>(), vec![1]);
        // A replayed ply drops the later ones: Move ply 1 after three moves.
        let replayed = fold(&[
            mv(1, 0, "a", None),
            mv(2, 1, "b", None),
            mv(3, 2, "c", None),
            mv(4, 1, "B", None),
        ]);
        assert_eq!(
            replayed.moves.keys().copied().collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(replayed.moves[&2].uci, "B");
    }

    /// UT-ST-013: clocks from the last `Move` or `Clock` record, or the stored ones when the
    /// store holds a later record.
    #[test]
    fn ut_st_013_clocks_come_from_the_journal_unless_the_store_is_ahead() {
        let mut records = vec![mv(1, 0, "a", Some((10, 20)))];
        records.push(JournalRecord {
            seq: 2,
            wall_ms: 0,
            payload: Payload::Clock(ClockPayload {
                white_ms: Some(7),
                black_ms: Some(8),
                running_side: None,
            }),
        });
        let f = fold(&records);
        assert_eq!(f.clocks, Some(Some((7, 8))));
        let rows = [row(1, "a")];
        let plan = reconcile(&input(&f, &rows, false, 2, 2));
        assert_eq!(plan.clocks, Some(Some((7, 8))));
        let ahead = reconcile(&input(&f, &rows, false, 3, 2));
        assert_eq!(
            ahead.clocks, None,
            "the stored clocks belong to a later record"
        );
        // An untimed game: null clocks in the record clear stored clocks.
        let untimed = fold(&[mv(1, 0, "a", None)]);
        let plan = reconcile(&input(&untimed, &rows, false, 1, 1));
        assert_eq!(plan.clocks, Some(None));
        // No Move or Clock record: nothing to set.
        let none = fold(&[]);
        assert_eq!(reconcile(&input(&none, &[], false, 0, 0)).clocks, None);
    }

    /// UT-ST-010: the intact journal wins over differing rows; extra rows go.
    #[test]
    fn ut_st_010_journal_wins_and_extra_rows_go() {
        let j = journal3();
        let stored = [
            row(1, "e2e4"),
            row(2, "WRONG"),
            row(3, "g1f3"),
            row(4, "extra"),
        ];
        let plan = reconcile(&input(&j, &stored, false, 4, 4));
        assert_eq!(plan.replace_from, Some(2));
        assert_eq!(plan.rows_to_delete_from, Some(4));
        assert_eq!(plan.source, RunSource::Journal);
        assert!(!plan.damaged);
        assert_eq!(
            plan.rows_to_write.iter().map(|m| m.ply).collect::<Vec<_>>(),
            vec![2, 3]
        );
        let (after, _) = apply_plan(&stored, Some((1, 2)), &plan);
        assert_eq!(
            after.iter().map(|m| m.uci.as_str()).collect::<Vec<_>>(),
            vec!["e2e4", "e7e5", "g1f3"]
        );
        // Equal rows: the empty plan. Missing rows are written.
        let same = [row(1, "e2e4"), row(2, "e7e5"), row(3, "g1f3")];
        assert!(reconcile(&input(&j, &same, false, 4, 4)).is_empty());
        let short = reconcile(&input(&j, &same[..1], false, 4, 4));
        assert_eq!(short.replace_from, Some(2));
        assert_eq!(short.rows_to_write.len(), 2);
    }

    /// UT-ST-011: damaged middle, the runs agree, the store is longer: the rows stay.
    #[test]
    fn ut_st_011_damaged_agreeing_and_store_longer_keeps_the_store() {
        let j = fold(&[mv(1, 0, "a", Some((1, 2)))]);
        let stored = [row(1, "a"), row(2, "b"), row(3, "c")];
        let plan = reconcile(&input(&j, &stored, true, 1, 1));
        assert_eq!(plan.source, RunSource::Store);
        assert!(plan.damaged);
        assert_eq!((plan.replace_from, plan.rows_to_delete_from), (None, None));
        assert!(plan.rows_to_write.is_empty());
        // Journal longer: its rows are written and it is the source.
        let j3 = journal3();
        let plan = reconcile(&input(&j3, &[row(1, "e2e4")], true, 4, 4));
        assert_eq!(plan.source, RunSource::Journal);
        assert_eq!(plan.rows_to_write.len(), 2);
        // Equal length: Both.
        let same = [row(1, "e2e4"), row(2, "e7e5"), row(3, "g1f3")];
        assert_eq!(
            reconcile(&input(&j3, &same, true, 4, 4)).source,
            RunSource::Both
        );
    }

    /// The third damaged row: the two differ at ply `d`, the journal wins from `d` and rows
    /// beyond its end are dropped.
    #[test]
    fn a_damaged_journal_that_differs_wins_from_the_first_difference() {
        let j = journal3();
        let stored = [
            row(1, "e2e4"),
            row(2, "XX"),
            row(3, "g1f3"),
            row(4, "extra"),
            row(5, "more"),
        ];
        let plan = reconcile(&input(&j, &stored, true, 6, 6));
        assert_eq!(plan.replace_from, Some(2));
        assert_eq!(plan.rows_to_delete_from, Some(4));
        assert_eq!(plan.source, RunSource::Journal);
        let (after, _) = apply_plan(&stored, Some((1, 2)), &plan);
        assert_eq!(after.len(), 3);
        assert_eq!(after[1].uci, "e7e5");
    }

    /// UT-ST-083 (D-66): `db_seq > last_intact_seq` is damage for a clean end, a torn tail and
    /// a damaged middle alike: the stored rows beyond the journal stay, `source = Store`, and
    /// the stored clocks are used.
    #[test]
    fn ut_st_083_a_store_ahead_of_the_journal_is_damage_and_keeps_its_rows() {
        let j = journal3(); // last intact seq 4
        let stored = [
            row(1, "e2e4"),
            row(2, "e7e5"),
            row(3, "g1f3"),
            row(4, "g8f6"),
        ];
        for scan_damaged in [false, true] {
            let plan = reconcile(&input(&j, &stored, scan_damaged, 5, 4));
            assert!(plan.damaged, "scan_damaged {scan_damaged}");
            assert_eq!(plan.source, RunSource::Store);
            assert!(plan.is_empty(), "{plan:?}");
            assert_eq!(plan.clocks, None, "the stored clocks stand");
        }
        // Without D-66 (db_seq == last_intact_seq) the same rows would be deleted.
        let plan = reconcile(&input(&j, &stored, false, 4, 4));
        assert_eq!(plan.rows_to_delete_from, Some(4));
    }

    #[test]
    fn a_null_stored_value_matches_the_journal_so_a_synthesized_row_is_not_a_difference() {
        let j = journal3();
        let mut stored: Vec<StoredMove> = j.moves.values().cloned().collect();
        for m in &mut stored {
            m.think_ms = None;
            m.clocks = None;
            m.played_at = None;
        }
        assert!(reconcile(&input(&j, &stored, false, 4, 4)).is_empty());
        // A non-NULL value that differs is a difference.
        stored[1].think_ms = Some(999);
        assert_eq!(
            reconcile(&input(&j, &stored, false, 4, 4)).replace_from,
            Some(2)
        );
        // So is a different clock pair.
        stored[1].think_ms = None;
        stored[1].clocks = Some((1, 1));
        assert_eq!(
            reconcile(&input(&j, &stored, false, 4, 4)).replace_from,
            Some(2)
        );
        // So is a different fen_after or san.
        stored[1].clocks = None;
        stored[1].think_ms = None;
        stored[1].fen_after = "other".into();
        assert_eq!(
            reconcile(&input(&j, &stored, false, 4, 4)).replace_from,
            Some(2)
        );
    }

    fn arb_move(ply: u32) -> impl Strategy<Value = StoredMove> {
        (
            0u8..3,
            proptest::option::of(0u64..3),
            proptest::option::of((0u64..2, 0u64..2)),
            proptest::option::of(0i64..3),
        )
            .prop_map(move |(u, think, clocks, played)| StoredMove {
                ply,
                uci: format!("m{u}"),
                san: format!("s{u}"),
                fen_after: format!("f{u}"),
                think_ms: think,
                clocks,
                played_at: played,
            })
    }

    fn arb_rows() -> impl Strategy<Value = Vec<StoredMove>> {
        proptest::collection::btree_set(1u32..7, 0..6)
            .prop_flat_map(|plies| plies.into_iter().map(arb_move).collect::<Vec<_>>())
    }

    proptest! {
        /// UT-ST-012: for arbitrary journals and stores (gappy ones included), damage flags and
        /// sequence numbers, applying the plan and reconciling again gives an empty plan.
        #[test]
        fn ut_st_012_the_plan_is_idempotent(
            journal_rows in arb_rows(),
            stored in arb_rows(),
            jclocks in proptest::option::of(proptest::option::of((0u64..2, 0u64..2))),
            stored_clocks in proptest::option::of((0u64..2, 0u64..2)),
            scan_damaged in any::<bool>(),
            db_seq in 0u64..8,
            last in 0u64..8,
        ) {
            let folded = Folded {
                moves: journal_rows.into_iter().map(|mut m| {
                    m.think_ms = m.think_ms.or(Some(0));
                    m.played_at = m.played_at.or(Some(0));
                    (m.ply, m)
                }).collect(),
                clocks: jclocks,
            };
            let first = reconcile(&ReconcileInput {
                journal: &folded, stored: &stored, scan_damaged, db_seq,
                last_intact_seq: last, stored_clocks,
            });
            let (rows, clocks) = apply_plan(&stored, stored_clocks, &first);
            let second = reconcile(&ReconcileInput {
                journal: &folded, stored: &rows, scan_damaged, db_seq,
                last_intact_seq: last, stored_clocks: clocks,
            });
            prop_assert!(second.is_empty(), "first {first:?} second {second:?}");
        }

        /// The function is deterministic.
        #[test]
        fn reconcile_is_deterministic(stored in arb_rows(), flag in any::<bool>()) {
            let j = journal3();
            let a = reconcile(&input(&j, &stored, flag, 3, 4));
            let b = reconcile(&input(&j, &stored, flag, 3, 4));
            prop_assert_eq!(a, b);
        }
    }
}
