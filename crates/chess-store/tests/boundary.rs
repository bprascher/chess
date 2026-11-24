//! The API boundary (UT-ST-038, TR-3, D-40) and the error catalogue (UT-ST-080, NFR-9, NFR-10).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers and proptest closures here exist to abort a failing fixture"
)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use chess_domain::{Mode, PlayerId};
use chess_store::{BackupCause, Entity, StoreError, UnavailableCause};
use uuid::Uuid;

/// The production part of a source file: everything before its `#[cfg(test)]` module.
fn source(name: &str) -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "src", name].iter().collect();
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    match text.find("\n#[cfg(test)]") {
        Some(at) => text[..at].to_owned(),
        None => text,
    }
}

/// A line that declares a field or parameter named like an identity with an integer type.
fn integer_identity_declarations(text: &str) -> Vec<String> {
    const NAMES: [&str; 8] = [
        "id",
        "game_id",
        "player_id",
        "white_id",
        "black_id",
        "voided_by",
        "game",
        "player",
    ];
    const INTS: [&str; 6] = ["i64", "u64", "i32", "u32", "usize", "isize"];
    let mut found = Vec::new();
    for line in text.lines() {
        let code = line.split("//").next().unwrap_or("");
        for name in NAMES {
            let mut rest = code;
            while let Some(at) = rest.find(&format!("{name}:")) {
                let before_ok = rest[..at]
                    .chars()
                    .next_back()
                    .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
                let after = rest[at + name.len() + 1..].trim_start();
                let after = after.strip_prefix("Option<").unwrap_or(after);
                if before_ok && INTS.iter().any(|t| after.starts_with(t)) {
                    found.push(line.trim().to_owned());
                }
                rest = &rest[at + name.len() + 1..];
            }
        }
    }
    found
}

#[test]
fn ut_st_038_no_public_type_or_method_takes_or_returns_an_integer_identity() {
    for file in ["api.rs", "store.rs", "error.rs", "datadir.rs"] {
        let found = integer_identity_declarations(&source(file));
        assert!(found.is_empty(), "{file}: {found:?}");
    }
}

#[test]
fn ut_st_038_the_scanner_itself_finds_what_it_is_meant_to_find() {
    let bad = "pub struct X {\n    pub game_id: i64,\n    pub white: Option<u32>,\n    pub player: Option<i64>,\n}\npub fn f(id: u64) {}";
    assert_eq!(integer_identity_declarations(bad).len(), 3);
    assert!(
        integer_identity_declarations("pub id: PlayerId, pub game: Option<GameId>, pub ply: u32")
            .is_empty()
    );
}

#[test]
fn ut_st_038_repositories_are_private_so_no_sql_or_row_key_leaves_the_crate() {
    let lib = source("lib.rs");
    assert!(lib.contains("mod repo;") && !lib.contains("pub mod repo"));
    assert!(!lib.contains("pub mod thread") && !lib.contains("pub mod readpool"));
    assert!(lib.contains("#![forbid(unsafe_code)]"));
}

#[test]
fn sql_text_exists_only_where_the_layering_rule_allows_it() {
    // SDD 3.8: no SQL text in thread.rs and store.rs; Connection::execute only in repo/,
    // migrate.rs, open.rs, backup.rs.
    for file in [
        "thread.rs",
        "store.rs",
        "readpool.rs",
        "api.rs",
        "error.rs",
        "datadir.rs",
    ] {
        let text = source(file);
        for word in [
            "SELECT ", "INSERT ", "UPDATE ", "DELETE ", "execute(", "prepare(",
        ] {
            assert!(!text.contains(word), "{file} contains {word:?}");
        }
    }
}

fn every_error() -> Vec<StoreError> {
    let secret = Path::new("/Users/someone/secret place/chess.db").to_path_buf();
    vec![
        StoreError::Busy,
        StoreError::Unavailable {
            cause: UnavailableCause::DiskFull,
        },
        StoreError::Unavailable {
            cause: UnavailableCause::Io(std::io::ErrorKind::Other),
        },
        StoreError::Corrupt,
        StoreError::Missing,
        StoreError::SchemaTooNew { db: 9, app: 1 },
        StoreError::BackupFailed {
            cause: BackupCause::TargetExists,
        },
        StoreError::SlotTaken {
            game_id: common::gid(1),
            mode: Mode::Strict,
            player_id: PlayerId::from_uuid(Uuid::from_u128(77)),
        },
        StoreError::HasUnfinishedGames {
            ids: vec![common::gid(1)],
        },
        StoreError::UnlistedUnfinishedGames {
            ids: vec![common::gid(2)],
        },
        StoreError::NameTaken,
        StoreError::NotFound {
            entity: Entity::Game,
            id: Uuid::from_u128(5),
        },
        StoreError::RosterIndexTaken,
        StoreError::InvariantViolated,
        StoreError::DurabilityUnsupported {
            path: secret,
            errno: 45,
        },
        StoreError::Internal,
    ]
}

#[test]
fn ut_st_080_display_prints_a_category_only_no_path_sql_or_identifier() {
    for error in every_error() {
        let text = error.to_string();
        assert!(!text.is_empty());
        for forbidden in [
            "/",
            "chess.db",
            "SELECT",
            "INSERT",
            "UPDATE",
            "constraint",
            "0000",
            "45",
            "secret",
        ] {
            assert!(
                !text.contains(forbidden),
                "{error:?} prints {text:?} (contains {forbidden:?})"
            );
        }
    }
}

#[test]
fn durability_unsupported_carries_the_path_and_errno_in_the_variant_only() {
    let error = StoreError::DurabilityUnsupported {
        path: PathBuf::from("/Volumes/X"),
        errno: 45,
    };
    match &error {
        StoreError::DurabilityUnsupported { path, errno } => {
            assert_eq!((path.as_path(), *errno), (Path::new("/Volumes/X"), 45));
        }
        other => panic!("{other:?}"),
    }
    assert!(!error.to_string().contains("Volumes"));
}
