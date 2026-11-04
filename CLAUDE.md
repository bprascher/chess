# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is this repository?

The project is an offline macOS chess app (Rust core, ratatui terminal UI, local web UI via `chess serve`, Tauri desktop app, bundled Stockfish 19). The SDLC is following a top-down systems engineering process.  [README.md](README.md) has the feature list and technology table; [RELEASE_NOTES.md](RELEASE_NOTES.md) records what each release of documents changed.

## Document map and precedence

Read in this order when you need to resolve a question; earlier documents win over later ones:

1. [docs/requirements.md](docs/requirements.md) — requirements (`FR-`, `NFR-`, `PLAT-` …), verification cases, TBD register, decision log (`CR-n.m` entries record user decisions per amendment).
2. [docs/architecture.md](docs/architecture.md) — crate graph (§3.2), shared core `chess-app` (§4), protocol (§5), persistence/journal (§7), decisions `D-n` (§11), open items (§14). Planned repo layout is §3.1; implementation order is §13.
3. [docs/functional_decomp.md](docs/functional_decomp.md) and [docs/use_cases/](docs/use_cases/README.md) (UC-1…UC-29) — functions `F-x.y.z` allocated to architecture blocks.
4. [docs/test-strategy.md](docs/test-strategy.md) — test levels and quality gates (the definition of done).
5. [docs/design/](docs/design/) — one IEEE 1016 SDD per crate/block (rules, domain, engine, store, app, server, cli, ui, desktop-tauri), plus [SDD-consistency-review.md](docs/design/SDD-consistency-review.md) (cross-SDD mismatches `XC-n`) and [SDD-architect-triage.md](docs/design/SDD-architect-triage.md) (rulings, still pending user approval). SDDs are **0.1 DRAFT**; their "Code:" paths are planned, not real.

## Architecture in brief

- Seven Rust crates under `crates/` with a strict dependency graph (architecture §3.2): `chess-rules` and `chess-domain` are pure (no I/O); `chess-engine`, `chess-store`, `chess-app` (the shared core: services, game-session actors, protocol types) sit above; `chess-server` and `chess-cli` are entry points; `desktop/src-tauri` is the Tauri shell. All front-ends embed the core and coordinate only through the shared data directory — there is no daemon.
- One serializable protocol, three transports: in-process (CLI), Tauri IPC, localhost WebSocket. TypeScript types in `ui/src/protocol/` are **generated** by `ts-rs` — never hand-edit.
- Persistence: SQLite (WAL) plus a per-game move journal flushed before each move is acknowledged; no completed move may be lost.
- Resolved (D-38, approved 2026-10-31): `chess-app -> chess-rules` is an edge in §3.2, and front-ends depend only on `chess-app`. Amendment 7 (CR-9) records decisions 1–13 of [docs/design/SDD-architect-triage.md](docs/design/SDD-architect-triage.md); the SDDs are at 0.2 DRAFT. A Review Board pass on D-41…D-43 is required before any `chess-store` code (OQ-16).

## Process rules (docs/engineering/SWE-001-engagement-profile.md)

- **Design gates:** G0 (docs/tests/formatting) needs no approver; G1 (inside one crate, edges and §5.7 contracts unchanged) needs a system-architect desk review; G2 (touches two crates, an interface, dependency edge, data ownership, protocol or ADR) and G3 (stored formats, public commands/exit codes) need **Brian Prascher**. Never self-approve.
- **Coding standard (when code starts):** Rust 1.99.1 / edition 2024, TypeScript 7.0.2 on Node 26.13.0 LTS; compiler warnings, `clippy`, `rustfmt --check`, strict `tsc`, ESLint treated as errors; suppressions need a written reason; `cargo-deny`/`npm audit` CVEs block release. SEI CERT is applied by *intent* only — make no conformance claim. The local toolchain (cargo 1.89, node 26.12.1) is older than the baseline and must be updated before step 1.
- **Evidence tags** in engineering docs: `[OBS]` observed, `[RAN]` executed, `[INF]` inferred, `[UNV]` unverified. Do not state unverified compiler/crate behaviour as fact.
- IDs follow `PREFIX-n` (e.g. `D-`, `FR-`, `UF-APP-nn`, `GATE-101+`); don't reuse or renumber existing IDs — amendments add, they don't renumber. Changes to baselined documents are recorded as amendments with a Decision Log entry and a RELEASE_NOTES entry.
- Intel Macs can't be tested; platform verification without hardware is "not run", best effort.

## Diagrams

Docs use Mermaid. `.mermaid/puppeteer.json` (untracked) configures headless Firefox for `mmdc` rendering/validation; validate diagrams before committing.

## Git

Work is merged to `main` from topic branches with descriptive imperative commit messages (e.g. "Add draft SDDs for all software blocks…").

## Tone and Style Guidance

- **Clarity.** Write in a straightforward and easy to understand manner using simple, direct language avoiding unnecessary words and jargon 
- **Accuracy.** Only report information that is correct and reliable; do not report facts without checking them first
- **Organization.** Structure content in a coherent manner with a clear logical flow
- **Details.** Provide sufficient detail to ensure the reader can achieve their goals


<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:46cd31e7 -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

**Architecture in one line:** issues live in a local Dolt DB; sync uses `refs/dolt/data` on your git remote; `.beads/issues.jsonl` is a passive export. See https://github.com/gastownhall/beads/blob/main/docs/core-concepts/sync-concepts.md for details and anti-patterns.

## Agent Context Profiles

The managed Beads block is task-tracking guidance, not permission to override repository, user, or orchestrator instructions.

- **Conservative (default)**: Use `bd` for task tracking. Do not run git commits, git pushes, or Dolt remote sync unless explicitly asked. At handoff, report changed files, validation, and suggested next commands.
- **Minimal**: Keep tool instruction files as pointers to `bd prime`; use the same conservative git policy unless active instructions say otherwise.
- **Team-maintainer**: Only when the repository explicitly opts in, agents may close beads, run quality gates, commit, and push as part of session close. A current "do not commit" or "do not push" instruction still wins.

## Session Completion

This protocol applies when ending a Beads implementation workflow. It is subordinate to explicit user, repository, and orchestrator instructions.

1. **File issues for remaining work** - Create beads for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **Handle git/sync by active profile**:
   ```bash
   # Conservative/minimal/default: report status and proposed commands; wait for approval.
   git status

   # Team-maintainer opt-in only, unless current instructions forbid it:
   git pull --rebase
   bd dolt push
   git push
   git status
   ```
5. **Hand off** - Summarize changes, validation, issue status, and any blocked sync/commit/push step

**Critical rules:**
- Explicit user or orchestrator instructions override this Beads block.
- Do not commit or push without clear authority from the active profile or the current user request.
- If a required sync or push is blocked, stop and report the exact command and error.
<!-- END BEADS INTEGRATION -->
