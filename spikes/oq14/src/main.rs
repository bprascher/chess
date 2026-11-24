//! OQ-14 spike: what does Rust std give for the chess-store file primitives on macOS?
//! No `unsafe` here (forbidden by the package lint): everything below is safe std.
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

type R = io::Result<()>;

fn open_rw(p: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(p)
}

/// Subcommand `hold <path>`: take the exclusive lock, say so, block until killed.
fn hold(p: &Path) -> R {
    let f = open_rw(p)?;
    f.lock()?;
    println!("LOCKED");
    io::stdout().flush()?;
    loop {
        std::thread::sleep(Duration::from_secs(3600));
        let _keep = &f;
    }
}

/// Subcommand `try <path>`: report the try_lock outcome as a word.
fn try_it(p: &Path) -> R {
    let f = open_rw(p)?;
    match f.try_lock() {
        Ok(()) => println!("ACQUIRED"),
        Err(TryLockError::WouldBlock) => println!("WOULDBLOCK"),
        Err(TryLockError::Error(e)) => println!("ERROR {e} raw={:?}", e.raw_os_error()),
    }
    Ok(())
}

fn child_try(exe: &Path, p: &Path) -> io::Result<String> {
    let out = Command::new(exe).arg("try").arg(p).output()?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn lock_tests(dir: &Path, exe: &Path) -> R {
    let p = dir.join("lock.jnl");
    // L1: same process, second open file description conflicts (flock is per open file description).
    let a = open_rw(&p)?;
    let b = open_rw(&p)?;
    a.try_lock()?;
    let l1 = matches!(b.try_lock(), Err(TryLockError::WouldBlock));
    println!("L1 second open in same process -> WouldBlock: {l1}");
    // L2: another process sees contention.
    println!("L2 other process while held -> {}", child_try(exe, &p)?);
    // L3: unlock, then the other process acquires.
    a.unlock()?;
    println!("L3 other process after unlock -> {}", child_try(exe, &p)?);
    // L4: shared locks coexist; exclusive is refused while shared is held.
    a.lock_shared()?;
    let l4 = b.try_lock_shared().is_ok();
    println!("L4 two shared locks coexist: {l4}");
    drop(b);
    a.unlock()?;
    // L5: release on process death (SIGKILL).
    let mut holder = Command::new(exe)
        .arg("hold")
        .arg(&p)
        .stdout(Stdio::piped())
        .spawn()?;
    let mut line = [0u8; 6];
    if let Some(out) = holder.stdout.as_mut() {
        out.read_exact(&mut line)?;
    }
    println!("L5 holder says {:?}", String::from_utf8_lossy(&line).trim());
    println!(
        "L5 other process while holder alive -> {}",
        child_try(exe, &p)?
    );
    holder.kill()?; // SIGKILL
    holder.wait()?;
    println!(
        "L5 other process after SIGKILL of holder -> {}",
        child_try(exe, &p)?
    );
    // L6: lock survives rename of the path (lock belongs to the open file description).
    let c = open_rw(&p)?;
    c.try_lock()?;
    let moved = dir.join("lock.moved");
    fs::rename(&p, &moved)?;
    println!(
        "L6 locked file renamed; other process on new path -> {}",
        child_try(exe, &moved)?
    );
    Ok(())
}

fn full_sync_tests(dir: &Path) -> R {
    let p = dir.join("sync.bin");
    let mut f = open_rw(&p)?;
    f.write_all(&[7u8; 4096])?;
    // F1: sync_all succeeds on an APFS file.
    println!("F1 sync_all on APFS file: {:?}", f.sync_all());
    // F2: timing of sync_all (ms-scale suggests the drive cache flush of F_FULLFSYNC).
    let n = 50;
    let t = Instant::now();
    for _ in 0..n {
        f.write_all(&[1u8; 64])?;
        f.sync_all()?;
    }
    println!("F2 sync_all mean over {n}: {:?}", t.elapsed() / n);
    let t = Instant::now();
    for _ in 0..n {
        f.write_all(&[1u8; 64])?;
        f.sync_data()?;
    }
    println!("F2 sync_data mean over {n}: {:?}", t.elapsed() / n);
    // T1: ftruncate (set_len) then full sync; reopen and verify.
    let acked = f.metadata()?.len();
    f.write_all(&[9u8; 100])?; // an "ambiguous append"
    f.set_len(acked)?;
    f.sync_all()?;
    drop(f);
    let len = fs::metadata(&p)?.len();
    println!(
        "T1 acked_len={acked} after truncate+sync_all len={len} equal={}",
        len == acked
    );
    // T2: append position after set_len needs an explicit seek if the cursor is past the end.
    let mut g = open_rw(&p)?;
    g.seek(SeekFrom::Start(acked))?;
    g.write_all(b"next")?;
    g.sync_all()?;
    println!("T2 rewrite at acked_len ok, len={}", g.metadata()?.len());
    Ok(())
}

fn dir_and_rename_tests(dir: &Path) -> R {
    // D1: directory fsync via File::open(dir).sync_all().
    let d = File::open(dir)?;
    println!("D1 directory sync_all: {:?}", d.sync_all());
    // N1: std::fs::rename replaces silently (the thing D-83 must not do).
    let a = dir.join("a.jnl");
    let b = dir.join("b.jnl");
    fs::write(&a, b"A")?;
    fs::write(&b, b"B")?;
    fs::rename(&a, &b)?;
    println!(
        "N1 fs::rename over existing replaced silently: {}",
        fs::read(&b)? == b"A"
    );
    // N2: no-replace rename from std only: hard_link then remove_file.
    let src = dir.join("src.rebase");
    let dst = dir.join("dst.jnl");
    fs::write(&src, b"S")?;
    fs::write(&dst, b"EXISTING")?;
    let r = fs::hard_link(&src, &dst);
    println!(
        "N2 hard_link onto existing dst -> {:?}",
        r.as_ref().map_err(|e| (e.kind(), e.raw_os_error()))
    );
    println!("N2 dst untouched: {}", fs::read(&dst)? == b"EXISTING");
    fs::remove_file(&dst)?;
    fs::hard_link(&src, &dst)?;
    println!(
        "N2 both names exist between link and unlink: {}",
        src.exists() && dst.exists()
    );
    fs::remove_file(&src)?;
    println!(
        "N2 after unlink: src gone={}, dst content={:?}",
        !src.exists(),
        fs::read(&dst)?
    );
    Ok(())
}

fn crc_test() {
    let v = crc32fast::hash(b"123456789");
    println!(
        "C1 crc32fast::hash(\"123456789\") = {v:#010X} expected 0xCBF43926 ok={}",
        v == 0xCBF4_3926
    );
    let mut h = crc32fast::Hasher::new();
    h.update(b"1234");
    h.update(b"56789");
    println!("C1 incremental = {:#010X}", h.finalize());
    println!(
        "C1 empty = {:#010X} (ISO-HDLC: 0x00000000)",
        crc32fast::hash(b"")
    );
}

fn probe(p: &Path) -> R {
    // Probe for D-71: sync_all on a file in a directory; report errno on failure.
    let f = open_rw(&p.join("probe.id"))?;
    match f.sync_all() {
        Ok(()) => println!("PROBE ok"),
        Err(e) => println!(
            "PROBE err kind={:?} raw={:?} msg={e}",
            e.kind(),
            e.raw_os_error()
        ),
    }
    Ok(())
}

/// Subcommand `syncpath <file>`: sync_all on an existing path; report errno (shows std propagates it).
fn syncpath(p: &Path) -> R {
    let f = File::open(p)?;
    match f.sync_all() {
        Ok(()) => println!("SYNCPATH ok"),
        Err(e) => println!(
            "SYNCPATH err kind={:?} raw={:?} msg={e}",
            e.kind(),
            e.raw_os_error()
        ),
    }
    Ok(())
}

/// Subcommand `excl <old> <new>`: rustix safe wrapper for renameatx_np(RENAME_EXCL).
fn excl(old: &Path, new: &Path) -> R {
    use rustix::fs::{CWD, RenameFlags, renameat_with};
    match renameat_with(CWD, old, CWD, new, RenameFlags::NOREPLACE) {
        Ok(()) => println!("EXCL ok"),
        Err(e) => println!("EXCL err {e} raw={}", e.raw_os_error()),
    }
    Ok(())
}

fn main() -> R {
    let args: Vec<String> = std::env::args().collect();
    let exe = std::env::current_exe()?;
    match args.get(1).map(String::as_str) {
        Some("hold") => return hold(Path::new(&args[2])),
        Some("try") => return try_it(Path::new(&args[2])),
        Some("probe") => return probe(Path::new(&args[2])),
        Some("excl") => return excl(Path::new(&args[2]), Path::new(&args[3])),
        Some("syncpath") => return syncpath(Path::new(&args[2])),
        _ => {}
    }
    let dir: PathBuf = args
        .get(1)
        .map_or_else(|| PathBuf::from("./oq14-work"), PathBuf::from);
    fs::create_dir_all(&dir)?;
    lock_tests(&dir, &exe)?;
    full_sync_tests(&dir)?;
    dir_and_rename_tests(&dir)?;
    crc_test();
    Ok(())
}
