# READY-step2-3: detailed-design readiness check for chess-domain, chess-store, chess-engine, chess-app, chess-cli, chess-server, ui, desktop-tauri

| Field | Value |
|---|---|
| Document | docs/engineering/readiness-step2-3.md |
| Task | chess-a0r.1.11 (baseline B3; B4 may follow) |
| Date | 2026-11-05 |
| Stance | Reviewer (readiness assessor), fresh invocation. Findings and questions only. No requirement, architecture, SDD, use-case or test-strategy file was edited. Nothing here is an approval, and the SDD baseline v1.0 decision is not made here (it is Brian Prascher's, G2) |
| Targets | The eight SDDs below, for detailed design and implementation in architecture §13 order (step 1 `chess-domain` beside the already-checked `chess-rules`; step 2 store, engine, app, CLI, server; step 3 UI, desktop) |
| Repository state read | `main` at `9b2a43a`; working tree clean except untracked `.mermaid/` `[RAN git rev-parse, git status]`. No code exists `[RAN ls Cargo.toml crates third_party -> No such file]` |
| Expected gate for the work | **G1** per block (one crate, §3.2 edges and §5.7 contracts unchanged; system-architect desk review, SWE-001 §3). **G3 if any item of the SF-01..SF-22 stored-format batch is touched, G2 if an edge or §5.7 contract moves.** No `GATE-101+` record exists yet (section 5, FIND-R23-008). PENDING |
| Overall verdict | **READY-WITH-CONDITIONS.** 0 Blocker, 5 Major, 10 Minor. Every block is READY-WITH-CONDITIONS. Nothing found stops a design or implementation start in §13 order; the conditions are dated to the step that needs them |

Evidence tags: `[OBS file:line]` read this session, `[RAN]` executed, `[INF]` inferred, `[UNV]` not verified.

## 1 Artifacts and revisions read

| Artifact | Revision as read | Used for |
|---|---|---|
| docs/design/SDD-chess-domain.md | 0.4.3 DRAFT, 2026-11-04 `[OBS :6]` | Target |
| docs/design/SDD-chess-store.md | 0.4.3 DRAFT `[OBS :6]` | Target |
| docs/design/SDD-chess-engine.md | 0.3.3 DRAFT `[OBS :6]` | Target |
| docs/design/SDD-chess-app.md | 0.4.4 DRAFT `[OBS :6]` | Target |
| docs/design/SDD-chess-cli.md, SDD-chess-server.md, SDD-ui.md, SDD-desktop-tauri.md | 0.4.1 DRAFT each `[OBS :6]` | Targets |
| docs/design/SDD-chess-rules.md | 0.3.3 DRAFT `[OBS :6]` | Seam only (not a target) |
| docs/requirements.md | v1.0 Amendment 8 incl. CR-10, CR-11, CR-12 `[OBS :1424-1426]`; TBD/TBR/TBC register `[OBS :1208-1262]` | R1 to R12 |
| docs/architecture.md | v1.0 amendment 8, addenda A to F; §3.2, §3.4, §5.7, §7.2, §7.6, §9, §10.1, §11 (D-63 to D-88), §11.4 (SF-01..SF-22), §13, §14 | A1 to A12, items (c), (d) |
| docs/functional_decomp.md, docs/use_cases/ | Allocation column of functional_decomp read by script; use cases not read | Allocation |
| docs/test-strategy.md | 1.0, Amendment 8 `[OBS :5]` | R12, A11 |
| docs/engineering/SWE-001-engagement-profile.md | 1.1, 2026-10-30 | Profile, gates |
| docs/engineering/readiness-chess-rules.md | Reference for format; its closure note (section 10) | Seam, format |
| docs/engineering/dependency-lookup-step1.md | 0.1 draft proposal | OQ-17 part 1 |
| docs/design/Board-pass-store-journal.md, OQ-16-desk-review-*.md, OQ-16-limited-rereview-*.md, SDD-v0.5-consistency-pass.md, PROPOSED-D-78-DAT-3-for-Brian.md, RELEASE_NOTES.md, README.md, CLAUDE.md | Read as data for context and status | Items (c) to (e) |

Toolchain on this Mac: `[RAN cargo --version; rustc --version; node --version -> cargo 1.99.1 (5f94df478 2026-08-27); rustc 1.99.1 (58cf8f9c3 2026-10-13); v26.13.0]`. This meets architecture §3.4 (Rust 1.99.1, Node 26.13.0 `[OBS architecture.md:452-453]`).

## 2 Checklist results

The requirements are one set allocated across the blocks (architecture §10.1), so the R-items are assessed once with per-block differences noted. Marks: P pass, N pass with note, F fail. A column is the block.

### 2.1 Requirement readiness (owner: Systems Engineer)

| # | Check | domain | store | engine | app | cli | server | ui | tauri | Evidence and note |
|---|---|---|---|---|---|---|---|---|---|---|
| R1 | Identified and allocated | P | P | P | P | P | P | P | P | `[RAN]` script: every requirement in architecture §10.1 whose primary component is one of these crates appears in that SDD's trace section; 0 missing for domain, store, engine, app, cli, server, ui, tauri. NFR-5 and NFR-6 appear in no SDD trace; §10.1 allocates them to the enabling blocks `packaging/` and `CI checks`, which have no SDD by design `[OBS architecture.md:1799, 1869-1870]`. Functions: every F-id allocated to `chess-store` and `chess-engine` in functional_decomp is cited in the matching SDD `[RAN script]`; same for domain, cli, server, tauri; app: 0 missing |
| R2 | Verifiable | P | P | P | P | N | P | N | P | `[RAN]` all 70 requirement sections of requirements.md contain a Verification statement (0 without). Trace rows without a case ID: app NFR-4 (dependency review, `[OBS SDD-chess-app.md:1416]`), cli NFR-1, NFR-9 (`:746,748`), tauri NFR-1, NFR-8 (`:496,498`), ui GAME-3, UI-2/UI-5, GAME-2, PLAT-2, NFR-8 (`SDD-ui.md:722,727,737,741,743`). They name a method ("E2E", "latency test") but no case ID: FIND-R23-013 |
| R3 | Quantified | N | N | N | N | N | N | N | N | Quantities exist (NFR-1 3 s, NFR-8 100 ms, ENG-5 200 ms, DAT-3 bounds, perft depths). Many are `EST`: TBD-5, 6, 8, 9, 12, 13, 19, 20, 30, 32, 33, 34, 35, TBR-2 `[OBS requirements.md:1215-1248]`, each with a named measuring step. Acceptable for ordinary criticality (SWE-001) |
| R4 | Singular and unambiguous | P | P | P | P | P | P | P | P | The DAT-3 ambiguity (time of an unfinished turn) is now decided: Brian, option A, "may lose up to one turn; is not required to" `[OBS requirements.md:1426 (CR-12 item 1), :633]` |
| R5 | Complete for behavior | P | P | P | P | N | P | P | P | Failure and degraded behavior added by Amendment 8 (D-65, D-71, D-80, D-82, D-83). CLI: exit status for TUI and plain-mode signals and for subcommands is open (TBD-36) `[OBS requirements.md:1246]`; headless is fixed (TBD-21). FIND-R23-011 |
| R6 | Interfaces specified | P | P | P | P | N | P | P | P | Protocol, §5.5 catalogue, §5.7 contracts, stored formats present. CLI: same TBD-36 gap. `DurabilityUnsupported` exit class 3 decided in all run forms `[OBS requirements.md:1426 (7); SDD-chess-cli.md:388]` |
| R7 | Consistent | N | N | P | N | P | P | P | P | See section 4 (a) to (e). Residual: stale status wording (FIND-R23-006) and version lines (FIND-R23-007). No requirement conflict found |
| R8 | TBD/TBR controlled | N | N | N | N | N | N | N | N | Register has owner, close-by and status per item. Open items in section 3. TBR-4 is open and gates the store SDD baseline (FIND-R23-005). The TBD-36 close-by "Before step 5" does not match §13 numbering (FIND-R23-011) |
| R9 | Non-functional present | P | P | P | P | P | P | P | P | NFR-1 to NFR-10, UI-4/6/7/8, PLAT-3. Security is best practice plus CVE avoidance only, by the user's decision (SWE-001 row Criticality; FIND-016) |
| R10 | Traceable | P | P | P | P | P | P | P | P | Decision Log CR-1 to CR-12. CR-12 is the last entry; there is no entry for the G-2 pass and OQ-16 closure of 2026-11-04 (`grep CR-13 requirements.md` -> no match `[RAN]`). FIND-R23-002 |
| R11 | Criticality assigned | P | P | P | P | P | P | P | P | Ordinary, not safety, not regulated, not security-critical; CERT-informed, no conformance claim `[OBS SWE-001 §1]`. Q8 of the gate still applies to stored formats whatever the class (SWE-001 §3) |
| R12 | Verification environment | N | N | N | N | N | N | N | N | Tools named and versioned (architecture §3.4, test-strategy). `cargo-mutants`, `cargo-fuzz`, `cargo-insta` not installed locally `[OBS dependency-lookup-step1.md E3]`; `cargo-fuzz` is no longer admitted (D-76). Test doubles and `FaultPlan` failpoints are specified in the store and app SDDs. Real-hardware items stay "not run" per SWE-001 |

### 2.2 Architecture readiness (owner: System Architect)

| # | Check | domain | store | engine | app | cli | server | ui | tauri | Evidence and note |
|---|---|---|---|---|---|---|---|---|---|---|
| A1 | Component boundary | P | P | P | P | P | P | P | P | §3.3 responsibilities; each SDD section 1 lists out-of-scope items |
| A2 | Interfaces | P | P | P | P | P | P | P | P | §5.7 contracts incl. `JournalCodec`, `NewGame.started_at`, `GameContextProvider` (D-74), `ReplayItem`/`MoveCheck` (D-79) `[OBS architecture.md:1016-1022, 2070, 2075]`. Seam gap: OQ-DOM-004 (`Preferences` field list), FIND-R23-014 |
| A3 | Dependencies | P | P | P | P | P | P | P | P | §3.2 edges equal in every SDD edge paragraph (v0.5 pass `[OBS SDD-v0.5-consistency-pass.md §4]`; not re-diffed edge by edge here `[UNV]`). `shakmaty` as a `[build-dependencies]` entry of `chess-rules` (D-75) adds no edge |
| A4 | Data ownership | P | P | n.a. | P | n.a. | n.a. | n.a. | n.a. | §7.4 ownership table; store owns format, domain owns `Fact`, app owns `JournalCodec`. See item (c) |
| A5 | Quality drivers | N | P | N | N | N | N | N | N | FF-1 to FF-8 `[OBS architecture.md §9]`; NFR-2 is the store's driver and now has the D-71 refusal case (TBR-4 pending). The others rely on EST budgets |
| A6 | Runtime behavior | P | P | P | P | P | P | P | P | §4.3 state machine, §4.8 concurrency, §7.6 recovery sequences including failure paths (D-65, D-80, D-82, D-83, D-85 to D-88) |
| A7 | Technology constraints | P | N | P | N | N | N | N | N | Versions in §3.4. All crates of steps 2 and 3 are `[UNV]` for features, licence, CVE (OQ-17 part 2); SDDs cite API names "from memory" (ASM-SRV-3, ASM-CLI-1, ASM-TAU-1, ASM-UI-1/2, ASM-APP-02). FIND-R23-003. Store needs primitives beyond `std` possibly (FIND-R23-004) |
| A8 | Decisions recorded | P | N | P | P | P | P | P | P | One-way doors have decisions: D-41, D-42 to D-43, D-63, D-64, D-68, D-69, D-70 (SF-01..22), D-58 re-classed one-way after release. Open: OQ-18 (termination string list, before first release), OQ-15 (deferred), ASM-6 (nothing released) |
| A9 | Cross-cutting | P | P | P | P | P | P | P | P | Error catalogue §5.5, logging NFR-10, config D-24, no authn by ASM-1 |
| A10 | Resource budgets | N | N | N | N | N | N | N | N | `ESTIMATE` values only (ASM-2, ASM-ST-3, ASM-APP-09, ASM-SRV-2); measured at step 2 |
| A11 | Testability | P | P | P | P | P | P | P | P | `FaultPlan` failpoints, `Core::open_for_calibration`, fake engine, test core; D-76 stable-only tools. Test IDs: `[RAN]` scan found 568 defined IDs; 20 cited-but-not-matched IDs on the first pass, 6 of which are the history rows of store change log 0.3 (UT-ST-001/020/057/058/059, CT-ST-011 `[OBS SDD-chess-store.md:1154]`), the rest range or list rows my pattern missed (spot-checked CT-EN-010..017, E2E-CLI-01,02, `[OBS]`). The v0.5 pass found 0 undefined and 0 duplicates `[OBS SDD-v0.5-consistency-pass.md §4]`; I did not re-prove every ID `[UNV]` |
| A12 | Consistency with requirements | P | P | P | P | P | P | P | P | No orphan allocation (R1). Reverse check §10.2 `[OBS architecture.md:1907-1922]` lists no orphan component |

## 3 Per-block results

| Block | SDD version | Verdict | Conditions (owner, before) |
|---|---|---|---|
| chess-domain | 0.4.3 | **READY-WITH-CONDITIONS** | C-3 OQ-17 part 1 accepted (SA, first `Cargo.toml`); C-9 ASM-DOM-008 confirmed (SE, before DD-DOM clock entity coding); OQ-DOM-004 (SW with app owner, before app `update_preferences`). Depends on `chess-rules` (already READY-WITH-CONDITIONS). The domain code part that the store needs ("part B", `chess-a0r.2.7` per RELEASE_NOTES `[OBS RELEASE_NOTES.md:19]`) must exist before store code (the store imports domain value types, `[OBS SDD-chess-store.md ASM-ST-1]`) |
| chess-store | 0.4.3 | **READY-WITH-CONDITIONS** | C-3 OQ-17 part 2 for the store crates (SA, first store `Cargo.toml`); C-4 OQ-14 spike with the wider scope of FIND-R23-004 (SW; SA admits any wrapper crate), before the first store code; C-5 TBR-4 accepted (SE proposes, Brian reviews G2) before the store SDD is baselined; C-1 rules SDD canonical-form text (SW) before baseline. Stored-format batch complete (item (c)). OQ-16 no longer blocks `[OBS architecture.md:2764; RELEASE_NOTES.md:25]` |
| chess-engine | 0.3.3 | **READY-WITH-CONDITIONS** | C-3 OQ-17 part 2 (`tokio` features, process crates); ASM-3/TBC-7/8/9 and OQ-EN-001 closed by the first run of the Stockfish 19 build script (SW or architect, step 2). `PINNED_UCI_ELO_RANGE` is deliberately unwritten `[OBS SDD-chess-engine.md:215, 541]`. No Amendment 8 item touches this crate `[OBS :17]` |
| chess-app | 0.4.4 | **READY-WITH-CONDITIONS** | C-3 OQ-17 part 2 (`serde_json`, `uuid`, ZIP, SHA-256, `tracing-subscriber`, `tokio-util`, `futures-util`; OQ-APP-008); FIND-R23-010 (publication of the adopted-prefix snapshot, app SDD owner, before CLI/UI degraded-board code); OQ-APP-010 (G1 additive wording, SA); needs store, engine and domain APIs as designed |
| chess-cli | 0.4.1 | **READY-WITH-CONDITIONS** | TBD-36 (Brian, G3, before CLI release; placeholder mapping is specified `[OBS SDD-chess-cli.md:390]`); ASM-CLI-1 `ratatui`/`crossterm` API names verified at first use; OQ-17 for `clap`, `crossterm`, `ratatui`, `expectrl`, `vt100` |
| chess-server | 0.4.1 | **READY-WITH-CONDITIONS** | OQ-SRV-001 spike for `axum` frame and message size options (SW, step 2); OQ-17 for `axum`, `rust-embed`; ASM-SRV-2 limits are EST |
| ui | 0.4.1 | **READY-WITH-CONDITIONS** | OQ-9 board spike (SPIKE-UI-1) as the first GUI task, with the D1 fallback; OQ-10 browserslist at set-up; TBD-18 wording (Brian reviews); FIND-R23-013 case IDs for generic trace rows |
| desktop-tauri | 0.4.1 | **READY-WITH-CONDITIONS** | OQ-11 sidecar naming for a universal target and OQ-TAU-002 binary name (SA, at packaging, step 3); ASM-TAU-1 Tauri 2.12.5 API names are from memory; OQ-TAU-005 (Quit control wording) |

No block has a Blocker. The reason no block is plain READY is OQ-17 (dependency lookups are `[UNV]` for every step 2 and step 3 crate and part 1 is a draft), which is a condition on the first `Cargo.toml` entry, not on design.

## 4 Specific checks requested

### (a) Cross-references and version lists after CR-3

| Check | Result | Evidence |
|---|---|---|
| Version headers agree with RELEASE_NOTES and requirements | **Pass.** Store 0.4.3, app 0.4.4, domain 0.4.3, rules 0.3.3, engine 0.3.3, cli/ui/tauri/server 0.4.1 | `[RAN grep '^| Version' on all nine SDDs]` equals `[OBS RELEASE_NOTES.md:10, 36]` and `[OBS requirements.md:1426 (RR-S-5, RR-S-6 correction)]` |
| Each SDD lists the other eight as siblings | **Mostly.** ui lists 6, desktop-tauri lists 4, others list 8. Harmless | `[OBS SDD-ui.md Sibling row; SDD-desktop-tauri.md Sibling row]` |
| Every `D-n`, `OQ-n`, `FF-n`, `SF-n` cited is defined | **Pass at v0.5**; not re-run here for 0.4.2 to 0.4.4 text. I confirmed SF-01..SF-22 each have a defining row `[RAN grep '^| SF-nn ' architecture.md -> 22 of 22, lines 2319-2340]` and each appears in the store SDD `[RAN grep -c]` | `[OBS SDD-v0.5-consistency-pass.md §1, §4]` for the earlier full scan |
| "Upstream baselines read" rows | **Stale** (FIND-R23-007): rules says architecture amendment 7; domain says requirements Amendment 7 and D-63 to D-77; app says architecture amendment 7 with addendum A and "store v0.3"; engine says addenda A to E and requirements Amendment 7 | `[OBS SDD-chess-rules.md:13; SDD-chess-domain.md "Upstream baselines read"; SDD-chess-app.md "Source revisions read"; SDD-chess-engine.md:16]` |
| Change-log order and status text | **Minor defects.** cli, server, ui, tauri change logs are not in version order (cli rows 0.1, 0.3, 0.3.1, 0.4.1, 0.4, 0.2) | `[OBS SDD-chess-cli.md:853-858; SDD-chess-server.md:697-703; SDD-ui.md:843-848; SDD-desktop-tauri.md:596-601]` |
| Project instructions | **Stale:** CLAUDE.md says "SDDs are 0.3.1 DRAFT" | `[OBS CLAUDE.md:17, 24]` |
| Findings from the v0.5 pass | JR5-001 (DurabilityUnsupported in front-ends), -002 (FF-3/FF-4 wording), -003 (clock wording), -004 (reset commit time), -006, -007, -008 are applied in text `[OBS SDD-chess-cli.md:388, 629; SDD-ui.md:611; architecture.md:1784-1785; SDD-chess-store.md:1104]`. **JR5-005 is not applied** (FIND-R23-001) |

### (b) Every requirement allocated to these blocks has a verification case

**Result: yes, with two notes.**
- Requirements side: all 70 requirement sections carry a Verification block `[RAN]`. CR-12 added the DAT-3 time-clause case and the NFR-2 full-sync refusal case `[OBS requirements.md:633, 1250, 1426]`.
- Design side: for each block, every requirement that architecture §10.1 makes primary for it appears in that SDD's trace section, and every row that names a requirement has a case ID or a named method except the 10 rows in R2 `[RAN]`. The DAT-3 case maps to UT-DOM-122/123 and UT-APP-32; the NFR-2 refusal case to CT-ST-085, CT-ST-001 and CT-APP-33 `[OBS requirements.md:1250]`.
- Note 1: the cases themselves do not exist (no code); "has a verification case" means a named, specified case, not a run.
- Note 2: TBR-4 (acceptance of the NFR-2 refusal case wording) is open (FIND-R23-005).

### (c) Store-block stored-format batch present in architecture §7.6 and the store SDD

**Result: complete. No missing item found.** Method: read §7.6 table, §7.2 notes and §11.4.2 in full `[OBS architecture.md:1428-1429, 1554-1612, 2319-2340]`; compared with the store DDL and §3.11.1 by reading; `[RAN]` counted SF-nn mentions in the store SDD (every one of SF-01..SF-22 appears at least twice).

| Batch item | Architecture | Store SDD |
|---|---|---|
| D-41 (kinds, one Fact to one record, no implicit resume, nullable clocks) | `:1555-1571`, D-41 `:2032` | journal table `:765-777`; header and record format `:752-757` |
| D-42 / D-68 `content_hash`, `sha256:v1:<hex>`, Date rule | `:1573`, D-68 `:2059`, SF-03/04 `:2321-2322` | DDL `:310`, `:407`, `FinalizeRequest` `:494`; Date rule is the app's (ALG-APP-09 `[OBS SDD-chess-app.md:895]`) |
| D-43 `error_permille INTEGER` and CHECKs (SF-19) | `:1428`, `:2337` | `:276-278` |
| D-63 / SF-01 finalize time totals | `:1569`, `:2054`, `:2319` | `:306`, `:409`, `:494`, `:775`, `:836` |
| D-64 / SF-02 / SF-13 `custom_start`, six-field comparison | `:1561`, `:1579`, `:1429`, `:2055`, `:2320`, `:2331` | `:297-298`, `:312-313`, `:406` |
| D-65 terminal re-check and durable prefix | `:1606-1607`, `:2056` | store side: CT-ST-003 extension `:1015`; the behavior sits in domain `:639` and app `:856-857` (as designed: it is app-owned) |
| D-66 `db_seq > last_intact_seq` is damage | `:1602`, `:2057` | `:808-814`, UT-ST-012 `:986` |
| D-67 voided hash advisory | `:1573`, `:2058` | `:480`, `:777`, UT-ST-086 `:1006` |
| D-69 / SF-05 / SF-06 `Voided.ended_at`, `journal_seq` | `:1571`, `:1612`, `:2060` | `:777`, UT-ST-085 `:1005` |
| D-70 / SF-07 to SF-16 (kind codes, CRC-32 ISO-HDLC, 65 536 limit, VoidReason, ClockSpec text, UUID text, null start_fen, half-null clocks, reason codes, start stamp) | `:2325-2334` | `:752-761`, `:793`, DDL `:258`, `:291`, `:299` |
| SF-17 to SF-22 CHECKs and `Suspend.cause`, `DrawDropped.by` | `:2335-2340` | `:311`, `:314`-`:316`, `:278`, `:291` |
| D-71 refusal of a volume without `F_FULLFSYNC` | `:2062` | `:940` (DD-ST-005), CT-ST-085 `:1046`, CT-ST-001 `:1013` |
| D-78 to D-80, D-82 to D-84, D-88 and OQ-26/27 | `:2069-2075`, `:1586-1596` | `:528` (D-82), `:816-823` (D-83), UT-ST-087 to 092 `:1007-1012` |

Two things on the page limit this conclusion:
- The text added after the independent limited re-review (store 0.4.3, app 0.4.4: D-83 extra cases, Addendum F, OQ-26 and OQ-27 applied) was checked only mechanically, by the closing pass. RELEASE_NOTES says so itself: "No reviewer has seen the closing-pass text, and the Board was a single same-family reviewer" `[OBS RELEASE_NOTES.md:9]`. Brian closed OQ-16 knowing this. Recorded as FIND-R23-015 for risk acceptance, not as a defect I found.
- ASM-9 (an imported row always gets a hash) is the premise of CHECK SF-17. The app SDD states the rule (`[OBS SDD-chess-app.md:1155, UT-APP-33 :1324]`), but architecture §14 still shows ASM-9 as Open `[OBS architecture.md:2746]` (FIND-R23-009).

### (d) BP-001 to BP-005: does fix text exist (for the PM's decision on the tracker bugs)

"Fix text exists" means the decision and the design text that carries it are on the page, and a verifying case is named. It does not mean the fix is built or tested (nothing is built). Whether to close the bug is the PM's call.

| Board finding | Fix text in architecture | Fix text in an SDD | Verifying case named | Fix text exists? |
|---|---|---|---|---|
| **BP-001** `GameEnd` lacks the finalize time totals | D-63 `[OBS architecture.md:2054]`; SF-01 `:2319`; §7.6 row `GameEnd` `:1569`; §7.2 note (3) `:1429` | Store `:306` (columns), `:409` (row 13), `:494` (`FinalizeRequest`), `:775` (payload) | UT-ST-070..073 `:998`; UT-APP-26 (live finalize equals finalize from `GameEnd` alone) `[OBS SDD-chess-app.md:1317]` | **Yes** |
| **BP-002** crash between `Move` and `Ended` leaves a terminal position resumed live | D-65 part 1 `:2056`; §7.6 "Terminal re-check on resume" `:1606` | Domain `:639` and diagram alt `:510`; app SEQ-APP-09 `:749`, `resume` row `:1059` | CT-APP-29 `:1360` (kill between `Move` and `GameEnd` for each terminal reason); CT-ST-003 extension `[OBS SDD-chess-store.md:1015]`; UT-DOM-117 (cited in v0.5 pass) | **Yes** |
| **BP-003** multi-record step not atomic on a write error | D-65 part 2 `:2056`; §7.6 "Partial-step failure: adopt the durable prefix" `:1607`; D-80 `:2071`; D-82 `:2073`, `:1596` | App ALG-APP-01 steps 5a, 5b `[OBS SDD-chess-app.md:856-857]`; store writer rule `:528` | UT-APP-28 `:1319`, UT-APP-40 `:1331`, UT-APP-35 `:1326`; UT-ST-087, CT-ST-087 `[OBS SDD-chess-store.md:1007, 1016]` | **Yes** |
| **BP-004** reconcile deletes `MOVE` rows when the tail is dropped as torn | D-66 `:2057`; §7.6 recovery bullet `:1602`; D-83 rebase `:1603`, `:2074` | Store reconcile table `:808-811`, clock rule `:814`, rebase `:816-823` | UT-ST-012 extended (journal truncated by one acknowledged record, store holds it) `:986`; UT-ST-088 to 091, CT-ST-088, CT-ST-089 `:1008-1011, 1017-1018` | **Yes** |
| **BP-005** "custom start" defined three ways, no column | D-64 `:2055`; SF-02, SF-13 `:2320, 2331`; §7.2 note (1) `:1429`; §7.6 flag 4 rewritten `:1579`; `Start` row `:1561`; OQ-27 option A `:2778` | Store DDL `:297-298`, CHECKs `:312-313`, row 10 `:406`; domain `StartPosition` `[OBS SDD-chess-domain.md:732]`; rules `canonical_form` omission `[OBS SDD-chess-rules.md:607]` | UT-APP-29 (four starts) `[OBS SDD-chess-app.md:1320]`; UT-DOM-115 `[OBS SDD-chess-domain.md:868]`; UT-RUL-031 `[OBS SDD-chess-rules.md:28]` | **Yes** |

Caveats the PM should weigh: (1) the `canonical_form` text in the rules SDD does not carry the `v1` version rule (FIND-R23-001), which belongs to BP-006/BP-007 rather than BP-005, so BP-005 itself is covered; (2) the independent re-review did not see the 0.4.3/0.4.4 text (FIND-R23-015); (3) all cases are specified only; none has run.

### (e) Stale "OQ-16 open" wording a reader could take as current status

`[RAN]` grep of every `*.md` for `OQ-16` with open/stays/pending/before-any/required wording, then read each hit's context. OQ-16 is closed `[OBS architecture.md:2764; README.md:71; RELEASE_NOTES.md:5-6]`.

**A. Reads as current status, no history marker (list for the owner to fix):**

| file:line | Wording |
|---|---|
| CLAUDE.md:24 | "OQ-16 stays open until they are applied. No `chess-store` code before OQ-16 closes." Also "the SDDs are at 0.3.1 DRAFT" |
| docs/architecture.md:23 | "None of this has been reviewed by anyone but its author; OQ-16 stays open." (Addendum F summary) |
| docs/architecture.md:129 | Addendum E list: "Not reviewed; OQ-16 stays open." |
| docs/architecture.md:130 | Addendum F list: "Not reviewed; OQ-16 stays open." |
| docs/architecture.md:2278 | §11.4 Status: "OQ-16 stays open until the batch is carried into the SDDs ... and a desk review" |
| docs/architecture.md:2467 | "OQ-16 is still open." |
| docs/architecture.md:2770 | OQ-22 row: "Open until OQ-16 closes" |
| docs/design/SDD-chess-store.md:10 | Approver row: "OQ-16 stays open. D-77: no `chess-store` code of any kind starts before OQ-16 closes." |
| docs/design/SDD-chess-store.md:1143 | Approval record: "OQ-16 stays open" |
| docs/design/SDD-chess-app.md:10 | Header: "OQ-16 stays open until the system-architect and the systems engineer have reviewed" (the dated "Update" note beside it covers only the desk reviews) |
| docs/design/SDD-chess-domain.md:10 | same wording |
| docs/design/SDD-chess-domain.md:1295 | Amendment 8 row: "OQ-16 stays open until ..." |
| docs/requirements.md:1250 | TBR-4 close-by "Before the store SDD is baselined (OQ-16 closure, chess-a0r.1.8 chain)": names a closed gate as a condition |

**B. Dated history (acceptable as written; listed so nobody counts them twice):** RELEASE_NOTES.md:26, 40, 45, 63, 86 (the top entry at :5 says it supersedes them); docs/architecture.md:2160-2165, 2173, 2181, 2369, 2371, 2474, 2642, 2644, 2731, 2776 (marked history), 2804; docs/requirements.md:1424, 1426 (CR-10 and CR-12, dated 2026-11-04, the same day OQ-16 closed; no later entry corrects them, see FIND-R23-002); change-log rows docs/design/SDD-chess-domain.md:27-29, SDD-chess-app.md:1609-1611, SDD-chess-store.md:1156-1157; docs/design/OQ-16-*.md, Board-pass-store-journal.md:19, SDD-v0.5-consistency-pass.md:51, docs/engineering/readiness-chess-rules.md:81.

**C. Correct:** README.md:5, 71; architecture.md:36 (past tense), :2764 (the OQ-16 row), RELEASE_NOTES.md:5-25.

## 5 Findings (addressed to owners)

Severity: Blocker (design cannot proceed safely), Major (proceed only on a recorded assumption; owner resolves by the stated point), Minor. "Blocks design start?" is about starting detailed design or implementation of the named block.

| FIND | Check | Artifact and ID | Observation | Why it matters | Question to owner | Owner | Sev | Gate | Blocks design start? |
|---|---|---|---|---|---|---|---|---|---|
| FIND-R23-001 | A2, A8, A12 | SDD-chess-rules 0.3.3 `:28, :357, :612`; SDD-v0.5 JR5-005; architecture D-68, SF-03 `:2059, :2321` | The rules SDD owns the bytes that `content_hash` covers (`canonical_form`, DD-RUL-016). It still describes the value as `"sha256:" + hex` and says the prefix belongs to the app; it states no rule that a change to steps 1 to 5 is a new `v2`, and no `Date` rule. Its own change log says JR5-005 is "deferred" `[OBS]` | `canonical_form` is a one-way door at release (D-68). An editor of that function has no text telling them that a change breaks every stored hash. The app SDD carries `v1` and the Date rule (ALG-APP-09 `:895`), so the producer is right and the owner of the input is silent | Will the rules SDD gain the `v1` rule, the version-bump rule and the `Date` pointer before DD-RUL-016 is coded and before the SDDs are baselined? | Software-engineer (rules SDD owner) | **Major** | G0 text | No for the eight blocks. Blocks DD-RUL-016 coding and a clean rules baseline |
| FIND-R23-002 | R10, R7 | requirements.md Decision Log `:1424-1426`; RELEASE_NOTES.md:23 ("Decision Log entry ... not yet written") | No Decision Log entry records Brian's 2026-11-04 decisions after CR-12: D-83 added cases confirmed (G2), G-2 passed, OQ-16 closed. CR-10 and CR-12 still say OQ-16 is open `[RAN grep CR-13 -> none]`. CLAUDE.md ranks requirements.md first in precedence and requires amendments to carry a Decision Log entry | The highest-precedence document is silent on the decision that unblocked store work; a reader following precedence finds "OQ-16 stays open" | Will the SE add the entry (CR-13) before the SDDs are baselined? | Systems-engineer | **Major** | G0 | No. Before baseline |
| FIND-R23-003 | A7, A8, R12 | architecture §3.4, OQ-17 `:2765`; dependency-lookup-step1.md status; D-47 `:2038` | OQ-17 part 1 (step 1 crates) is a "Draft proposal" with no acceptance on the page. Part 2 (every crate of the store, engine, app, CLI, server, UI, desktop) is `[UNV]` for features, licence and CVE: `serde_json`, `crc32fast`, `tokio` features, `rusqlite` (the bundled SQLite release "was not read" `[OBS architecture.md:458]`), ZIP, SHA-256, `clap`, `ratatui`, `crossterm`, `axum`, `rust-embed`, Tauri, npm packages. SDD API names for several are "from memory" | Dependencies are decisions (guardrail 5). `rusqlite`'s bundled SQLite must support what the DDL uses (`json_valid` CHECK, `WITHOUT ROWID`, WAL, `VACUUM INTO`; ASM-ST-5 `[OBS SDD-chess-store.md:1115]`). A rejected crate is cheap before code, costly after | Has part 1 been accepted, and when will each part 2 lookup be done (store crates first)? Who confirms the SQLite options the DDL needs? | System-architect (OQ-17 owner); Software-engineer (lookups) | **Major** | G2 for any crate not already admitted (SWE-001 §3) | No. Blocks the first `Cargo.toml` entry of each crate |
| FIND-R23-004 | A7, A11 | architecture §3.4 "Deferred (spike first)", OQ-14 `:2762`; store SDD `:219-221`, OQ-ST-004 `:1081`, D-82 `:528`, D-83 `:821` | The OQ-14 spike is scoped as "does `std` provide `flock` and a full sync, or is `rustix`/`libc` needed". The store design also relies on, and marks `[UNV]`: `ftruncate` followed by `F_FULLFSYNC` on APFS (D-82 cleanup), a no-replace rename (`renamex_np` with `RENAME_EXCL`, or `link` then `unlink`; D-83), directory sync after a cross-directory rename, and `crc32fast` being the ISO-HDLC variant `[OBS SDD-chess-store.md:218]` | If any primitive needs `libc` or `rustix`, that is a new dependency and an `unsafe` module in the one crate that guards NFR-2, discovered after the store design is baselined. `link`/`unlink` exist in `std`, so an in-`std` path may exist `[INF]` | Will the spike cover those four primitives and the CRC check value (`0xCBF43926`, SF-08), and record the result before the store SDD is baselined? | Software-engineer (spike); System-architect (admits any wrapper crate) | **Major** | G1 (spike, text); G2 if a crate is added | No. Blocks the first store code; deferral to "before step 2" is acceptable with the wider scope |
| FIND-R23-005 | R8, A5 | requirements TBR-4 `:1250`; CR-12 item 5 `:1426` | The NFR-2 full-sync refusal verification case (D-71) is mapped to CT-ST-085, CT-ST-001, CT-APP-33 but its wording and pass criterion await the SE and Brian (G2). Close-by text names OQ-16 closure, which is done | NFR-2 is the requirement the store exists for; its case must be accepted before the store SDD is baselined (CR-12 says so) | Will the SE propose the final wording and Brian review it before the store SDD baseline? Please update the close-by cell | Systems-engineer proposes; Brian Prascher reviews | **Major** | G2 | No. Gates the store SDD baseline, not store design |
| FIND-R23-006 | R7 | Section 4 (e), list A (13 locations) | Current-status wording says OQ-16 is open | A reader (human or agent following CLAUDE.md) may refuse store work that is allowed, or mis-state the gate | Please change the listed lines to "closed 2026-11-04 (Brian Prascher)" or add a history marker | Software-engineer (SDD headers, CLAUDE.md); System-architect (architecture); Systems-engineer (requirements TBR-4) | Minor | G0 | No |
| FIND-R23-007 | R7, A12 | Section 4 (a) | "Upstream baselines read" and "Source revisions read" rows lag (rules, domain, app, engine); CLAUDE.md says SDDs 0.3.1; four change logs are out of version order | Repeat of FIND-JR4-005 and FIND-JR5-006: a reader cannot tell which upstream revision a design was checked against | Please refresh the rows and sort the logs at the baseline edit | Software-engineer | Minor | G0 | No |
| FIND-R23-008 | A7, seam | SWE-001 `:7-30`; `docs/engineering/gates/` absent `[RAN ls docs/engineering]` | SWE-001 still says the toolchain is older than baseline and "Update both before step 1", lists upstream artifacts at Amendment 6, and "No gate has been recorded". The toolchain is now at baseline `[RAN]`. No `GATE-101+` file exists for any block | G1 needs a recorded change proposal and gate before work is called done; the stale profile misstates the baseline | Will the profile be refreshed and a GATE record opened at each block's first code task? | Software-engineer | Minor | G0 | No |
| FIND-R23-009 | R8, A8 | architecture §14 rows ASM-9 `:2746`, OQ-22 `:2770`, ASM-11 `:2748`; §3.4 Rust and Node rows `:452-453` | Status accounting: ASM-9 is shown Open though the app SDD states and tests the import-hash rule (UT-APP-33); OQ-22 "Open until OQ-16 closes"; §3.4 still says the dev Mac has Rust 1.89.0 and Node 26.12.1 | SF-17 is a stored CHECK whose premise is shown open; stale environment note | Can ASM-9 be closed by the architect against UT-APP-33, OQ-22 closed, and the §3.4 notes refreshed? | System-architect | Minor | G0 | No |
| FIND-R23-010 | A2, A6 | SDD-chess-cli OQ-CLI-007 `:846`; SDD-ui OQ-UI-006 `:837`; app ALG-APP-01 step 5a `:856` | The app SDD does not say whether adopting a durable prefix publishes a snapshot event, but the CLI and UI degraded-board designs (DD-CLI-014, DD-UI-015) need the adopted prefix shown | A seam between three SDDs left to the front-ends to guess | Does step 5a publish a snapshot (a G1 text decision inside `chess-app`)? | Software-engineer (app SDD owner) | Minor | G1 | No. Before front-end degraded-board code |
| FIND-R23-011 | R5, R6, R8 | requirements TBD-36 `:1246`; SDD-chess-cli OQ-CLI-002 `:847`, `:386-398` | Exit status of the TUI and plain mode on signals and of non-headless subcommands is open and deferred, owner Brian (public CLI surface, G3). Close-by is "Before step 5 (CLI front-end)", but §13 has three steps and puts the CLI in step 2 | The CLI SDD specifies placeholders until TBD-36 closes, so design can proceed; the "step 5" does not map to anything in §13 | Which step does "step 5" name, and will Brian decide before CLI release? | Systems-engineer (register); Brian Prascher (decision) | Minor | G3 | No |
| FIND-R23-012 | R4, A5 | SDD-chess-domain ASM-DOM-008 `:1254`; XC-DOM-A8-4 `:1239`; RELEASE_NOTES.md:14 (chess-a0r.1.45) | The domain rule that draw-offer time counts in `total_active_ms` but in neither side's used time is "SE to confirm against DAT-3". The draw-offer interval before a stop is not counted by the D-78 seed (decided by Brian as designed) | A stored total (`total_active_ms`) depends on the rule; confirmation is still pending | Does the SE confirm the draw-offer rule of 3.11.13 as an acceptable reading of DAT-3? | Systems-engineer | Minor | G0 | No. Before the clock entity is coded |
| FIND-R23-013 | R2, A11 | Section 2.1 R2 row | Ten trace rows name a method but no case ID | A reader cannot tell which case verifies the row | Please add case IDs to those rows | Software-engineer (cli, ui, tauri, app owners) | Minor | G0 | No |
| FIND-R23-014 | A2 | SDD-chess-domain OQ-DOM-004 `:1216`; app `:1026`; requirements UI-1 `:729-740` | `Preferences` is a domain type with a `ts-rs` derive (D-50) but no SDD gives its Rust field list; the app SDD describes it in prose as a "versioned blob, `deny_unknown_fields`". It is stored in `PREFERENCES.json` | A stored JSON shape and a generated wire type with no single definition. Not in the SF batch; additive fields look possible, `deny_unknown_fields` makes downgrade fail `[INF]` | Who writes the field list and defaults, and is the blob's forward-compatibility rule intended? | Software-engineer (app owner) with Systems-engineer | Minor | G1 | No. Before app `update_preferences` and the UI Settings screen |
| FIND-R23-015 | A8, seam | RELEASE_NOTES.md:9; SDD-chess-store 0.4.3 `:1159`; app 0.4.4 `:1613`; architecture Addendum F | The text added after the last independent review (D-83 added cases, `started_at` mechanism, OQ-27 canonical form, D-78 ratification wording) was checked only by the closing pass (ID scan, Mermaid, version headers), by the author's side. Brian closed OQ-16 with that knowledge | The store rebase and recovery are the most intricate text in the set and run before any user can lose a move | Does Brian accept this residual risk, or want a desk review of the 0.4.3 rebase text before store code? | Brian Prascher (risk acceptance); System-architect (desk review if asked) | Minor (information) | G2 (acceptance) | No |

## 6 Open items and deferral

"Acceptably deferred" is my reading against the skill's rule: deferral is acceptable when the named step precedes any design or code that depends on the answer, an owner and a closing step exist, and a fallback bounds the damage.

| Item | State `[OBS]` | Affects | Named step / owner | Acceptably deferred? |
|---|---|---|---|---|
| OQ-14 (`flock`, full sync in `std`) | Open `[architecture.md:2762]` | store journal writer | Spike before step 2, core developer | **Yes, with the wider scope of FIND-R23-004.** Must finish before the first store code; no design change expected, fallback is `libc`/`rustix` (a G2 dependency) |
| OQ-17 (dependency lookups) | Open; part 1 draft, part 2 not done `[architecture.md:2765]` | every block with a crate | "Before each crate is first used", architect | **Yes for design; not for the first `Cargo.toml` entry** (FIND-R23-003) |
| OQ-9 (D-4 board spike) | Open `[architecture.md:2757]`; SPIKE-UI-1 `[SDD-ui.md:710]` | ui board adapter | First GUI task, step 3, GUI developer | **Yes.** The board contract isolates the adapter; fallback is the D1 adapter (§11.1) |
| OQ-10 (browserslist query) | Open `[architecture.md:2758]` | ui build config | When the build is set up, step 3 | **Yes.** Affects configuration only |
| OQ-11 (Tauri sidecar naming) | Open `[architecture.md:2759]`; OQ-TAU-002 | desktop packaging | Packaging, step 3, architect | **Yes.** Affects bundle configuration only |
| OQ-28 (D-78, DAT-3) | **Closed** 2026-11-04, Brian option A `[architecture.md:2779; requirements.md:1426]` | domain, app | n.a. | Closed. DAT-3 case exists (`requirements.md:633`) |
| OQ-26, OQ-27 | Closed, Brian option A `[architecture.md:2777-2778]` | store, app, rules | n.a. | Closed |
| OQ-16 | Closed 2026-11-04 `[architecture.md:2764]` | store code | n.a. | Closed (stale wording: FIND-R23-006) |
| ASM-3, TBC-7, TBC-8, TBC-9, OQ-EN-001 (Stockfish 19 facts, `UCI_Elo` range) | Open `[architecture.md:2740; requirements.md:1257-1259; SDD-chess-engine.md:668]` | engine, ENG-2 calibration | Closed by the first run of the engine build script, step 2, architect | **Yes.** The SDD writes no number until then; `PINNED_UCI_ELO_RANGE` is declared first in one file |
| TBR-4 (NFR-2 refusal case) | Open `[requirements.md:1250]` | store SDD baseline, NFR-2 | SE proposes, Brian reviews, before store SDD baseline | **Yes for design; no for baselining the store SDD** (FIND-R23-005) |
| TBD-36 (CLI signal and subcommand exit status) | Open, deferred `[requirements.md:1246]` | cli | Brian (G3), "before step 5" | **Yes.** Placeholders specified; step numbering to fix (FIND-R23-011) |
| TBD-18 (screen-reader wording) | Open (design) `[requirements.md:1228]` | ui | Step 3, before the VoiceOver walk-through | **Yes** |
| TBD-25 (PGN size 10 MB, EST), TBD-35 (archive about 24 MiB, EST), OQ-19 | Open (design) | app, server, ui | Step 2 importer and archive-size check | **Yes.** Limits are parameters (`ServeOptions`, `ValidationFailed` text, OQ-APP-009/010) |
| TBD-5, 6, 8, 9, 12, 13, 19, 20, 30 to 34, TBR-2, ASM-2, ASM-ST-3, ASM-APP-09/10 (EST values) | EST | engine, app, store, server, ui | Named measuring step 2 | **Yes.** Values live in config or one constant |
| ASM-4, ASM-CLI-7 (terminal resize escape) | Open `[architecture.md:2741]` | cli | PTY check, step 2; fallback message exists | **Yes** |
| ASM-5, TBC-10 (macOS facts for `install-cli`, signing) | Open `[architecture.md:2742]` | cli install-cli, packaging | M walkthrough, step 3 | **Yes** (design tolerates a wrong answer) |
| ASM-6 (nothing released, formats cheap to change) | Open `[architecture.md:2743]` | store one-way doors | Revisit at first-release gate | **Yes**, provided no build reaches real data first |
| ASM-9 (imported rows always hashed) | Shown Open; rule stated in app text | SF-17 CHECK | Architect closes against UT-APP-33 | **Yes** (FIND-R23-009) |
| ASM-11 (`F_FULLFSYNC` probe returns `ENOTSUP`/`ENOTTY`) | Open `[architecture.md:2748]` | D-71 | CT-ST-001, OQ-14 spike | **Yes**, but widen with FIND-R23-004 |
| OQ-18 (termination strings frozen) | Open `[architecture.md:2766]` | archive format, SF-10/SF-15 | Before first release | **Yes.** Codes are listed in domain 3.5 and store `:775` |
| OQ-15 (nested variations) | Deferred by user `[architecture.md:2763]` | store, app | None scheduled | **Yes** (one line per row stands) |
| OQ-22 | Row says Open; outcomes applied | store, domain, app | Close the row | **Yes** (FIND-R23-009) |
| OQ-DOM-004, OQ-APP-003, OQ-APP-010 to 012, OQ-SRV-001/002/006, OQ-TAU-001/004/005, OQ-CLI-004/007, OQ-UI-002/006, OQ-ST-001/003/004/005 | Open in SDD tables | named blocks | Owner named in each row | **Yes** for design; the ones that need a decision are findings 010, 014 |
| OQ-EN-002 (where the `UCI_Elo` clamp runs) | Open `[SDD-chess-engine.md:669]` | engine, app | App owner, SA desk review (G1) | **Yes** |

## 7 Assumptions needed to proceed

| ID | Assumption | Risk if wrong | Owner / when resolved |
|---|---|---|---|
| ASM-R23-1 | `link`/`unlink` or `renamex_np` can provide the no-replace rename of D-83 without breaking the no-`unsafe` preference (FIND-R23-004) | A `libc`/`rustix` dependency and one documented `unsafe` module appear in `chess-store` (G2 admission) | SW spike result, SA admission, before first store code |
| ASM-R23-2 | The bundled SQLite behind `rusqlite` 0.41.1 has JSON functions and `WITHOUT ROWID` as the DDL needs (ASM-ST-5) | DDL edit before release (two-way until release, ASM-6) | SW, OQ-17 part 2 for `rusqlite` |
| ASM-R23-3 | The residual unreviewed text of FIND-R23-015 holds | A recovery defect found at store coding, fixed in text (G1) | Brian accepts or asks for a desk review |

None of these touches a one-way door that is undecided or changes a gate tier; so none is a blocker.

## 8 Verdict and what unblocks what

**Overall: READY-WITH-CONDITIONS.** Requirement checks R1 to R12 and architecture checks A1 to A12 pass, with the notes above, for all eight blocks. There is no Blocker. Design and implementation of each block may start in §13 order, with the conditioned parts marked provisional.

| # | Condition | Owner | Before |
|---|---|---|---|
| C-1 | Rules SDD gets the `canonical_form` version and `Date` text (FIND-R23-001) | Software-engineer | DD-RUL-016 coding; SDD baseline |
| C-2 | Decision Log entry for G-2 and OQ-16 closure (FIND-R23-002) | Systems-engineer | SDD baseline |
| C-3 | OQ-17 part 1 accepted; part 2 done per crate (FIND-R23-003) | System-architect, Software-engineer | First `Cargo.toml` entry of each crate |
| C-4 | OQ-14 spike with the wider scope (FIND-R23-004) | Software-engineer; System-architect | First store code |
| C-5 | TBR-4 accepted (FIND-R23-005) | Systems-engineer, Brian Prascher | Store SDD baseline |
| C-6 | Stale OQ-16 wording and version rows fixed (FIND-R23-006, 007, 009) | Owners as listed | SDD baseline (not code) |
| C-7 | `GATE-101+` records opened per block (FIND-R23-008) | Software-engineer | Each block's first code task |

Items the PM may want closed before the SDDs are baselined at v1.0 (the decision is Brian's): C-1, C-2, C-5, C-6. Items needed before code: C-3, C-4, C-7. The questions for each owner are in section 5.

## 9 Hand-back log

| FIND | Sent to | Sent | Answered | Reopened |
|---|---|---|---|---|
| FIND-R23-002, 005, 011, 012 | Systems-engineer (005, 011: Brian also) | Not sent by me; the program manager routes this report | | |
| FIND-R23-003 (SA part), 009, 004 (SA part) | System-architect | Not sent by me | | |
| FIND-R23-001, 004 (spike), 007, 008, 010, 013, 014, 003 (lookups), 006 | Software-engineer | In this report | | |
| FIND-R23-015, 005, 011 | Brian Prascher | Not sent by me | | |

No finding is marked resolved. No owner artifact was edited.

## 10 Not analyzed or not run

- No code exists; no build, test, coverage, mutation, scan or benchmark was run. Commands run: `cargo --version`, `rustc --version`, `node --version`, `npm --version`, `ls`, `git status`, `git log`, `git rev-parse`, greps, and four Python scans over the markdown (requirement and Verification presence, trace-section coverage by requirement, F-id coverage, test-ID definition scan).
- The test-ID scan is not a proof: it left 20 unmatched IDs, of which I resolved the history rows and spot-checked three list or range rows; the other items are not individually resolved `[UNV]`.
- The SDDs (about 8,800 lines) and architecture.md were read by section (7.6, 7.2 notes, 3.4, 10.1, 11.4.2, 14, change logs, headers, trace and open-item tables), not end to end. Content outside those regions can hide defects.
- Not re-diffed: crate-edge paragraphs against §3.2 (relied on the v0.5 pass), the `Fact`-to-record table against domain and app text field by field (relied on the v0.5 pass plus my reading of the store side), Mermaid diagrams (not re-validated; `validate-mermaid` not run; this file has no diagram), the use cases (not read), `functional_decomp` allocation beyond the F-id coverage script, and the Board and desk-review reports beyond BP-001 to BP-005 and the re-review conditions.
- Not checked against sources: the FIDE Laws, PGN standard, Stockfish 19 behavior, `shakmaty`/`pgn-reader`, `rusqlite`, `axum`, Tauri, `ratatui` APIs; the SDD statements about them are `[UNV]` as the SDDs say. `dependency-lookup-step1.md` was read, not re-run.
- Whether the Python scans' mapping of "primary component" to crates (hand-built from §10.1) is exact for every requirement: I mapped by reading the table; a requirement with a primary component named by a service (for example `PlayerService`) was assigned to `chess-app`.
- Not verified: that no other file in the repository (code comments, beads issues, `.pm/` baselines) carries stale OQ-16 wording; the grep covered `*.md` only. Beads was not touched.
