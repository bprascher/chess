//! Acceptance criterion "DDL equals architecture 7.2". Two guards, both reading the documents
//! at test time so a change to either side fails here:
//!  1. every table of the ER block in architecture.md section 7.2 exists in the migrated database
//!     with the same columns, in the same order, the same primary-key columns and a compatible
//!     type (int, bool and datetime are INTEGER by SDD 3.5.3 row 1; text is TEXT);
//!  2. the DDL block of SDD-chess-store 3.5.2 and migrations/0001_init.sql contain the same
//!     statements once comments and white space are removed.
//!
//! The ER block carries no constraints; constraint behaviour is tested in schema.rs.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test crate: clippy.toml allows these only inside #[test] functions, and the helpers and proptest closures here exist to abort a failing fixture"
)]

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use common::{open_in, raw};
use tempfile::TempDir;

fn repo_file(relative: &str) -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", relative]
        .iter()
        .collect();
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

struct ErColumn {
    name: String,
    sql_type: &'static str,
    primary: bool,
}

/// Entities of the first `erDiagram` block after the heading "### 7.2 Data model".
fn er_entities(architecture: &str) -> BTreeMap<String, Vec<ErColumn>> {
    let start = architecture
        .find("### 7.2 Data model")
        .expect("section 7.2");
    let rest = &architecture[start..];
    let block_start = rest.find("erDiagram").expect("erDiagram");
    let block = &rest[block_start..];
    let block = &block[..block.find("```").expect("end of block")];
    let mut entities = BTreeMap::new();
    let mut current: Option<(String, Vec<ErColumn>)> = None;
    for line in block.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_suffix('{') {
            current = Some((name.trim().to_lowercase(), Vec::new()));
        } else if line == "}" {
            if let Some((name, columns)) = current.take() {
                entities.insert(name, columns);
            }
        } else if let Some((_, columns)) = current.as_mut() {
            let mut words = line.split_whitespace();
            let (Some(kind), Some(name)) = (words.next(), words.next()) else {
                continue;
            };
            let before_comment = line.split('"').next().unwrap();
            let keys: Vec<&str> = before_comment.split_whitespace().skip(2).collect();
            let sql_type = match kind {
                "int" | "bool" | "datetime" => "INTEGER",
                "text" => "TEXT",
                other => panic!("unmapped ER type {other}"),
            };
            columns.push(ErColumn {
                name: name.to_owned(),
                sql_type,
                primary: keys.iter().any(|k| k.trim_end_matches(',') == "PK"),
            });
        }
    }
    entities
}

#[test]
fn migrated_schema_has_every_er_table_and_column_of_architecture_7_2() {
    let entities = er_entities(&repo_file("docs/architecture.md"));
    assert_eq!(
        entities.len(),
        14,
        "architecture 7.2 names 14 tables: {:?}",
        entities.keys()
    );
    let dir = TempDir::new().unwrap();
    drop(open_in(dir.path()));
    let conn = raw(dir.path());
    for (table, expected) in &entities {
        let mut statement = conn
            .prepare("SELECT name, type, pk FROM pragma_table_info(?1) ORDER BY cid")
            .unwrap();
        let actual: Vec<(String, String, i64)> = statement
            .query_map([table], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        let names: Vec<&str> = actual.iter().map(|c| c.0.as_str()).collect();
        let want: Vec<&str> = expected.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, want, "columns of {table}");
        for (got, want) in actual.iter().zip(expected) {
            assert_eq!(got.1, want.sql_type, "type of {table}.{}", want.name);
            assert_eq!(
                got.2 > 0,
                want.primary,
                "primary key flag of {table}.{}",
                want.name
            );
        }
    }
    // And no table exists that the architecture does not name (sqlite_ internals aside).
    let mut statement = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
        .unwrap();
    let tables: Vec<String> = statement
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    for table in tables {
        assert!(
            entities.contains_key(&table),
            "table {table} is not in architecture 7.2"
        );
    }
}

fn normalized_statements(sql: &str) -> Vec<String> {
    let without_comments: String = sql
        .lines()
        .map(|line| line.split("--").next().unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    without_comments
        .split(';')
        .map(|s| s.chars().filter(|c| !c.is_whitespace()).collect::<String>())
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn migration_0001_equals_the_ddl_of_sdd_3_5_2() {
    let sdd = repo_file("docs/design/SDD-chess-store.md");
    let start = sdd
        .find("-- migrations/0001_init.sql")
        .expect("DDL block of 3.5.2");
    let block = &sdd[start..];
    let block = &block[..block.find("```").expect("end of block")];
    let from_sdd = normalized_statements(block);
    let from_file = normalized_statements(include_str!("../migrations/0001_init.sql"));
    assert_eq!(
        from_sdd.len(),
        20,
        "statements in the SDD DDL (14 tables, 6 indexes)"
    );
    assert_eq!(from_file, from_sdd);
}
