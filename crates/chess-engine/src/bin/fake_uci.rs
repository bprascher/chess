//! DD-EN-012: a small UCI executable for tests. It answers like an engine that always plays the
//! smallest legal move, so the real `UciEngine`, `Spawner`, pool and timers are exercised end to
//! end. Selected through `CHESS_STOCKFISH` by the end-to-end suites. Never packaged.
//!
//! argv[1] picks a fault mode (tests only): normal, no-uciok, no-readyok, wrong-id,
//! exit-on-go, hang-go, garbage-bestmove, long-line, stderr-probe (answers `h2h3` instead of the
//! smallest legal move when its own stderr is not /dev/null, EF-10). Exits when stdin reaches end of file.

use std::io::{BufRead, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use chess_engine::PINNED_UCI_ELO_RANGE;
use chess_rules::{Fen, Move, Position};

fn out(text: &str) {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = writeln!(lock, "{text}");
    let _ = lock.flush();
}

/// True when this process's standard error is the null device (SDD 3.10.5). Safe code only:
/// `/dev/stderr` resolves to the open descriptor 2.
fn stderr_is_null() -> bool {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    match (
        std::fs::metadata("/dev/stderr"),
        std::fs::metadata("/dev/null"),
    ) {
        (Ok(e), Ok(n)) => e.file_type().is_char_device() && e.rdev() == n.rdev(),
        _ => false,
    }
}

fn legal(position_args: &str) -> Vec<String> {
    let mut tokens = position_args.split_whitespace();
    let mut pos = match tokens.next() {
        Some("startpos") => Position::standard(),
        Some("fen") => {
            let fields: Vec<&str> = tokens.by_ref().take(6).collect();
            match Fen::parse(&fields.join(" ")) {
                Ok(f) => Position::from_fen(&f),
                Err(_) => return Vec::new(),
            }
        }
        _ => return Vec::new(),
    };
    if tokens.next() == Some("moves") {
        for t in tokens {
            match Move::from_uci(t).ok().and_then(|m| pos.play(m).ok()) {
                Some(next) => pos = next,
                None => return Vec::new(),
            }
        }
    }
    let mut moves: Vec<String> = pos.legal_moves().iter().map(Move::to_uci).collect();
    moves.sort();
    moves
}

struct Thinking {
    deadline: Instant,
    moves: Vec<String>,
}

fn finish(t: &Thinking, multipv: usize) {
    for (i, m) in t.moves.iter().take(multipv).enumerate() {
        let n = i + 1;
        let cp = 50usize.saturating_sub(i * 10);
        out(&format!(
            "info depth 1 seldepth 1 multipv {n} score cp {cp} nodes 1 nps 1 time 1 pv {m}"
        ));
    }
    match t.moves.first() {
        Some(m) => out(&format!("bestmove {m}")),
        None => out("bestmove (none)"),
    }
}

fn main() {
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "normal".to_string());
    let (tx, rx) = mpsc::channel::<Option<String>>();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => {
                    if tx.send(Some(l)).is_err() {
                        return;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = tx.send(None);
    });

    let mut position = String::from("startpos");
    let mut multipv = 1usize;
    let mut thinking: Option<Thinking> = None;
    loop {
        let wait = thinking.as_ref().map_or(Duration::from_secs(3600), |t| {
            t.deadline.saturating_duration_since(Instant::now())
        });
        let line = match rx.recv_timeout(wait) {
            Ok(Some(l)) => l,
            Ok(None) | Err(mpsc::RecvTimeoutError::Disconnected) => return,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(t) = thinking.take() {
                    finish(&t, multipv);
                }
                continue;
            }
        };
        let mut words = line.split_whitespace();
        match words.next() {
            Some("uci") => {
                let name = if mode == "wrong-id" {
                    "Other Engine 1"
                } else {
                    "FakeUCI 1"
                };
                out(&format!("id name {name}"));
                out("id author the chess project");
                out("option name Threads type spin default 1 min 1 max 1024");
                out("option name Hash type spin default 16 min 1 max 33554432");
                out("option name MultiPV type spin default 1 min 1 max 500");
                out("option name Skill Level type spin default 20 min 0 max 20");
                out("option name UCI_LimitStrength type check default false");
                out(&format!(
                    "option name UCI_Elo type spin default {} min {} max {}",
                    PINNED_UCI_ELO_RANGE.min, PINNED_UCI_ELO_RANGE.min, PINNED_UCI_ELO_RANGE.max
                ));
                if mode != "no-uciok" {
                    out("uciok");
                }
            }
            Some("isready") => {
                if mode != "no-readyok" {
                    out("readyok");
                }
            }
            Some("setoption") => {
                let rest: Vec<&str> = words.collect();
                if let Some(i) = rest.iter().position(|w| *w == "MultiPV")
                    && let Some(v) = rest.get(i + 2).and_then(|v| v.parse().ok())
                {
                    multipv = v;
                }
            }
            Some("position") => position = words.collect::<Vec<&str>>().join(" "),
            Some("go") => {
                let args: Vec<&str> = words.collect();
                let ms: u64 = args
                    .iter()
                    .position(|w| *w == "movetime")
                    .and_then(|i| args.get(i + 1))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1000);
                match mode.as_str() {
                    "exit-on-go" => std::process::exit(3),
                    "hang-go" => {}
                    "stderr-probe" if !stderr_is_null() => out("bestmove h2h3"),
                    "garbage-bestmove" => out("bestmove zzzz"),
                    "long-line" => {
                        out(&"x".repeat(200_000));
                        out("bestmove e2e4");
                    }
                    _ => {
                        thinking = Some(Thinking {
                            deadline: Instant::now() + Duration::from_millis(ms),
                            moves: legal(&position),
                        });
                    }
                }
            }
            Some("stop") => {
                if let Some(t) = thinking.take() {
                    finish(&t, multipv);
                }
            }
            Some("fakecmd") => {
                if words.next() == Some("env") {
                    let names: Vec<String> = std::env::vars_os()
                        .map(|(k, _)| k.to_string_lossy().into_owned())
                        .collect();
                    out(&format!("info string env [{}]", names.join(",")));
                }
            }
            Some("quit") => return,
            _ => {}
        }
    }
}
