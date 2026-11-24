//! DD-ST-017 `JournalReader`: the scan with torn-tail and damage detection (SDD 3.11.2).
//!
//! ```text
//! header: bad magic / checksum / other game -> Unreadable; version > build -> VersionMismatch;
//!         store id differs -> ForeignStore
//! pos = end of header, expected_seq = 1
//! loop: pos == EOF -> clean
//!       a record is bad when: sync wrong, len past the end or above 65 536 (or below the fixed
//!       fields), crc wrong, kind invalid, payload refused, or seq != expected_seq
//!       bad -> look ahead for the next sync marker whose record has a valid crc and
//!              seq > last_good_seq: found -> damaged middle; none, but some CRC-valid frame
//!              (refused payload, kind or seq) in the dropped region -> `UndecodableTail`
//!              (damage, CH-5 B); none at all -> torn tail, dropped silently
//! ```
//! The scan classifies by the journal alone. The caller adds the D-66 rule (`GAME.journal_seq`
//! greater than the last intact `seq` makes a dropped tail damage, `DamageKind::StoreAhead`),
//! which needs the store and lives in reconcile and recovery.
//!
//! A record whose checksum is right but which the decoder refuses (payload, kind, `seq`) is never
//! returned and never applied. At the tail it is reported as damage, not dropped (CH-5 B, Brian
//! Prascher 2026-11-22): the caller feeds `scan.damaged.is_some()` into the unchanged
//! `ReconcileInput.scan_damaged`.

use std::fs;
use std::path::Path;

use uuid::Uuid;

use crate::error::JournalError;
use crate::journal::format::{
    FrameError, HEADER_LEN, Header, JOURNAL_VERSION, JournalRecord, SYNC, decode_frame,
    decode_header, read_frame,
};

/// Where a damaged middle is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageKind {
    /// A bad record at byte `at` with an intact record after it, starting at `resumes_at`.
    Middle { at: usize, resumes_at: usize },
    /// The store applied a record that the journal no longer proves (D-66). Set by the caller.
    StoreAhead,
    /// CH-5 B: a CRC-valid frame that the decoder refuses (payload, kind or `seq`) sits at `at`
    /// in the dropped tail, with nothing resumable after it. It was acknowledged by some writer,
    /// so it is damage on the D-66 path and never a silent torn tail. The intact run still ends
    /// before `intact_end`; `at` is the first such frame at or after it.
    UndecodableTail { at: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamagedInfo {
    pub kind: DamageKind,
}

/// The result of reading one journal file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scan {
    pub header: Header,
    /// The intact run: records `1..=n`, in order.
    pub records: Vec<JournalRecord>,
    /// Bytes from the start of the file to the end of the last intact record. Everything after
    /// it is a torn tail or damage.
    pub intact_end: usize,
    /// A bad tail with nothing intact after it was dropped (never acknowledged).
    pub torn_tail_dropped: bool,
    pub damaged: Option<DamagedInfo>,
}

impl Scan {
    /// `seq` of the last intact record, 0 when there is none.
    pub fn last_seq(&self) -> u64 {
        self.records.last().map_or(0, |r| r.seq)
    }
}

/// Scans a whole journal held in memory. `game` and `store` are what the file must belong to.
pub fn scan_bytes(bytes: &[u8], game: Uuid, store: Uuid) -> Result<Scan, JournalError> {
    let header = decode_header(bytes).map_err(|_| JournalError::Unreadable)?;
    if header.version == 0 {
        return Err(JournalError::Unreadable);
    }
    if header.version > JOURNAL_VERSION {
        return Err(JournalError::VersionMismatch {
            found: header.version,
            expected: JOURNAL_VERSION,
        });
    }
    if header.game != game {
        return Err(JournalError::Unreadable);
    }
    if header.store != store {
        return Err(JournalError::ForeignStore);
    }
    let mut records = Vec::new();
    let mut pos = HEADER_LEN;
    let mut expected = 1u64;
    let mut scan = Scan {
        header,
        records: Vec::new(),
        intact_end: HEADER_LEN,
        torn_tail_dropped: false,
        damaged: None,
    };
    while pos < bytes.len() {
        let accepted = read_frame(bytes, pos)
            .ok()
            .filter(|frame| frame.seq == expected)
            .and_then(|frame| decode_frame(&frame).ok().map(|rec| (frame.total, rec)));
        match accepted {
            Some((total, record)) => {
                records.push(record);
                expected += 1;
                pos += total;
            }
            None => {
                let last_good = expected - 1;
                let tail = classify_tail(bytes, pos, last_good);
                scan.damaged = match (tail.resumes_at, tail.crc_valid_at) {
                    (Some(resumes_at), _) => Some(DamagedInfo {
                        kind: DamageKind::Middle {
                            at: pos,
                            resumes_at,
                        },
                    }),
                    (None, Some(at)) => Some(DamagedInfo {
                        kind: DamageKind::UndecodableTail { at },
                    }),
                    (None, None) => {
                        scan.torn_tail_dropped = true;
                        None
                    }
                };
                break;
            }
        }
    }
    scan.records = records;
    scan.intact_end = pos;
    Ok(scan)
}

/// What the bytes from the first bad position on contain.
struct Tail {
    /// The next sync marker after `pos` whose record has a valid checksum and a `seq` above
    /// `last_good` (a damaged middle).
    resumes_at: Option<usize>,
    /// The first sync marker at or after `pos` whose frame has a valid checksum (a CRC-valid
    /// frame, even one the decoder refuses: CH-5 B).
    crc_valid_at: Option<usize>,
}

/// Looks through the dropped region. Bounded by the file size; one checksum per marker found.
/// Stops at the first resume point, which decides the outcome.
fn classify_tail(bytes: &[u8], pos: usize, last_good: u64) -> Tail {
    let mut tail = Tail {
        resumes_at: None,
        crc_valid_at: None,
    };
    let mut at = pos;
    while at + 1 < bytes.len() {
        if bytes.get(at..at + 2) == Some(&SYNC[..]) {
            match read_frame(bytes, at) {
                Ok(frame) => {
                    if at > pos && frame.seq > last_good {
                        tail.resumes_at = Some(at);
                        return tail;
                    }
                    tail.crc_valid_at.get_or_insert(at);
                }
                // The checksum held and only the kind byte is wrong.
                Err(FrameError::Kind) => {
                    tail.crc_valid_at.get_or_insert(at);
                }
                Err(_) => {}
            }
        }
        at += 1;
    }
    tail
}

/// Reads and scans the file at `path`. A missing file is `Unreadable`.
pub fn scan_file(path: &Path, game: Uuid, store: Uuid) -> Result<Scan, JournalError> {
    let bytes = fs::read(path).map_err(|_| JournalError::Unreadable)?;
    scan_bytes(&bytes, game, store)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::format::{Payload, TakebackPayload, encode_header, encode_record};

    const GAME: u128 = 7;
    const STORE: u128 = 9;

    fn rec(seq: u64) -> Vec<u8> {
        encode_record(&JournalRecord {
            seq,
            wall_ms: i64::try_from(seq).unwrap(),
            payload: Payload::Takeback(TakebackPayload { to_ply: 1 }),
        })
        .unwrap()
    }

    fn file(records: &[u64]) -> Vec<u8> {
        let mut out = encode_header(Uuid::from_u128(GAME), Uuid::from_u128(STORE)).to_vec();
        for seq in records {
            out.extend(rec(*seq));
        }
        out
    }

    fn scan(bytes: &[u8]) -> Result<Scan, JournalError> {
        scan_bytes(bytes, Uuid::from_u128(GAME), Uuid::from_u128(STORE))
    }

    #[test]
    fn a_clean_file_and_a_header_only_file() {
        let s = scan(&file(&[1, 2, 3])).unwrap();
        assert_eq!(s.records.len(), 3);
        assert_eq!(s.last_seq(), 3);
        assert!(!s.torn_tail_dropped && s.damaged.is_none());
        assert_eq!(s.intact_end, file(&[1, 2, 3]).len());
        let empty = scan(&file(&[])).unwrap();
        assert!(empty.records.is_empty());
        assert_eq!((empty.intact_end, empty.last_seq()), (HEADER_LEN, 0));
    }

    /// UT-ST-074 to 076: torn tails (short prefix, short body, bad checksum at the end) are
    /// dropped silently and the intact run is kept.
    #[test]
    fn ut_st_074_torn_tails_are_dropped_silently() {
        let base = file(&[1, 2]);
        let third = rec(3);
        let variants: Vec<Vec<u8>> = vec![
            [base.clone(), third[..4].to_vec()].concat(), // short prefix
            [base.clone(), third[..third.len() - 5].to_vec()].concat(), // short body
            [base.clone(), {
                let mut bad = third.clone();
                let last = bad.len() - 1;
                bad[last] ^= 0xFF;
                bad
            }]
            .concat(), // bad checksum at the end
            [base.clone(), vec![0xA5]].concat(),          // one stray byte
            [base.clone(), vec![0u8; 64]].concat(),       // zero-filled block
        ];
        for (i, bytes) in variants.iter().enumerate() {
            let s = scan(bytes).unwrap();
            assert_eq!(s.records.len(), 2, "variant {i}");
            assert!(s.torn_tail_dropped, "variant {i}");
            assert!(s.damaged.is_none(), "variant {i}");
            assert_eq!(s.intact_end, base.len(), "variant {i}");
        }
    }

    /// UT-ST-077: a bad record with an intact one after it is a damaged middle.
    #[test]
    fn ut_st_077_a_damaged_middle_is_found_by_look_ahead() {
        let mut bytes = file(&[1, 2, 3, 4]);
        let second_at = HEADER_LEN + rec(1).len();
        bytes[second_at + 12] ^= 0xFF; // body of record 2
        let s = scan(&bytes).unwrap();
        assert_eq!(s.records.len(), 1);
        assert!(!s.torn_tail_dropped);
        assert_eq!(
            s.damaged.unwrap().kind,
            DamageKind::Middle {
                at: second_at,
                resumes_at: second_at + rec(2).len()
            }
        );
        assert_eq!(s.intact_end, second_at);
    }

    #[test]
    fn a_wrong_sequence_number_is_bad_and_a_repeated_seq_is_not_resumable() {
        // 1, 2, 2: the repeat is not seq == 3 and no later record has seq > 2. It is a CRC-valid
        // frame the scan refuses, so it is reported as damage (CH-5 B), not dropped silently.
        let s = scan(&file(&[1, 2, 2])).unwrap();
        assert_eq!(s.records.len(), 2);
        assert!(!s.torn_tail_dropped);
        assert!(matches!(
            s.damaged,
            Some(DamagedInfo {
                kind: DamageKind::UndecodableTail { .. }
            })
        ));
        // 1, 3, 4: record 3 at position 2 is bad; 4 has seq > 1, so this is damage.
        let s = scan(&file(&[1, 3, 4])).unwrap();
        assert_eq!(s.records.len(), 1);
        assert!(matches!(
            s.damaged,
            Some(DamagedInfo {
                kind: DamageKind::Middle { .. }
            })
        ));
    }

    /// R04 (kept) and CH-5 B: after a record with a bad checksum, a CRC-valid repeat of the last
    /// good `seq` is not a resume point (`seq > last_good_seq`, not `>=`), so this is not a damaged
    /// middle. It is still a CRC-valid frame in the dropped region, so it is reported as
    /// `UndecodableTail`, never as a silent torn tail.
    #[test]
    fn a_repeat_of_the_last_good_seq_after_a_bad_record_is_not_a_resume_point() {
        let mut bytes = file(&[1, 2]);
        let mut bad = rec(3);
        let last = bad.len() - 1;
        bad[last] ^= 0xFF;
        let bad_at = bytes.len();
        bytes.extend(&bad);
        let repeat_at = bytes.len();
        bytes.extend(rec(2));
        let s = scan(&bytes).unwrap();
        assert_eq!(s.records.len(), 2);
        assert!(!s.torn_tail_dropped);
        assert_eq!(
            s.damaged.unwrap().kind,
            DamageKind::UndecodableTail { at: repeat_at }
        );
        assert_eq!(s.intact_end, bad_at);
    }

    /// A CRC-valid frame with arbitrary kind byte, `seq` and JSON, for tests.
    fn frame(seq: u64, kind: u8, json: &[u8]) -> Vec<u8> {
        let mut body = seq.to_le_bytes().to_vec();
        body.push(kind);
        body.extend(0i64.to_le_bytes());
        body.extend(json);
        let len = u32::try_from(body.len()).unwrap();
        let mut covered = len.to_le_bytes().to_vec();
        covered.extend(&body);
        let mut out = vec![0xA5, 0x5A];
        out.extend(len.to_le_bytes());
        out.extend(crate::journal::format::crc32(&covered).to_le_bytes());
        out.extend(body);
        out
    }

    /// CH-5 B (Brian, 2026-11-22): every CRC-valid tail frame the decoder refuses is damage on
    /// the D-66 path. Cases of review R01: bad value, unknown kind, kind 0, wrong `seq`, a gap, a
    /// `seq` above `i64::MAX`, a number above `i64::MAX`.
    #[test]
    fn ch5_a_crc_valid_frame_the_decoder_refuses_at_the_tail_is_damage_not_a_torn_tail() {
        let base = file(&[1, 2, 3]);
        let big = (u64::try_from(i64::MAX).unwrap() + 1).to_string();
        let bad_value = format!(r#"{{"white_ms":{big},"black_ms":{big},"running_side":null}}"#);
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("bad value", frame(4, 3, br#"{"to_ply":"x"}"#)),
            ("not json", frame(4, 3, b"\xFF\xFE")),
            ("unknown kind 99", frame(4, 99, b"{}")),
            ("kind 0", frame(4, 0, b"{}")),
            ("kind 15", frame(4, 15, b"{}")),
            ("repeat of seq 3", frame(3, 3, br#"{"to_ply":1}"#)),
            ("gap seq 9", frame(9, 3, br#"{"to_ply":1}"#)),
            ("seq above i64::MAX", frame(u64::MAX, 3, br#"{"to_ply":1}"#)),
            ("number above i64::MAX", frame(4, 4, bad_value.as_bytes())),
        ];
        for (name, tail) in cases {
            let bytes = [base.clone(), tail].concat();
            let s = scan(&bytes).unwrap();
            assert_eq!(s.records.len(), 3, "{name}");
            assert_eq!(s.intact_end, base.len(), "{name}");
            assert!(!s.torn_tail_dropped, "{name}: not a silent torn tail");
            assert_eq!(
                s.damaged,
                Some(DamagedInfo {
                    kind: DamageKind::UndecodableTail { at: base.len() }
                }),
                "{name}"
            );
        }
    }

    /// D3 ruling 2026-11-24: a CRC-valid tail record that breaks a value bound of the format
    /// (`ended_at <= 0` of `GameEnd`, `Abort`, `Voided`; `Start.wall_ms <= 0`; a `Start.matchup`
    /// text that is not `hh`, `hc` or `cc`) is `UndecodableTail` damage (CH-5 B), not a torn
    /// tail and not a record. D4 needs no new rule for it.
    #[test]
    fn d3_a_crc_valid_tail_record_that_breaks_a_value_bound_is_undecodable_tail_damage() {
        let base = file(&[1, 2, 3]);
        let game_end = |ended_at: i64| {
            format!(
                r#"{{"result":"1-0","reason":"checkmate","ended_at":{ended_at},"eco":null,"opening_name":null,"analysis_used":false,"content_hash":"h","white_time_used_ms":0,"black_time_used_ms":0,"total_active_ms":0}}"#
            )
        };
        let start = |matchup: &str| {
            format!(
                r#"{{"game":"{g}","white":"{g}","black":"{g}","white_kind":"human","black_kind":"computer","matchup":"{matchup}","mode":"strict","custom_start":false,"start_fen":null,"clock_spec":null,"rated":false,"self_play":false}}"#,
                g = Uuid::from_u128(1)
            )
        };
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("GameEnd 0", frame(4, 12, game_end(0).as_bytes())),
            ("GameEnd -1", frame(4, 12, game_end(-1).as_bytes())),
            (
                "Abort 0",
                frame(4, 13, br#"{"ended_at":0,"content_hash":"h"}"#),
            ),
            (
                "Voided -4",
                frame(
                    4,
                    14,
                    br#"{"reason":"reset","reset_player":null,"content_hash":"h","ended_at":-4}"#,
                ),
            ),
            // `frame` writes wall_ms 0: a Start is refused for that alone.
            ("Start wall_ms 0", frame(4, 1, start("hc").as_bytes())),
            (
                "Start long matchup",
                frame(4, 1, start("human-computer").as_bytes()),
            ),
        ];
        for (name, tail) in cases {
            let bytes = [base.clone(), tail.clone()].concat();
            let s = scan(&bytes).unwrap();
            assert_eq!(s.records.len(), 3, "{name}");
            assert_eq!(s.intact_end, base.len(), "{name}");
            assert!(!s.torn_tail_dropped, "{name}");
            assert_eq!(
                s.damaged,
                Some(DamagedInfo {
                    kind: DamageKind::UndecodableTail { at: base.len() }
                }),
                "{name}"
            );
        }
        // The same GameEnd with a good time decodes (the case is not refused for another reason).
        let ok = [base.clone(), frame(4, 12, game_end(5).as_bytes())].concat();
        let s = scan(&ok).unwrap();
        assert_eq!((s.records.len(), s.damaged), (4, None));
    }

    /// CH-5 B with the D-66 path of review R01: the database is at 3 (`db_seq == last_intact`)
    /// or at 4. Before the fix the first case planned `damaged = false` and hid the record; now
    /// `scan.damaged` feeds the unchanged `ReconcileInput.scan_damaged` and both are damage.
    #[test]
    fn ch5_the_tail_damage_reaches_reconcile_for_db_seq_3_and_4() {
        use crate::journal::reconcile::{ClockPair, ReconcileInput, fold, reconcile};
        let bytes = [file(&[1, 2, 3]), frame(4, 3, br#"{"to_ply":"x"}"#)].concat();
        let s = scan(&bytes).unwrap();
        let folded = fold(&s.records);
        for db_seq in [3u64, 4] {
            let plan = reconcile(&ReconcileInput {
                journal: &folded,
                stored: &[],
                scan_damaged: s.damaged.is_some(),
                db_seq,
                last_intact_seq: s.last_seq(),
                stored_clocks: ClockPair::default(),
            });
            assert!(plan.damaged, "db_seq {db_seq}");
        }
        // A genuine torn tail stays silent at db_seq == last_intact (no regression).
        let mut torn = rec(4);
        let last = torn.len() - 1;
        torn[last] ^= 0xFF;
        let s = scan(&[file(&[1, 2, 3]), torn].concat()).unwrap();
        assert!(s.torn_tail_dropped && s.damaged.is_none());
        let plan = reconcile(&ReconcileInput {
            journal: &fold(&s.records),
            stored: &[],
            scan_damaged: s.damaged.is_some(),
            db_seq: 3,
            last_intact_seq: s.last_seq(),
            stored_clocks: ClockPair::default(),
        });
        assert!(!plan.damaged);
    }

    /// Spec 3.1 (c): the same refused frame (a number above `i64::MAX`) **followed by** an intact
    /// higher-`seq` frame is a damaged middle, not an undecodable tail.
    #[test]
    fn a_refused_frame_followed_by_an_intact_one_is_a_damaged_middle() {
        let big = r#"{"white_ms":9223372036854775808,"black_ms":1,"running_side":null}"#;
        let base = file(&[1, 2]);
        let refused_at = base.len();
        let refused = frame(3, 4, big.as_bytes());
        let after = rec(4);
        let resumes_at = refused_at + refused.len();
        let s = scan(&[base, refused, after].concat()).unwrap();
        assert_eq!(s.records.len(), 2);
        assert_eq!(
            s.damaged.unwrap().kind,
            DamageKind::Middle {
                at: refused_at,
                resumes_at
            }
        );
        assert!(!s.torn_tail_dropped);
    }

    /// FIND-B-11 B: a frame whose JSON carries a field this build does not know is a good record
    /// (tolerant decode), not a torn tail and not damage.
    #[test]
    fn a_frame_with_an_unknown_json_field_is_read_as_a_record() {
        let bytes = [file(&[1]), frame(2, 3, br#"{"to_ply":1,"future":[1]}"#)].concat();
        let s = scan(&bytes).unwrap();
        assert_eq!(s.records.len(), 2);
        assert!(s.damaged.is_none() && !s.torn_tail_dropped);
    }

    /// UT-ST-078, 079: header faults.
    #[test]
    fn ut_st_078_header_faults_map_to_their_errors() {
        let good = file(&[1]);
        let mut bad_magic = good.clone();
        bad_magic[1] = b'X';
        assert_eq!(scan(&bad_magic).unwrap_err(), JournalError::Unreadable);
        assert_eq!(scan(&good[..20]).unwrap_err(), JournalError::Unreadable);
        assert_eq!(scan(&[]).unwrap_err(), JournalError::Unreadable);
        // Other game in the header: unreadable for this game.
        let other_game = scan_bytes(&good, Uuid::from_u128(1), Uuid::from_u128(STORE));
        assert_eq!(other_game.unwrap_err(), JournalError::Unreadable);
        // Other store.
        let other_store = scan_bytes(&good, Uuid::from_u128(GAME), Uuid::from_u128(1));
        assert_eq!(other_store.unwrap_err(), JournalError::ForeignStore);
        // Higher version with a correct header checksum.
        let mut newer = good.clone();
        newer[4] = 2;
        let crc = crate::journal::format::crc32(&newer[..38]);
        newer[38..42].copy_from_slice(&crc.to_le_bytes());
        assert_eq!(
            scan(&newer).unwrap_err(),
            JournalError::VersionMismatch {
                found: 2,
                expected: 1
            }
        );
        // Version 0 is not a version.
        let mut zero = good;
        zero[4] = 0;
        let crc = crate::journal::format::crc32(&zero[..38]);
        zero[38..42].copy_from_slice(&crc.to_le_bytes());
        assert_eq!(scan(&zero).unwrap_err(), JournalError::Unreadable);
    }

    proptest::proptest! {
        /// Whatever happens to the tail, the scan returns the intact prefix and never panics;
        /// flipping one byte after record `k` never loses records before it.
        #[test]
        fn a_flipped_byte_never_loses_earlier_records(
            n in 1usize..6, flip in 0usize..400, bit in 0u8..8,
        ) {
            let seqs: Vec<u64> = (1..=u64::try_from(n).unwrap()).collect();
            let mut bytes = file(&seqs);
            let at = HEADER_LEN + flip % (bytes.len() - HEADER_LEN);
            bytes[at] ^= 1 << bit;
            let record_len = rec(1).len();
            let hit = (at - HEADER_LEN) / record_len;
            let s = scan(&bytes).unwrap();
            proptest::prop_assert!(s.records.len() >= hit.min(n));
            for (i, r) in s.records.iter().enumerate() {
                proptest::prop_assert_eq!(r.seq, u64::try_from(i).unwrap() + 1);
            }
        }
    }
}
