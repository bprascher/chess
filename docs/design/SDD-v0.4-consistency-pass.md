# SDD v0.4 consistency pass (nine SDDs against the final architecture)

| Field | Value |
|---|---|
| Document | `docs/design/SDD-v0.4-consistency-pass.md` |
| Version | **0.1 DRAFT** |
| Date | 2026-11-01 |
| Stance | Reviewer (`swe-governance`). Findings only. No SDD, architecture or requirements file was edited |
| Gate | **G0** (a review document). The fixes carry their own tier (column "Gate"). No `GATE-` ID is allocated |
| Approver | **PENDING** |
| Reviewed | The nine SDDs (`rules`, `domain`, `store`, `engine`, `app`, `server`, `cli`, `ui`, `desktop-tauri`) against `architecture.md` as it stands after **amendment 7, addendum B** (D-60, D-61, OQ-23, section 5.8 refined) and `requirements.md` CR-9.14 |
| Baseline | [SDD-v0.3-joint-review](SDD-v0.3-joint-review.md) (FIND-JR3-001 to 014) |
| Evidence tags | `[OBS file:line]` read this session. `[RAN]` a grep or a small Python scan was executed. `[INF]` inferred. Nothing was compiled; no code exists |

## 1 Method and limits

- `[RAN]` grep over the nine SDDs for the terms addendum B changes (`tag_name`, `moves_san`, `import_id`, `on_duplicate`, `keep-pgn`, `eco_for_import`, `D-60`, `D-61`, `OQ-23`), then targeted reads of the hit regions.
- `[RAN]` Python scan: every `D-n`, `OQ-n` and `FF-n` cited in an SDD is defined in `architecture.md`; cross-SDD test-ID references checked by hand.
- Read side by side: architecture 7.2 ER model against the store DDL; architecture 3.2 edges against the "edges" paragraph of each SDD; heartbeat and lease values.
- Not read end to end: the SDDs (12,000 lines), `test-strategy.md`, use cases. A defect outside the hit regions can be missed.
- Not checked: Mermaid validity, `ts-rs` export list against UI guards, any `ESTIMATE` number beyond agreement.

## 2 Result in one line

Nine SDDs are consistent with the final architecture on stored data, crate edges, journal, heartbeat values and every decision except two: **one Major finding** (the PGN import shape that addendum B leaves open as OQ-23) and **four Minor** text findings. The SDDs predate addendum B in their wording: none cites D-60 or D-61.

## 3 Findings

| ID | Sev | Finding | Evidence | Owner | Gate |
|---|---|---|---|---|---|
| FIND-JR4-001 | Major | **PGN import: the SDDs implement a two-pass flow, the architecture describes one reply.** Architecture 5.8 says one `ImportConflicts{import_id, list}` reply "may also hold `duplicate_game` entries under the same `import_id`" and that `ResolveImport` "carries `on_duplicate` as well"; the shape is `ResolveImport{import_id, on_duplicate?, eco}`. The architect flagged this as an addition and opened **OQ-23** ("the app, CLI and UI texts must confirm it"). The SDDs do the opposite: the app answers duplicates first with `ImportConflicts{list}` and **no** `import_id`, the client re-sends `ImportPgn` with per-game choices, and only then the ECO question arrives with an `import_id` (SEQ-APP-06). All five SDDs write `ResolveImport{import_id, eco}` with no `on_duplicate`. The UI keeps its own open item (OQ-UI-005, "whether one reply can hold both kinds"). **No SDD cites OQ-23.** If the architecture stands, `ResolveImport` gains a field (a protocol shape change); if the SDD flow stands, architecture 5.8 and OQ-23 change | `[OBS architecture.md:1015,2240]`; `[OBS SDD-chess-app.md:648-663,891,968]`; `[OBS SDD-chess-server.md:457]`; `[OBS SDD-desktop-tauri.md:378]`; `[OBS SDD-ui.md:514,523]` | Architect decides (OQ-23); then app, server, CLI, UI, Tauri | **G2** if `ResolveImport` changes; G1 text if the architecture is reworded |
| FIND-JR4-002 | Minor | **App error table still shows the old `eco_mismatch` entry.** IFP-APP-03 lists `{game_index, white, black, tag_eco, tag_name?, table_eco, table_name, ply}`. Architecture 5.8 (D-60 item 5), the app's own section 5 text and CLI/UI/Tauri use `{..., table_eco, table_name, ply, moves_san}` with no `tag_name`. This row is the one FF-1 and the TypeScript guards will be compared with | `[OBS SDD-chess-app.md:1222]` against `[OBS SDD-chess-app.md:1139]`, `[OBS architecture.md:1011]` | app | G0 |
| FIND-JR4-003 | Minor | **Stale "architecture still to be updated" wording.** Architecture 5.8 now names `moves_san`. App says the field name follows 5.8 "which the architect is updating"; CLI OQ-CLI-005 says "architecture and app still to be updated"; UI IA-UI-8 says "the app SDD must follow". The app also says the 10-minute idle time-out is "set here, as architecture 5.8 leaves it to the app design"; 5.8 now states 10 minutes `ESTIMATE` itself | `[OBS SDD-chess-app.md:891,1139]`; `[OBS SDD-chess-cli.md:823]`; `[OBS SDD-ui.md:579]`; `[OBS architecture.md:1019]` | app, CLI, UI | G0 |
| FIND-JR4-004 | Minor | **No SDD cites D-60 or D-61.** The rulings are applied (see section 4) but under "ruling n of 2026-11-01" or "D-59". `[RAN]` `grep -c "D-60\|D-61"` is 0 in all nine. D-61 is applied as app OQ-APP-014 / "ruling 6" and in the engine addendum. A reader cannot trace the stored-code rule, `--eco` mapping, stateless archive or `EngineFactory` ownership to the decision records | grep | all nine | G0 |
| FIND-JR4-005 | Minor | **Version and status lines lag the content.** CLI, server, UI and Tauri keep `Version 0.3 DRAFT` and a Status row ending "aligned to ... D-59 (accepted)", "Not reviewed", while their change logs carry a `0.3.1 DRAFT` row dated 2026-11-01 (the v0.3 review's FIND-JR3-012 said the status line was fixed; only the change-log text says so). Domain, rules and engine say `0.3 DRAFT` with an "(addendum)" row; app says `0.3 DRAFT (rev. 2)`. The five forms of naming one state make a baseline hard to state | `[OBS SDD-chess-cli.md:6,8,834]`; `[OBS SDD-ui.md:6,8,818]`; `[OBS SDD-chess-server.md:6,8,693]`; `[OBS SDD-desktop-tauri.md:6,8,588]` | CLI, UI, server, Tauri; others for the label | G0 |

Counts: **5 findings (Major 1, Minor 4).** Open v0.3 items that this pass did not touch and that still stand: FIND-JR3-001 to -006 are closed in the SDDs by the rulings (section 4); the mixed test-ID forms (`CT-SRV-30` with `CT-SRV-033`, `CT-EN-001..021` with undefined members, two-digit CLI/UI/Tauri forms) remain, as CR-9.9 allows.

## 4 Checks that passed `[OBS]` unless tagged

| Check | Result |
|---|---|
| D-60 (1) plain mode asks at a terminal | Architecture 5.8 = CLI DD-CLI-013 = app ALG-APP-16 text (the terminal test is the CLI's); CR-9.14 wording agrees |
| D-60 (2) `--eco keep-pgn\|keep-table` | CLI uses the hyphen forms (9 hits), protocol, app and UI keep `keep_pgn`/`keep_table`; the CLI mapping is stated. OQ-CLI-003 closed |
| D-60 (3) tag normalisation, stored code normalised, name empty | Domain `normalize_eco_tag`, rules `eco_tag` raw (UT-RUL-029), app ALG-APP-16, same A00 to E99 rule |
| D-60 (4) ply 0 looked up | Domain `keys[0]` is ply 0; rules UT-RUL-030; app. No "standard start only" text remains outside superseded rows `[RAN]` |
| D-60 (5) `moves_san`, no `tag_name` | CLI, UI, Tauri, server, app section 5. Exception: FIND-JR4-002 |
| D-60 (6) archives stateless | App 1140, CLI 520, UI 520-524, server CT-SRV-035, Tauri CT-TAU-019: no `import_id`, no `CancelImport`, `eco` in `ApplyDataImport` |
| D-60 (7) idle time-out | 10 min `ESTIMATE` in app; UI and Tauri give no number. Equal to architecture |
| D-61 `EngineFactory` | Trait and both factories in `chess-app` (app DD-APP-17); engine addendum says no engine change; no `engine -> app` edge |
| Schema | Architecture 7.2 against store DDL, all 14 tables, every column and type: **equal**. The store is stricter only in `NOT NULL` on `move_eval.engine_version` and CHECK constraints; none contradicts the ER text. Integer ratings, `error_permille`, `content_hash` nullable until terminal, `MOVE_EVAL` from White's point of view, `variation.created_at` all present |
| Crate edges | Architecture 3.2 against the edge paragraph of rules, domain, store, engine, app, server, CLI, Tauri: equal (`cli -> server, app`; `server, tauri -> app`; `app -> engine, store, domain, rules`; `engine -> rules`; `store -> domain`; `domain -> rules`) |
| Heartbeat and lease | Process 10 s, analysis claim 10 s, game lease row 2 s agree in architecture (lines 768-771), store and app. The 30 s staleness threshold is a store design `ESTIMATE` and architecture says it is the store's to set |
| Journal table | Unchanged since the v0.3 review (identical in architecture 7.6, domain, store, app) |
| Cited IDs `[RAN]` | Every `D-n`, `OQ-n`, `FF-n` cited in an SDD exists in the architecture. Cross-SDD test references (`UT-RUL-022`, `UT-RUL-027`, `UT-DOM-091`) resolve. v0.3 findings JR3-007 to -011 are closed in the text (JR3-008, -009, -011a verified) |

## 5 For Brian

1. **OQ-23 / FIND-JR4-001.** Keep the architecture's single reply for duplicates and ECO questions with `ResolveImport{import_id, on_duplicate?, eco}`, or keep the SDDs' two-pass flow and reword 5.8. The two-pass flow needs no protocol change; the single reply changes one command shape (G2).

Everything else is wording (G0) and needs no decision.

## 6 Status

Mode: Review | Gate: G0 (approver PENDING) | Findings: 5 (Major 1, Minor 4) | Needs Brian: 1 | Not run: build, test, Mermaid validation; no code exists. All SDDs stay 0.3 DRAFT, approvals pending.

## 7 Disposition

Added 2026-11-01. FIND-JR4-001: Brian Prascher chose to keep the SDD flow and reword architecture 5.8 (D-62, OQ-23 closed). FIND-JR4-002 to -005 were then corrected in the SDDs. The findings above are unchanged.
