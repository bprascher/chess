//! Generates the ECO table (DD-RUL-013, D-20, D-75) from the checked-in snapshot in
//! `third_party/eco/`. Reads only checked-in files; opens no network connection (PLAT-3). A row
//! that does not replay fails the build with file and line.

#[path = "build/eco_gen.rs"]
mod eco_gen;
#[path = "src/key_derive.rs"]
mod key_derive;

use std::path::PathBuf;
use std::process::ExitCode;

fn run() -> Result<(), String> {
    let manifest =
        std::env::var("CARGO_MANIFEST_DIR").map_err(|e| format!("CARGO_MANIFEST_DIR: {e}"))?;
    let out_dir = std::env::var("OUT_DIR").map_err(|e| format!("OUT_DIR: {e}"))?;
    let data = PathBuf::from(manifest).join("../../third_party/eco");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build/eco_gen.rs");
    println!("cargo:rerun-if-changed=src/key_derive.rs");
    println!("cargo:rerun-if-changed={}", data.display());
    let generated = eco_gen::generate(&data)?;
    println!(
        "cargo:warning=ECO table: {} rows, {} positions, {} duplicate positions",
        generated.rows,
        generated.entries.len(),
        generated.duplicates
    );
    std::fs::write(
        PathBuf::from(out_dir).join("eco_generated.rs"),
        generated.source,
    )
    .map_err(|e| format!("cannot write eco_generated.rs: {e}"))
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("ECO table generation failed: {message}");
            ExitCode::FAILURE
        }
    }
}
