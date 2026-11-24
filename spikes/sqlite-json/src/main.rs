//! S1: does the bundled SQLite have json_valid, and does CHECK(json_valid(x)) reject bad JSON?
use rusqlite::{Connection, Result};

fn main() -> Result<()> {
    let db = Connection::open_in_memory()?;
    let v: String = db.query_row("SELECT sqlite_version()", [], |r| r.get(0))?;
    println!("sqlite_version = {v}");
    println!("rusqlite bundled lib version number = {}", rusqlite::version_number());

    for (label, sql) in [
        ("json_valid('{bad')", "SELECT json_valid('{bad')"),
        ("json_valid('{\"a\":1}')", "SELECT json_valid('{\"a\":1}')"),
        ("json_valid('')", "SELECT json_valid('')"),
        ("json_valid(NULL)", "SELECT json_valid(NULL)"),
    ] {
        let r: Option<i64> = db.query_row(sql, [], |r| r.get(0))?;
        println!("{label} = {r:?}");
    }

    db.execute_batch("CREATE TABLE t (x TEXT CHECK (json_valid(x)));")?;
    let good = db.execute("INSERT INTO t (x) VALUES (?1)", ["{\"a\":1}"]);
    println!("insert valid json   -> {good:?}");
    let bad = db.execute("INSERT INTO t (x) VALUES (?1)", ["{bad"]);
    println!("insert '{{bad'       -> {bad:?}");
    let null = db.execute("INSERT INTO t (x) VALUES (NULL)", []);
    println!("insert NULL         -> {null:?}  (CHECK passes when the expression is NULL)");
    let n: i64 = db.query_row("SELECT count(*) FROM t", [], |r| r.get(0))?;
    println!("rows in t = {n}");

    let mut stmt = db.prepare("PRAGMA compile_options")?;
    let opts: Vec<String> = stmt.query_map([], |r| r.get(0))?.collect::<Result<_>>()?;
    for o in opts.iter().filter(|o| o.contains("JSON") || o.contains("FOREIGN") || o.contains("LOAD_EXT")) {
        println!("compile_option {o}");
    }
    Ok(())
}
