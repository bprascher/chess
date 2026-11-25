//! DD-ST-024 `FaultPlan`: named failpoints for failure-injection and kill tests.
//!
//! **Feature off (the default):** this module is private. Its only content is `check` and the
//! two `sync_*` helpers, which are `#[inline(always)]` and reduce to the plain `sync_all` call;
//! `FaultPlan` and everything else below does not exist, so the call sites in the store compile
//! to nothing extra (acceptance (1); a compile-fail doctest in `lib.rs` pins the absence).
//!
//! **Feature `test-support`:** the module is public. A test builds a `FaultPlan`, installs it
//! for one data directory (`FaultPlan::install`) and gets a `FaultGuard`; dropping the guard
//! removes the plan. A plan is scoped by directory: a failpoint call carries a path, and only
//! a plan whose root is a prefix of that path applies, so tests in one process that use their
//! own temporary directories cannot disturb each other. The registry is a process-wide
//! `Mutex`, so the store thread, the `spawn_blocking` threads and the test thread all see it.
//!
//! Actions: `Error` (an `io::Error` from an errno or an `ErrorKind`), `Delay`, and `Kill`
//! (see `KILL_MARKER`: the process prints the marker, then waits for its parent to `SIGKILL`
//! it, so a kill leaves the directory exactly as a crash at that point would).
//! A failpoint name that is not in the registry (`all()`) is refused when the plan is installed,
//! so a typo cannot make a test pass by never firing.

#[cfg(feature = "test-support")]
use std::collections::BTreeMap;
use std::fs::File;
use std::io;
use std::path::Path;
#[cfg(feature = "test-support")]
use std::sync::{Arc, Mutex, MutexGuard};
#[cfg(feature = "test-support")]
use std::time::Duration;

/// The failpoint names of the SDD section 4 list (DD-ST-024). Call sites exist for the ones
/// whose part is built; the others are armable and reachable through `trigger` and are wired
/// by the part that builds the code around them.
#[cfg(feature = "test-support")]
pub const SDD_FAILPOINTS: &[&str] = &[
    "journal.before_write",
    "journal.after_write",
    "journal.before_fsync",
    "journal.after_fsync",
    "apply.before_commit",
    "apply.after_commit",
    "finalize.after_journal_end",
    "finalize.before_commit",
    "finalize.after_commit_before_rename",
    "reset.after_commit_before_voided_record",
    "backup.after_copy",
    "restore.after_replay",
    "restore.before_rename",
];

/// The three names added by Addendum K (D-108, D-110).
#[cfg(feature = "test-support")]
pub const ADDENDUM_K_FAILPOINTS: &[&str] = &[
    "backup.before_rename",
    "journal.rebase.old_linked",
    "journal.rebase.new_linked",
];

/// Names that SDD test rows CT-ST-087 and CT-ST-088 use (D-82, D-83), registered here so those
/// tests can arm them when part B builds the code.
#[cfg(feature = "test-support")]
pub const CT_NAMED_FAILPOINTS: &[&str] = &[
    "journal.partial_write",
    "journal.fsync_error",
    "journal.rebase.built",
    "journal.rebase.db_committed",
    "journal.rebase.old_aside",
    "journal.rebase.new_in_place",
    // SDD 3.7.3 rule (6): the cleanup truncate of a failed append (D-82). Not in the section 4
    // list; registered by part B.
    "journal.truncate_error",
    // D4 (B-R03, D-153): between `hard_link` and `remove_file` of `close_final`; the journal
    // exists under two names.
    "journal.close_final.linked",
    // D4 review fix (FIND-D4R-01): a seam, not a fault. It sits in the file-only prologue of a
    // resume or a startup recovery at the moment the prologue has decided that the game has no
    // journal file and no `.rebase`. A `Delay` there widens the window between that decision and
    // the rebase, which is what a busy store thread does in real life.
    "journal.resume.no_file_seen",
    // D4 review fix (FIND-D4R-09, B-R04): a seam in `MoveJournal::create` between the creation of
    // the file and its lock, where a prober can take the lock of the new name. A `Delay` there
    // makes the race of B-R04 certain instead of chance.
    "journal.create.created",
];

/// Names added by part D1 for the part A code (not in the SDD list; reported as an SDD gap).
/// The `sync.*` names stand for one full-sync call each, so a test can make that call fail and
/// thereby prove the call is there. `backup.verify` makes the verification of a backup copy
/// fail. The `open.*` and `migrate.*` names are kill points of the first launch and the
/// upgrade.
#[cfg(feature = "test-support")]
pub const D1_FAILPOINTS: &[&str] = &[
    "sync.probe",
    "sync.backup_copy_file",
    "sync.backup_copy_dir",
    "sync.backup_rename_dir",
    "sync.store_id_file",
    "sync.store_id_dir",
    "backup.verify",
    "migrate.after_backup",
    "open.after_db_created",
    "open.after_store_id",
    // Rework 2026-11-22 (FIND-B-06 narrowed, store-rework-specs 1.2): the copy of the database
    // and its hot journal made to decide whether the database holds data. An error here means
    // "cannot tell", and the open keeps its `Unavailable{ReadOnly}`.
    "open.nf2_copy",
    // Part B: the full syncs of the journal's create, move and rebase steps.
    "sync.journal_create_file",
    "sync.journal_create_dir",
    "sync.journal_move_dir",
    "sync.journal_rebase_file",
    // Part D2: the point before the commit of each lifecycle command (CT-ST-007). The SDD list
    // has `finalize.before_commit` only; these three are reported as an SDD gap.
    "abort.before_commit",
    "void.before_commit",
    "reset.before_commit",
];

/// Part D5: the point before the commit of each statistics-side write command (CT-ST-007):
/// `eval.before_commit` (`upsert_move_eval`), `variation.before_commit` (`save_variation`,
/// `delete_variation`), `job.before_commit` (the job queue commands). Not in the SDD list;
/// reported as an SDD gap (FIND-D5-05).
#[cfg(feature = "test-support")]
pub const D5_FAILPOINTS: &[&str] = &[
    "eval.before_commit",
    "variation.before_commit",
    "job.before_commit",
];

/// Every registered failpoint name.
#[cfg(feature = "test-support")]
pub fn all() -> impl Iterator<Item = &'static str> {
    SDD_FAILPOINTS
        .iter()
        .chain(ADDENDUM_K_FAILPOINTS)
        .chain(CT_NAMED_FAILPOINTS)
        .chain(D1_FAILPOINTS)
        .chain(D5_FAILPOINTS)
        .copied()
}

/// The line a process prints to standard error when a `Kill` action fires, followed by the
/// failpoint name. The harness of a parent test reads it to learn where the child stopped.
#[cfg(feature = "test-support")]
pub const KILL_MARKER: &str = "FAILPOINT-KILL";

/// How long a process that hit a `Kill` failpoint waits for its parent before it aborts by
/// itself (so a crashed parent leaves no orphan).
#[cfg(feature = "test-support")]
const KILL_WAIT: Duration = Duration::from_secs(120);

/// What a failpoint does when it fires.
#[cfg(feature = "test-support")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Return `io::Error::from_raw_os_error(errno)`.
    Errno(i32),
    /// Return `io::Error::from(kind)`.
    Kind(io::ErrorKind),
    /// Sleep, then continue.
    Delay(Duration),
    /// Print `KILL_MARKER` and the name, then wait to be killed (see the module doc).
    Kill,
}

/// A name that is not a registered failpoint.
#[cfg(feature = "test-support")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownFailpoint(pub String);

#[cfg(feature = "test-support")]
impl std::fmt::Display for UnknownFailpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown failpoint {:?}", self.0)
    }
}

#[cfg(feature = "test-support")]
impl std::error::Error for UnknownFailpoint {}

/// Why `FaultPlan::install` refused. Each case is a plan that would otherwise **silently never
/// fire** (FIND-D1R-02), so a failure test that passes because nothing happened cannot be written
/// by accident.
#[cfg(feature = "test-support")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallError {
    /// A rule names a failpoint that is not registered.
    Unknown(UnknownFailpoint),
    /// The root has a `..` after a component that does not exist, so it cannot be resolved to
    /// the form the failpoint contexts use and no call would ever match it.
    UnresolvableRoot(std::path::PathBuf),
    /// A plan is already installed for this (canonical) root; the second would hide the first
    /// and the first guard's counters would stay at zero.
    DuplicateRoot(std::path::PathBuf),
}

#[cfg(feature = "test-support")]
impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallError::Unknown(unknown) => unknown.fmt(f),
            InstallError::UnresolvableRoot(root) => write!(
                f,
                "plan root {root:?} has `..` after a path component that does not exist"
            ),
            InstallError::DuplicateRoot(root) => {
                write!(f, "a plan is already installed for root {root:?}")
            }
        }
    }
}

#[cfg(feature = "test-support")]
impl std::error::Error for InstallError {}

/// True when `path` has a `..` that follows a component that does not exist (`real/nope/../sub`):
/// `canonical` cannot resolve such a path.
#[cfg(feature = "test-support")]
fn has_parent_after_missing(path: &Path) -> bool {
    let mut walked = std::path::PathBuf::new();
    for component in path.components() {
        if component == std::path::Component::ParentDir && !walked.exists() {
            return true;
        }
        walked.push(component);
    }
    false
}

#[cfg(feature = "test-support")]
#[derive(Debug, Clone)]
struct Rule {
    action: Action,
    /// Hits that pass before the rule fires.
    skip: u32,
    /// Fire once, then remove the rule.
    once: bool,
}

#[cfg(feature = "test-support")]
#[derive(Debug, Default)]
struct Counters {
    hits: BTreeMap<&'static str, u32>,
    fired: BTreeMap<&'static str, u32>,
}

#[cfg(feature = "test-support")]
struct Installed {
    root: std::path::PathBuf,
    rules: Mutex<BTreeMap<&'static str, Rule>>,
    counters: Mutex<Counters>,
}

#[cfg(feature = "test-support")]
static PLANS: Mutex<Vec<Arc<Installed>>> = Mutex::new(Vec::new());

/// A poisoned lock only means another test panicked; the data is still usable.
#[cfg(feature = "test-support")]
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The canonical form of `path`, also for a path that does not exist yet: the longest existing
/// ancestor is canonicalized (symlinks such as macOS `/var` -> `/private/var` resolved) and the
/// rest is appended. If nothing can be resolved the path is returned as given. FIND-D5-06: a
/// plan root and a failpoint context are compared in this form, so a symlink cannot turn a
/// failpoint into a silent no-op.
#[cfg(feature = "test-support")]
fn canonical(path: &Path) -> std::path::PathBuf {
    let mut rest = Vec::new();
    let mut head = path;
    loop {
        if let Ok(resolved) = head.canonicalize() {
            return rest.iter().rev().fold(resolved, |acc, part| acc.join(part));
        }
        match (head.parent(), head.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name);
                head = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// A set of rules, built by the test and installed for one directory.
#[cfg(feature = "test-support")]
#[derive(Debug, Default, Clone)]
pub struct FaultPlan {
    rules: Vec<(String, Rule)>,
}

#[cfg(feature = "test-support")]
impl FaultPlan {
    pub fn new() -> FaultPlan {
        FaultPlan::default()
    }

    fn with(mut self, name: &str, action: Action) -> FaultPlan {
        self.rules.push((
            name.to_owned(),
            Rule {
                action,
                skip: 0,
                once: false,
            },
        ));
        self
    }

    /// Fail the failpoint with an OS error number.
    #[must_use]
    pub fn errno(self, name: &str, errno: i32) -> FaultPlan {
        self.with(name, Action::Errno(errno))
    }

    /// Fail the failpoint with an `ErrorKind`.
    #[must_use]
    pub fn fail(self, name: &str, kind: io::ErrorKind) -> FaultPlan {
        self.with(name, Action::Kind(kind))
    }

    /// Sleep at the failpoint.
    #[must_use]
    pub fn delay(self, name: &str, by: Duration) -> FaultPlan {
        self.with(name, Action::Delay(by))
    }

    /// Kill the process at the failpoint (only for a child process; see the module doc).
    #[must_use]
    pub fn kill(self, name: &str) -> FaultPlan {
        self.with(name, Action::Kill)
    }

    /// The last rule passes its first `n` hits before it fires.
    #[must_use]
    pub fn skip(mut self, n: u32) -> FaultPlan {
        if let Some((_, rule)) = self.rules.last_mut() {
            rule.skip = n;
        }
        self
    }

    /// The last rule fires once and is then removed.
    #[must_use]
    pub fn once(mut self) -> FaultPlan {
        if let Some((_, rule)) = self.rules.last_mut() {
            rule.once = true;
        }
        self
    }

    /// Parses `name=action[,name=action...]` where `action` is `kill`, `errno:<n>`,
    /// `delay:<ms>`, optionally followed by `@<skip>` (the hits to pass first) and then `!` (fire once). This is how a
    /// test passes a plan to a child process through its environment.
    pub fn parse(spec: &str) -> Result<FaultPlan, String> {
        let mut plan = FaultPlan::new();
        for item in spec.split(',').filter(|s| !s.is_empty()) {
            let (name, action) = item
                .split_once('=')
                .ok_or_else(|| format!("no '=' in {item:?}"))?;
            // A trailing `!` makes the rule fire once (part B: a failed append that is retried).
            let (action, once) = match action.strip_suffix('!') {
                Some(rest) => (rest, true),
                None => (action, false),
            };
            let (action, skip) = match action.split_once('@') {
                Some((a, n)) => (a, n.parse::<u32>().map_err(|e| e.to_string())?),
                None => (action, 0),
            };
            plan = match action.split_once(':') {
                None if action == "kill" => plan.kill(name),
                Some(("errno", n)) => {
                    plan.errno(name, n.parse::<i32>().map_err(|e| e.to_string())?)
                }
                Some(("delay", ms)) => plan.delay(
                    name,
                    Duration::from_millis(ms.parse::<u64>().map_err(|e| e.to_string())?),
                ),
                _ => return Err(format!("unknown action {action:?}")),
            }
            .skip(skip);
            if once {
                plan = plan.once();
            }
        }
        Ok(plan)
    }

    /// Installs the plan for every failpoint call whose path starts with `root`. Refuses a
    /// name that is not registered, a root with `..` after a missing component and a root that
    /// already has a plan (`InstallError`). The plan stays active until the guard is dropped.
    pub fn install(self, root: &Path) -> Result<FaultGuard, InstallError> {
        let mut rules = BTreeMap::new();
        for (name, rule) in self.rules {
            let known = all()
                .find(|n| *n == name)
                .ok_or_else(|| InstallError::Unknown(UnknownFailpoint(name.clone())))?;
            rules.insert(known, rule);
        }
        if has_parent_after_missing(root) {
            return Err(InstallError::UnresolvableRoot(root.to_path_buf()));
        }
        let installed = Arc::new(Installed {
            root: canonical(root),
            rules: Mutex::new(rules),
            counters: Mutex::new(Counters::default()),
        });
        let mut plans = lock(&PLANS);
        if plans.iter().any(|plan| plan.root == installed.root) {
            return Err(InstallError::DuplicateRoot(installed.root.clone()));
        }
        plans.push(Arc::clone(&installed));
        Ok(FaultGuard { installed })
    }
}

/// Keeps a plan installed; removes it on drop.
#[cfg(feature = "test-support")]
#[derive(Debug)]
pub struct FaultGuard {
    installed: Arc<Installed>,
}

#[cfg(feature = "test-support")]
impl std::fmt::Debug for Installed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Installed")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "test-support")]
impl FaultGuard {
    /// How often the failpoint was reached under this plan's directory (fired or not).
    pub fn hits(&self, name: &str) -> u32 {
        lock(&self.installed.counters)
            .hits
            .get(name)
            .copied()
            .unwrap_or(0)
    }

    /// How often the failpoint's action fired.
    pub fn fired(&self, name: &str) -> u32 {
        lock(&self.installed.counters)
            .fired
            .get(name)
            .copied()
            .unwrap_or(0)
    }
}

#[cfg(feature = "test-support")]
impl Drop for FaultGuard {
    fn drop(&mut self) {
        lock(&PLANS).retain(|p| !Arc::ptr_eq(p, &self.installed));
    }
}

/// Reaches the failpoint `name` for the path `ctx`. With no plan installed for `ctx`, it is a
/// no-op. A fired `Errno` or `Kind` action returns the error; `Delay` sleeps and returns `Ok`;
/// `Kill` does not return in a normal run (see the module doc). A name that is not registered
/// is `InvalidInput`, so a store call site with a typo fails its first test.
#[cfg(feature = "test-support")]
pub fn trigger(name: &str, ctx: &Path) -> io::Result<()> {
    let Some(known) = all().find(|n| *n == name) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unknown failpoint",
        ));
    };
    let plan = {
        let plans = lock(&PLANS);
        if plans.is_empty() {
            return Ok(());
        }
        let ctx = canonical(ctx);
        // The longest matching root wins, so a nested directory can override.
        plans
            .iter()
            .filter(|p| ctx.starts_with(&p.root))
            .max_by_key(|p| p.root.components().count())
            .cloned()
    };
    let Some(plan) = plan else { return Ok(()) };
    *lock(&plan.counters).hits.entry(known).or_insert(0) += 1;
    let action = {
        let mut rules = lock(&plan.rules);
        match rules.get_mut(known) {
            None => None,
            Some(rule) if rule.skip > 0 => {
                rule.skip -= 1;
                None
            }
            Some(rule) => {
                let action = rule.action.clone();
                if rule.once {
                    rules.remove(known);
                }
                Some(action)
            }
        }
    };
    let Some(action) = action else { return Ok(()) };
    *lock(&plan.counters).fired.entry(known).or_insert(0) += 1;
    match action {
        Action::Errno(errno) => Err(io::Error::from_raw_os_error(errno)),
        Action::Kind(kind) => Err(io::Error::from(kind)),
        Action::Delay(by) => {
            std::thread::sleep(by);
            Ok(())
        }
        Action::Kill => {
            use std::io::Write;
            let _ = writeln!(io::stderr(), "{KILL_MARKER} {known}");
            let _ = io::stderr().flush();
            std::thread::sleep(KILL_WAIT);
            std::process::abort()
        }
    }
}

/// The store's internal failpoint call (feature on).
#[cfg(feature = "test-support")]
#[inline]
pub(crate) fn check(name: &'static str, ctx: &Path) -> io::Result<()> {
    trigger(name, ctx)
}

/// The store's internal failpoint call (feature off): nothing.
#[cfg(not(feature = "test-support"))]
#[inline(always)]
#[allow(
    clippy::unnecessary_wraps,
    reason = "same signature as the test-support build, so call sites are identical in both"
)]
pub(crate) fn check(_name: &'static str, _ctx: &Path) -> io::Result<()> {
    Ok(())
}

/// A full sync of `file` that a test can make fail (`ctx` is a path in the data directory).
#[inline]
pub(crate) fn sync_file(name: &'static str, ctx: &Path, file: &File) -> io::Result<()> {
    check(name, ctx)?;
    file.sync_all()
}

/// A full sync of the directory `dir` that a test can make fail.
#[inline]
pub(crate) fn sync_dir_named(name: &'static str, dir: &Path) -> io::Result<()> {
    check(name, dir)?;
    File::open(dir)?.sync_all()
}
