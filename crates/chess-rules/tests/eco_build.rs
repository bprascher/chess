//! CT-RUL-003 (the generator fails with file and line on a bad row, FM-RUL-006) and the
//! generator's duplicate rule (UR-27, SDD 3.11.9). The generator is the same source file that
//! `build.rs` includes (`build/eco_gen.rs`); here it runs on temporary copies of the data.
//! Requirements DAT-3, D-20, D-75.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helper functions here exist to abort a failing fixture"
)]
#![allow(
    dead_code,
    reason = "the included generator module has items that only build.rs uses"
)]

#[path = "../build/eco_gen.rs"]
mod eco_gen;
#[path = "../src/key_derive.rs"]
mod key_derive;

use std::path::{Path, PathBuf};

fn real_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../third_party/eco")
}

/// A fresh temporary directory holding the real b..e files and the given `a.tsv` text.
fn scratch(name: &str, a_tsv: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chess-rules-eco-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["b", "c", "d", "e"] {
        std::fs::copy(
            real_dir().join(format!("{f}.tsv")),
            dir.join(format!("{f}.tsv")),
        )
        .unwrap();
    }
    std::fs::write(dir.join("a.tsv"), a_tsv).unwrap();
    dir
}

fn gen_err(name: &str, a_tsv: &str) -> String {
    let dir = scratch(name, a_tsv);
    let r = eco_gen::generate(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    r.err().expect("the generator must fail")
}

#[test]
fn ct_rul_003_d_20_the_real_snapshot_generates() {
    let g = eco_gen::generate(&real_dir()).unwrap();
    assert!(g.rows > 3000 && g.entries.len() <= g.rows);
    assert_eq!(g.rows - g.entries.len(), g.duplicates);
    assert!(g.source.contains("static TABLE"));
    // Sorted by key, no repeats: the lookup is a binary search.
    assert!(g.entries.windows(2).all(|w| w[0].epd < w[1].epd));
}

#[test]
fn ct_rul_003_fm_rul_006_an_illegal_san_fails_with_file_line_and_token() {
    let msg = gen_err(
        "illegal",
        "eco\tname\tpgn\nA00\tGood\t1. Nh3\nA00\tAlso good\t1. e4\nA00\tBad\t1. e4 e5 2. Nh5\n",
    );
    assert!(msg.contains("a.tsv:4"), "{msg}");
    assert!(msg.contains("Nh5"), "{msg}");
}

#[test]
fn ct_rul_003_fm_rul_006_a_malformed_row_a_bad_header_or_a_missing_file_fails() {
    let msg = gen_err(
        "cols",
        "eco\tname\tpgn\nA00\tGood\t1. Nh3\nA00 only two columns\n",
    );
    assert!(msg.contains("a.tsv:3"), "{msg}");
    let msg = gen_err("header", "code\tname\tmoves\nA00\tGood\t1. Nh3\n");
    assert!(msg.contains("a.tsv:1"), "{msg}");
    let dir = scratch("missing", "eco\tname\tpgn\nA00\tGood\t1. Nh3\n");
    std::fs::remove_file(dir.join("c.tsv")).unwrap();
    let msg = eco_gen::generate(&dir).err().expect("missing file");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(msg.contains("c.tsv"), "{msg}");
    let msg = gen_err("empty-pgn", "eco\tname\tpgn\nA00\tGood\t\n");
    assert!(msg.contains("a.tsv:2"), "{msg}");
}

fn entry_for<'a>(g: &'a eco_gen::Generated, name: &str) -> Option<&'a eco_gen::Entry> {
    g.entries.iter().find(|e| e.name == name)
}

#[test]
fn ut_rul_023_ur_27_a_duplicate_position_keeps_the_longer_line_then_the_smaller_code_then_name() {
    // "1. Nf3" and the 5-ply line "1. Nf3 Nf6 2. Ng1 Ng8 3. Nf3" reach one position.
    let g = {
        let dir = scratch(
            "dup-length",
            "eco\tname\tpgn\nZ99\tShort\t1. Nf3\nZ98\tLong\t1. Nf3 Nf6 2. Ng1 Ng8 3. Nf3\n",
        );
        let g = eco_gen::generate(&dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        g
    };
    assert!(entry_for(&g, "Long").is_some() && entry_for(&g, "Short").is_none());
    // Equal length: smaller code, then smaller name.
    let dir = scratch(
        "dup-tie",
        "eco\tname\tpgn\nD10\tTwo\t1. d4 d5 2. c4\nD06\tOne\t1. c4 d5 2. d4\nD06\tAardvark\t1. d4 d5 2. c4\n",
    );
    let g = eco_gen::generate(&dir).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(entry_for(&g, "Aardvark").is_some());
    assert!(entry_for(&g, "One").is_none() && entry_for(&g, "Two").is_none());
    assert!(g.duplicates >= 2);
}

fn _unused(_: &Path) {}
