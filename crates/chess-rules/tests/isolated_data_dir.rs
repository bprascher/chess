//! Self-test of the shared isolated data-directory helper (chess-a0r.2.2).
//! The helper lives in ci/test-support so every crate can include the same file.

#[path = "../../../ci/test-support/isolated_data_dir.rs"]
mod isolated_data_dir;

use isolated_data_dir::{ENV_VAR, IsolatedDataDir};

#[test]
fn each_instance_is_a_distinct_existing_directory_under_temp() {
    let a = IsolatedDataDir::new("a").unwrap();
    let b = IsolatedDataDir::new("a").unwrap();
    assert_ne!(a.path(), b.path());
    assert!(a.path().is_dir());
    assert!(b.path().is_dir());
    let temp = std::fs::canonicalize(std::env::temp_dir()).unwrap();
    assert!(a.path().starts_with(temp));
}

#[test]
fn directory_is_removed_on_drop() {
    let kept;
    {
        let d = IsolatedDataDir::new("drop").unwrap();
        std::fs::write(d.path().join("f"), b"x").unwrap();
        kept = d.path().to_path_buf();
        assert!(kept.is_dir());
    }
    assert!(!kept.exists());
}

#[test]
fn label_cannot_escape_the_temp_root() {
    let d = IsolatedDataDir::new("../../etc/x y").unwrap();
    let name = d.path().file_name().unwrap().to_str().unwrap().to_owned();
    assert!(name.starts_with("chess-test-"));
    assert!(!name.contains('/'));
    assert_eq!(
        d.path().parent(),
        std::fs::canonicalize(std::env::temp_dir()).ok().as_deref()
    );
}

#[test]
fn env_pair_names_the_variable_and_does_not_touch_this_process() {
    let before = std::env::var_os(ENV_VAR);
    let d = IsolatedDataDir::new("env").unwrap();
    let (name, value) = d.env();
    assert_eq!(name, "CHESS_DATA_DIR");
    assert_eq!(value, d.path());
    assert_eq!(std::env::var_os(ENV_VAR), before);
}
