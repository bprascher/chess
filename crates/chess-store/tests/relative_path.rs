//! FIND-009: a relative data directory whose name looks like a SQLite URI is a path, not a URI.
//! Own test binary: it changes the working directory of the process.
#![allow(
    clippy::unwrap_used,
    reason = "integration-test crate: the helpers here exist to abort a failing fixture"
)]

mod common;

use chess_store::{DataDir, Store, StoreOptions};
use common::block_on;

#[test]
fn a_relative_directory_named_like_a_sqlite_uri_is_used_as_a_path() {
    let scratch = tempfile::TempDir::new().unwrap();
    std::env::set_current_dir(scratch.path()).unwrap();
    let name = "file:dd?mode=memory&cache=shared";
    let opened = block_on(Store::open(StoreOptions::new(DataDir::new(name)))).unwrap();
    assert!(opened.report.first_launch);
    drop(opened);
    assert!(scratch.path().join(name).join("chess.db").is_file());
}
