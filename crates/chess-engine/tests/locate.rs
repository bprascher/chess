//! UT-EN-040..046 (binary discovery).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: the assertions abort a failing test"
)]

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use chess_engine::{BinaryLocator, BinarySource, EngineError};

fn make(dir: &Path, name: &str, mode: u32) -> PathBuf {
    let p = dir.join(name);
    fs::write(&p, b"#!/bin/sh\n").unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(mode)).unwrap();
    p
}

fn loc(over: Option<&str>, exe: &Path) -> BinaryLocator {
    BinaryLocator::new(over.map(OsString::from), exe.to_path_buf())
}

#[test]
fn ut_en_040_an_override_must_be_an_absolute_path() {
    let d = tempfile::tempdir().unwrap();
    let exe = make(d.path(), "chess", 0o755);
    assert_eq!(
        loc(Some("stockfish"), &exe).locate(),
        Err(EngineError::InvalidOverride)
    );
    assert_eq!(
        loc(Some("./stockfish"), &exe).locate(),
        Err(EngineError::InvalidOverride)
    );
    assert_eq!(
        loc(Some("../x/stockfish"), &exe).locate(),
        Err(EngineError::InvalidOverride)
    );
}

#[test]
fn ut_en_041_an_absolute_override_is_used_and_marked_as_an_override() {
    let d = tempfile::tempdir().unwrap();
    let exe = make(d.path(), "chess", 0o755);
    let fake = make(d.path(), "fake", 0o755);
    let got = loc(Some(fake.to_str().unwrap()), &exe).locate().unwrap();
    assert_eq!((got.path, got.source), (fake, BinarySource::Override));
}

#[test]
fn ut_en_042_an_empty_override_means_not_set() {
    let d = tempfile::tempdir().unwrap();
    let exe = make(d.path(), "chess", 0o755);
    let sf = make(d.path(), "stockfish", 0o755);
    let got = loc(Some(""), &exe).locate().unwrap();
    assert_eq!(
        (got.path, got.source),
        (sf.canonicalize().unwrap(), BinarySource::Bundled)
    );
}

#[test]
fn ut_en_043_missing_and_unsafe_files_are_typed_errors() {
    let d = tempfile::tempdir().unwrap();
    let exe = make(d.path(), "chess", 0o755);
    assert_eq!(loc(None, &exe).locate(), Err(EngineError::BinaryMissing));
    let sf = d.path().join("stockfish");
    fs::create_dir(&sf).unwrap();
    assert_eq!(
        loc(None, &exe).locate(),
        Err(EngineError::NotExecutable),
        "a directory"
    );
    fs::remove_dir(&sf).unwrap();
    for (mode, ok) in [
        (0o755, true),
        (0o700, true),
        (0o644, false),
        (0o600, false),
        (0o775, false),
        (0o757, false),
        (0o777, false),
    ] {
        make(d.path(), "stockfish", mode);
        fs::set_permissions(&sf, fs::Permissions::from_mode(mode)).unwrap();
        assert_eq!(loc(None, &exe).locate().is_ok(), ok, "mode {mode:o}");
    }
    let missing = d.path().join("nope");
    assert_eq!(
        loc(Some(missing.to_str().unwrap()), &exe).locate(),
        Err(EngineError::BinaryMissing)
    );
    assert_eq!(
        loc(None, &d.path().join("no-such-exe")).locate(),
        Err(EngineError::BinaryMissing)
    );
}

#[test]
fn ut_en_044_bundled_is_the_stockfish_next_to_the_executable() {
    let d = tempfile::tempdir().unwrap();
    let exe = make(d.path(), "chess", 0o755);
    let sf = make(d.path(), "stockfish", 0o755);
    let got = loc(None, &exe).locate().unwrap();
    assert_eq!(got.source, BinarySource::Bundled);
    assert_eq!(got.path, sf.canonicalize().unwrap());
}

#[test]
fn ut_en_045_a_symlinked_executable_is_resolved_first() {
    let real = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let exe = make(real.path(), "chess", 0o755);
    let sf = make(real.path(), "stockfish", 0o755);
    let link = other.path().join("chess");
    symlink(&exe, &link).unwrap();
    make(other.path(), "stockfish", 0o755); // a decoy next to the link, which must not be used
    let got = loc(None, &link).locate().unwrap();
    assert_eq!(got.path, sf.canonicalize().unwrap());
}

#[test]
fn ut_en_046_the_path_variable_is_never_read() {
    // The locator takes its two inputs explicitly, so no ambient value can reach it. This
    // second check is a source review by test and is deliberately weak: the only environment
    // variable named in the source is CHESS_STOCKFISH.
    let src = include_str!("../src/locate.rs");
    assert!(!src.contains("\"PATH\""));
    assert!(!src.contains("which"));
    let vars: Vec<&str> = src
        .match_indices("var_os(")
        .map(|(i, _)| &src[i..i + 30])
        .collect();
    assert_eq!(vars.len(), 1, "{vars:?}");
    assert!(vars[0].contains("CHESS_STOCKFISH"));
    // A decoy on a would-be path does not matter: nothing but the sibling file is consulted.
    let d = tempfile::tempdir().unwrap();
    let exe = make(d.path(), "chess", 0o755);
    let decoy = tempfile::tempdir().unwrap();
    make(decoy.path(), "stockfish", 0o755);
    assert_eq!(loc(None, &exe).locate(), Err(EngineError::BinaryMissing));
}

#[test]
fn ut_en_047_errors_do_not_show_the_path() {
    let d = tempfile::tempdir().unwrap();
    let exe = make(d.path(), "chess", 0o755);
    let e = loc(None, &exe).locate().unwrap_err();
    assert!(!e.to_string().contains(d.path().to_str().unwrap()));
}
