# SDD systems-engineer desk answers (Q-SE-1 to Q-SE-26)

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/design/SDD-SE-desk-answers.md | **0.1 DRAFT** | 2026-10-31 | G0 text document. Nothing in it is baselined or applied. | Systems engineer role (Claude), for Brian Prascher | **RESOLVED by Brian Prascher, 2026-10-31** (requirements Amendment 7, CR-9). Interpretations Q-SE-1, 2, 3, 4, 5, 12, 13, 18, 19, 22 and 25 confirmed as written; Q-SE-8 decided with decision 9 (archive size limit as an ESTIMATE, TBD-35); Q-SE-9 (h) decided as "refuse" (decision 13); Q-SE-17 **deferred** (open TBD-36); Q-SE-20 TBD-18 wording stays open for his review; Q-SE-21 dropped (not a requirement, decision 11). Original text: Brian Prascher for every item that adds or changes a requirement, or touches a public command or exit code (G2/G3). |

**Stance.** Engineer stance (author/propose). This is a desk answer, not an independent review. No requirement, architecture, SDD or use case was edited and nothing was committed.

**Scope of reading.** `[OBS]` = read in this pass. `[INF]` = inferred from what was read. `[UNV]` = not verified (external fact or text not read).
Read in full: `docs/requirements.md` (all 1371 lines). Read: `docs/design/SDD-architect-triage.md` section 7.2 and the triage rows for every XC/UR cited; the cited `SDD-consistency-review.md` rows; `docs/functional_decomp.md` rows F-2.2.4, F-3.5 to F-3.7, F-8.1 to F-8.12; `docs/use_cases/` UC-2, 3, 4, 5, 6, 9, 11, 12, 14, 15, 16, 22 to 27 (targeted passages, plus the "Channels to actors" line of every UC); architecture lines quoted below (greps, not a full read); domain SDD FIND-DOM-6 to FIND-DOM-10 and 3.5. Not read: SDD bodies beyond those passages, test-strategy beyond its "Clocks" row, UC-7, 8, 10, 13, 17 to 21, 28, 29 beyond greps.

**Classification used in the table.**
- **ANSWERED** = the requirements or use cases already decide it. Action: SDD edit only.
- **RECOMMEND** = not decided in text; I propose an answer that is an interpretation or clarification and changes no baselined text. It is logged as an SE interpretation for Brian to correct (the CR-4/CR-6.3 pattern). G0.
- **NEEDS-USER** = the answer would add a requirement, change or extend baselined text or a public command/exit code, or limit a promised behaviour. Brian decides.

## 1 Summary table

| Q-SE | Class | One-line answer |
|---|---|---|
| Q-SE-1 | RECOMMEND | Duplicate = same Seven Tag Roster values plus `SetUp`, `FEN`, `TimeControl` (where present) and same moves; custom and other tags ignored. Interpretation of IF-2. |
| Q-SE-2 | RECOMMEND | Store evaluations from White's point of view (mate signed the same way); CPL stays mover's POV (ANA-3). Start-position eval may be transient. `Takeback{ply}` is only a stale-command guard; the core derives the target from RULE-3. |
| Q-SE-3 | RECOMMEND | An archived Player is not selectable as a side (hidden, PLY-4). `archived_player` and `game_open` are defensive/refusal reasons for situations the requirements already describe. |
| Q-SE-4 | RECOMMEND | Only checkmate-over-fifty-move is stated. Order checkmate, stalemate, dead position, threefold, fifty-move (labels only differ). Add custom-clock limits as an EST TBD; Fischer increment; White's clock from game start; c-c Step runs the clock for that move only; takeback in `EngineError` allowed; reset boundary = store-creation time of the game. |
| Q-SE-5 | RECOMMEND | PGN `Termination` mapping is already in IF-1. Internal strings follow the domain table and become part of the `games.json` format. Custom FEN that is already mate/stalemate: reject for a new game, accept for analysis. Custom start has no ECO. |
| Q-SE-6 | ANSWERED | No separate typed-input latency requirement. NFR-8 (100 ms feedback to a key press during a search) is the only budget; echo of typed text is local. |
| Q-SE-7 | ANSWERED | No ECO tag on PGN export: the IF-1 tag set is closed (TBD-24). The archive carries ECO and opening name in `games.json`. |
| Q-SE-8 | NEEDS-USER | PGN 10 MB is a design EST (TBD-25). An archive size limit would restrict DAT-2/IF-5 and needs Brian (decision 9). Path-based CLI import should have no limit. |
| Q-SE-9 | RECOMMEND (part NEEDS-USER) | One row = one saved line plus anchor ply. PLY-7 filters variations by game, so no `player_id` is needed. A branch started after the last ply has no PGN variation: Brian decides refuse vs another form. |
| Q-SE-10 | ANSWERED | Intent: the clock-derived budget is an upper limit, never a target; the computer must not flag (RULE-4 test, UC-6). Constants are design values checked by the RULE-4 test. |
| Q-SE-11 | ANSWERED | RULE-2 says: accepts only if its own evaluation, from its own side, is <= 0. |
| Q-SE-12 | RECOMMEND | Confirmed: an in-flight failure is reported (ENG-4, UC-6 5b); retry is the user's next request. Reword F-3.7 "restart once". |
| Q-SE-13 | RECOMMEND | CSP: PLAT-3 states a floor (`default-src 'self'`); extra directives allowed if needed. `localhost:<port>` and `127.0.0.1:<port>` both count as the own loopback origin. No-`Origin` requests: Host must match always; WebSocket upgrade without `Origin` is rejected. |
| Q-SE-14 | ANSWERED | No requirement for a port hint. DAT-4 requires naming the front-end only. No store column. |
| Q-SE-15 | ANSWERED | Nothing in the requirements writes `config.toml`; it is hand-edited and read-only (PLAT-4, UI-1, NFR-5). |
| Q-SE-16 | RECOMMEND | Add a one-line statement for F-8.1 (three launch processes) and F-8.2 (CLI half: flags, config, browser, signals; server half: bind and serve). F-8.6 to F-8.10 having no SDD is already stated (CR-4.17). |
| Q-SE-17 | NEEDS-USER | Plain mode covers all flows already (UC channels). New exit-code classes for TUI signals/subcommands and any extra setup flags are public CLI surface: Brian (G3). |
| Q-SE-18 | RECOMMEND | Accept the 4096 code-point guard as a robustness bound reported as "too long". No text change. |
| Q-SE-19 | RECOMMEND | Yes: any complete backup that was verified to open counts as "the newest backup" for the 24 h rule. |
| Q-SE-20 | RECOMMEND | No bundle-size requirement (design budget only). TBD-18 sample wording is for Brian's review per the register. GUI key bindings are design within GUI-2 and UI-9. |
| Q-SE-21 | RECOMMEND | Not a requirement. Concurrent launches are covered by the store lock (DAT-1, DAT-4). Architecture 6.3 states it as design; the plugin is Brian's decision 11. |
| Q-SE-22 | RECOMMEND | Write the en-passant field always on output; keep the repetition key separate (FIDE capture-possible rule). No external expectation found in the requirements. |
| Q-SE-23 | ANSWERED | UI-2 (CR-4) says the TUI draws the Elo chart in character cells; the GUI chart is separate with a text alternative. |
| Q-SE-24 | ANSWERED, **superseded 2026-10-31 (CR-9.8)** | DAT-3 (CR-8): a game with no named position, "for example a custom start", has no ECO code and no name. "Unclassified" is display wording only. Superseded for an imported custom start with an `ECO` tag: the file's code is kept, no name. |
| Q-SE-25 | RECOMMEND | `5+0` = 5 minutes plus 0 seconds increment; PGN `TimeControl` in seconds. Units are not written in RULE-4 or CLI-4; document in help/NFR-5. |
| Q-SE-26 | ANSWERED | Neither changes a requirement: RULE-2 already says "from its own side, <= 0"; ENG-1/IF-4 say bundled native child process of the Rust core. |

Counts: ANSWERED 9 (6, 7, 10, 11, 14, 15, 23, 24, 26); RECOMMEND 15 (1, 2, 3, 4, 5, 9, 12, 13, 16, 18, 19, 20, 21, 22, 25); NEEDS-USER 2 (8, 17), plus the branch-after-last-ply point inside Q-SE-9.

## 2 Questions that need Brian Prascher's decision

1. **Q-SE-8** archive size limit (restricts DAT-2/IF-5; links to pending decision 9). Also confirm the PGN 10 MB EST (TBD-25).
2. **Q-SE-17** exit codes for TUI/plain signal stops and for non-headless subcommands, and any setup flags beyond CLI-4 (public CLI surface, G3).
3. **Q-SE-9 (h)** what happens to a what-if branch saved from the final position (refuse with a message, or another PGN form).
4. **Interpretation confirmations (no text change; log as one CR entry):** Q-SE-1, 2, 3, 4, 5, 12, 13, 18, 19, 22, 25.
5. **Already yours:** Q-SE-21 is decision 11 (single-instance plugin); Q-SE-20 TBD-18 sample wording needs your review per the register.

## 3 Desk answers

### Q-SE-1 PGN tags that count for a duplicate (CR-4.2)
**(a) Requirements say.** IF-2 (`requirements.md` L1055, amended CR-4): a duplicate is "an imported game whose tags and moves are identical to a game in the store, or to an earlier game in the same file"; prompt offers skip (default) and keep both; replace is never offered. IF-1 (L1031 to L1039) lists the Seven Tag Roster, `SetUp`/`FEN`, `TimeControl`, `Termination`, and custom tags `BlackClass`, `EndStatus`, `Mode`, `Rated`, `WhiteClass` (plus `GameUUID`, `WhiteUUID`, `BlackUUID` only in the archive). DAT-2 (L587) identifies archive collisions by UUID or game id, not by tags. [OBS]
**(b) Already answered?** Partly. "Identical tags" is not defined; the literal reading is every tag. [OBS]
**(c) Recommendation.** Count the Seven Tag Roster values (`Event`, `Site`, `Date`, `Round`, `White`, `Black`, `Result`), plus `SetUp`, `FEN` and `TimeControl` when present, plus the canonical move list. Ignore custom tags, `Termination`, `ECO`, rating tags and any unknown tag. Reasons: the app's own export re-imports as a duplicate whichever custom tags a build writes, and a hand-edited annotation tag does not turn a duplicate into a new game. Same-moves-different-player-names stays distinct because `White`/`Black` are in the set. Edge for the SA: a hash that is NULL until a game is terminal (triage TR-5) means an unfinished stored game never matches an imported copy; acceptable, since the import is a review-only copy. [INF]
**(d) Needs.** Interpretation of IF-2 for Brian to correct (CR entry). No new requirement. It narrows the literal "tags identical", so it is not silent: Brian should see it. Stored format of the hash is the SA's (G3 in triage).

### Q-SE-2 ANA-1/ANA-3 evaluation point of view, start-position evaluation, `Takeback{ply}`
**(a) Requirements say.** ANA-1 (L835 to L849): top N moves with "evaluation (centipawns/mate)"; no point of view stated. ANA-3 (L858 to L866): centipawn loss "from the mover's point of view"; "step through every position with engine evaluation"; amended: plies without evaluation show "evaluating..." with an n-of-N count. DAT-3 (L600): per-move evaluation and centipawn loss, computed after the game. IF-5 (L1093): `games.json` carries per-move evaluations, CPL, depth, engine version. RULE-3 (L447 to L453) fixes what takeback does (undo human move and reply; during a search undo one move; floor at the human's first turn); UC-12 (L29 to L42): `undo` has no target argument. [OBS]
**(b) Already answered?** Mover's-POV for CPL: yes. Display/stored POV, start-position evaluation and `Takeback{ply}` meaning: no. [OBS]
**(c) Recommendation.**
- Store and show evaluations from White's point of view (positive is good for White; mate scores signed the same way), the convention most chess GUIs use. CPL is computed per mover as ANA-3 states. [UNV: GUI convention not checked against a source]
- The start position needs an evaluation to give move 1 a CPL. It may be transient (computed when needed, not stored), because IF-5 round-trips per-move evaluations and each stored CPL already contains its effect. If the SA keeps it transient, the analysis view must show the start position as "evaluating..." until computed and after an archive import. [INF]
- `Takeback{ply}`: the requirements define no target choice, only the RULE-3 outcome. So the core computes the target; the `ply` field is at most a stale-command guard (reject if the client's ply differs from the live ply). No multi-step takeback in one command. [INF]
**(d) Needs.** The POV convention is user-visible, so add one clarifying sentence to ANA-1/ANA-3 by CR (Brian's confirmation). Stored POV is G3 data, covered by AMD-5. No new requirement ID.

### Q-SE-3 Archived Player as a side; `Denied` reasons
**(a) Requirements say.** PLY-2 (L218): archived Players are hidden from the profile list. PLY-3 (L231 to L234): the user chooses among "registered" Players. PLY-4 (L246 to L250): human Players are "deleted by archiving (hidden; history kept)"; archiving is refused while the Player has any unfinished game, the message names the games; the active Player cannot archive itself; computers cannot be renamed, archived, deleted or reset; "an archived Player never holds an unfinished-game slot"; the name stays taken (PLY-6). UI-5: history stays visible. GAME-1 (L343 to L349) and UC-5 (L34): choose "registered humans / computer Players". [OBS]
**(b) Already answered?** Not explicitly. Hidden and "deleted by archiving" imply it, and no text lets an archived Player be picked. [INF]
**(c) Recommendation.** An archived Player is not selectable as a side. The core still validates it, because another front-end can archive between listing and starting (race). The requirement set recognises these refusal situations: (1) archive refused with an unfinished game (maps to `game_open`); (2) computer Player cannot be renamed/archived/reset (`computer_player`); (3) active Player cannot archive itself (`active_player`); (4) slot taken in the mode (GAME-4, setup offers resume/resign/abort); (5) archived Player chosen as a side (`archived_player`). Reason codes are design (architecture 5.5); the situations are in the text for 1 to 4 and implied for 5. [INF]
**(d) Needs.** Interpretation for Brian to confirm. `archived_player` and `game_open` are additive catalogue entries (G2 per triage XC-17). No new requirement.

### Q-SE-4 End-reason precedence and open clock behaviours
**(a) Requirements say.** RULE-1 (L401 to L421): automatic draw at threefold and fifty-move; insufficient material only; checkmate on the fifty-move-completing move stands (CR-2 interpretation, confirmed CR-6.3). RULE-4 (L455 to L462): presets 1+0, 3+2, 5+0, 10+5, 15+10, 30+0 and "custom option (base time + increment)"; Strict only; flag fall loses unless the opponent cannot checkmate (then draw); clock keeps running while the engine is unavailable. GAME-2 / UC-8 3a1: pause freezes moves and both clocks. RULE-2 (L435 to L442): clocks stop while a draw offer is pending. RULE-3 (L447 to L453): takeback during a search cancels it; Training is untimed. PLY-7 (L313): reset Player's views show only games "played after the reset". UC-5 L35: custom base time and increment. [OBS]
**(b) Already answered?** Only checkmate over the fifty-move count. The rest is open, as FIND-DOM-6 says. [OBS]
**(c) Recommendation.**
- **Precedence (e).** Checkmate, stalemate, dead position (insufficient material), threefold, fifty-move, as domain DR-DOM-12. Only checkmate-versus-draw changes the result, and that is stated; the other orders change only the termination label. A move received when the mover's clock is already at zero is a flag result (domain rule, XC-23); a move made with time remaining that mates stands. [INF]
- **(a) Custom clock limits.** Base 1 s to 24 h, increment 0 to 1 h, whole seconds (keeps PGN `TimeControl` representable, IF-1 L1037). Register as an EST. [INF]
- **(b) Who starts.** White's clock runs from game start; Black's from White's first move. Resume from a custom FEN with Black to move starts Black's clock. [INF; FIDE article not checked: UNV]
- **(c) Increment.** Fischer: added after each completed move, including the first. Consistent with the `seconds+increment` PGN form IF-1 adopts. [INF; PGN TimeControl semantics not re-read: UNV]
- **(d) c-c Step.** While paused both clocks are frozen (stated). Step runs the stepping side's clock only while that one move is computed, then freezes again. [INF]
- **(f) Takeback in `EngineError`.** Allow it, treated like a search in flight (RULE-3 L449): it abandons the failed request and returns the human's turn; the retry/exit offer of ENG-4 is dropped with the request. Training is untimed, so no clock effect. [INF]
- **(g) Reset boundary.** A reset voids the Player's unfinished games (PLY-7), so no game of the reset Player spans the boundary. Use the time the game record was created in the store (game start for played games; import time for imported games), compared `>=` the reset time. Do not use the PGN `Date` tag of an imported game. [INF]
**(d) Needs.** The custom-limit values need a new EST entry in the TBD register (proposed TBD-34, owner architect, close by step 1). Everything else is an interpretation for Brian to correct. No requirement text changes.

### Q-SE-5 Termination strings, mate/stalemate custom FEN, ECO for imported custom starts
**(a) Requirements say.** IF-1 (L1036, L1040): `Termination` `normal` for checkmate, stalemate, resignation and agreed or rule-based draws; `time forfeit` for a game lost on the clock; `abandoned` for voided and aborted games; none for an unfinished game. DAT-3 (L602 to L603): termination reasons include resignation, aborted, "voided by reset"; DAT-7: "voided: unreadable"; RULE-2: "draw agreed". IF-3 (L1063): reject malformed or illegal FEN; the list "includes" kings, pawns on rank 1/8, side not to move in check. RULE-8 (L490 to L495): start or analyse from FEN; custom start unrated. DAT-3 (L607): no named position (for example a custom start) gives no ECO code and no name; an imported PGN `ECO` tag is kept, and the name comes from the table. [OBS]
**(b) Already answered?** The PGN mapping, yes. The internal strings, the mate/stalemate FEN and the name for an imported custom start with an `ECO` tag, no. [OBS]
**(c) Recommendation.**
- Keep the domain table of `GAME.termination` strings (checkmate, stalemate, insufficient material, threefold repetition, fifty-move rule, time forfeit, time forfeit with opponent cannot mate, resignation, draw agreed, aborted, voided by reset, voided: unreadable). A draw on the clock where the opponent cannot mate maps to `normal` (a rule-based draw). The strings are shown to users and, because PGN `normal` hides the detail, must travel in `games.json` to meet the DAT-2 round trip, so they are part of format version 1 (IF-5). The SA should freeze the list before the first release. [INF]
- A FEN that is legal but already checkmate or stalemate: reject for starting a game (a new setup error and message, with a new verification case under UC-5 ext 5a); accept for opening an analysis, which only shows the terminal position. IF-3's list says "includes", so it is not exhaustive, but a terminal position is not "illegal" either, so this is an added rule. [INF]
- Imported custom start with an `ECO` tag: keep the file's code (stated); the name stays empty, since the table matches by position and a custom start has none. [INF]
**(d) Needs.** Interpretations for Brian to correct. No requirement text changes. The string list is a stored/exported format item (SA, G3).

### Q-SE-6 Latency requirement for typed-move input
**(a) Requirements say.** NFR-8 (L1013): while an engine search or background analysis runs, every front-end shows visible feedback to a key press, click or tap within 100 ms (EST); 95th percentile; verified by injecting inputs during a maximum-length search. GUI-2(c) and CLI-3: typed SAN commits on Enter. Budget allocation across flush, core, transport and render is a Should (CR-7.4, L41). No other input-latency requirement. [OBS]
**(b) Already answered?** Yes, by NFR-8, for the case that matters. [OBS]
**(c) Recommendation.** No new requirement. A key press is satisfied by the local echo of the typed character, independent of any core round trip. A per-keystroke `ParseMove` query is acceptable if the echo never waits for it; the Enter commit shows a pending state within 100 ms and the loopback round trip is far below that. Prefer query-on-Enter, or a debounce, to avoid one core call per keystroke (design). FF-6 budget: use the NFR-8 100 ms limit as the ceiling, allocated as a Should. The 100 ms is an EST (TBR-2) confirmed by the latency test. [INF; latency unmeasured: UNV]
**(d) Needs.** Nothing. No CR, no TBD.

### Q-SE-7 ECO tags on export
**(a) Requirements say.** TBD-24 (L1201) is CLOSED with the custom tag set `BlackClass`, `EndStatus`, `Mode`, `Rated`, `WhiteClass` (+ UUIDs in the archive). IF-1 (L1039) says the tags follow the Seven Tag Roster in ASCII order. DAT-3 (L607) handles `ECO` on import only. IF-5 (L1093): `games.json` holds stored fields that PGN tags do not carry. [OBS]
**(b) Already answered?** Yes: no `ECO` or `Opening` tag is in the closed export set. [OBS]
**(c) Recommendation.** Do not write ECO tags in PGN export. Re-import recomputes ECO from the position (DAT-3), so the PGN round trip loses nothing. The ECO code and opening name travel in `games.json`. [INF: that `games.json` includes them follows from IF-5's "stored fields that PGN tags do not carry"; the exact field list is the SA's]
**(d) Needs.** Nothing, unless Brian wants ECO tags for other PGN readers: that would be a change to IF-1 and TBD-24.

### Q-SE-8 Import limits
**(a) Requirements say.** IF-2 (L1056): a file larger than TBD-25 is rejected with a message (NFR-9); UC-23 ext 2a. TBD-25 (L1202): open (design), architect's proposal 10 MB, EST. DAT-2 (L584 to L597) and IF-5 set no size limit; UC-25 has no size extension. DAT-2 promises "export all data and import it back". [OBS]
**(b) Already answered?** PGN: yes (reject above TBD-25), value is a design EST. Archive: no. [OBS]
**(c) Recommendation.** Keep the PGN 10 MB EST (architect-owned; no user decision unless it changes visible behaviour). For the archive, a limit conflicts with DAT-2 if it can block a user's own export from being imported. If the base64-in-frame route (triage decision 9, 32 MiB frame cap) is chosen, the usable archive is about 24 MiB [INF: base64 expands by 4/3], which a store of a few thousand games with evaluations might exceed [UNV: no size measurement]. Mitigations: (1) the CLI imports by path with no limit; (2) the GUI export warns when the archive would exceed what the GUI can import; (3) the limit message names the limit, the file size and the CLI alternative; (4) the store is unchanged on rejection (DAT-2). Whether a GUI-only limit is acceptable is a product decision.
**(d) Needs.** **Brian.** Decision 9 (G2) and a clarification that the GUI import path has a size limit, or a requirement that import has none. Add the archive limit as a TBD (proposed TBD-35, EST, architect).

### Q-SE-9 VARIATION shape; reset and saved variations
**(a) Requirements say.** ANA-2 (L851 to L856): play hypothetical moves "on a branch", step back/forward, branch again; "exit discards or optionally saves the branch (PGN variation)". UC-14 step 5 and UC-15 step 5: "save it as a PGN variation". IF-1 (L1042): saved what-if branches are exported as PGN variations. IF-2 (L1052, L1054): variations are accepted and kept, and open in what-if. IF-5: variations stay in `games.pgn`. PLY-7 (L313): "that Player's game history, statistics, W/L/D, and saved variations show only games played after the reset." [OBS]
**(b) Already answered?** The reset question: the wording filters by game. The row shape: the text speaks of saving "the branch" (singular). Not decided for imported nested variations. [INF]
**(c) Recommendation.**
- One `VARIATION` row = one saved line: anchor ply on the main line plus the move list of that line. Save what the user is looking at; that is what UC-14/15 describe.
- Imported nested variations (IF-2 accepts them) can be stored as separate lines, each starting at its own main-line anchor and repeating the parent's prefix. A round trip then yields the same lines but not necessarily the same nesting text. If exact nesting must survive DAT-2's "identical logical contents", a parent link or a tree is needed; the text does not require it. [INF]
- D-26 and `player_id`: PLY-7's wording hides saved variations because the game they belong to is hidden from the reset Player's views. A reset Player cannot open a hidden pre-reset game, so cannot add variations to it. No `player_id` is needed to meet PLY-7. `created_at` remains useful but not required. [INF]
- Related, from UR-19 / FIND-DOM-6(h): a branch started after the last ply is not a variation of any move, so PGN has no place for it. It is reachable (resigned and flag games have a legal next move). Options: refuse to save with a message (NFR-9), or define another form. Refusing limits "optionally saves the branch" in ANA-2. [INF]
**(d) Needs.** Row shape and `player_id`: interpretation for Brian to confirm (no text change). **Branch after the last ply: Brian decides** (it limits ANA-2 or adds an IF-1 form). `VARIATION` columns are a G3 schema item (AMD-5).

### Q-SE-10 Clock-driven search budget
**(a) Requirements say.** RULE-4 verification (L461): a computer does not lose on time in 20 games at each preset (EST, TBD-6) and "each of its moves takes no longer than its clock-derived budget". ENG-3 (L536): no move longer than the configured maximum. UC-6 (L90): think time is the minimum of the Player's limit, the profile's maximum and the clock budget. Test-strategy "Clocks" row: "computer budgets from remaining time". Architecture L609 (min rule). [OBS]
**(b) Already answered?** Intent, yes. No formula exists, by design. [OBS]
**(c) Recommendation.** Confirm the intent as: the budget is an upper limit derived from remaining time and increment; the engine may play faster; it must never take the computer's clock to zero in the RULE-4 test. The engine owner's placeholder `min(remaining/30 + 3*inc/4, remaining/2)` with a 50 ms floor (engine SDD L491) is consistent with that intent. One check for the SDD author: if the 50 ms floor is applied after the `remaining/2` cap, then with under 100 ms remaining the floor exceeds half the remaining time; apply the cap last. [INF; constants unmeasured: UNV]
**(d) Needs.** Nothing. Constants stay design values proven by the RULE-4 test.

### Q-SE-11 Computer draw decision point of view
**(a) Requirements say.** RULE-2 (L429): it accepts a draw offer "only if its own evaluation, from its own side, is <= 0"; L438: "accept only if its evaluation from its own side is <= 0". UC-11 L33 and ext 3b: evaluates "from its own side". [OBS]
**(b) Already answered?** Yes. [OBS]
**(c) Recommendation.** Implement exactly: normalise to the computer's side (after the Q-SE-2 normalisation), accept when the score is <= 0 cp, decline otherwise; a mate score against the computer accepts, a mate for the computer declines. Do not show the evaluation (UC-11 3b1). The triage's phrase "eval 0 or worse" is the same rule.
**(d) Needs.** Nothing.

### Q-SE-12 Engine failure in flight (F-3.7 / ENG-1)
**(a) Requirements say.** ENG-4 (L545 to L552): on crash, no start, readiness over 5 s or no move 2 s past the limit, keep the game resumable and "show a message that names the failure and offers retry or exit"; the clock keeps running; verification: "Retry restarts the engine and play continues". UC-6 ext 5b1 to 5b3 say the same. F-3.7 (`functional_decomp.md` L124): "restart once; keep the game resumable; offer retry or leave". [OBS]
**(b) Already answered?** The requirement and use case govern; F-3.7's "restart once" is ambiguous but not contradicted. [OBS]
**(c) Recommendation.** Confirm: a failure in flight is reported with the ENG-4 message; the search is not repeated automatically; Retry (the next request) restarts the engine and the search is asked again. A crash found while the engine is idle may be restarted transparently before the next request. Read F-3.7's "restart once" as that, and correct its wording by amendment so the app SDD (DD-APP-10, 4.9) and the engine SDD agree. [INF]
**(d) Needs.** Wording amendment to `functional_decomp.md` F-3.7 (SE-owned document; a Decision Log and RELEASE_NOTES entry per CLAUDE.md). No requirement change.

### Q-SE-13 CSP exactness, `localhost`, no-`Origin` requests
**(a) Requirements say.** PLAT-3 (L149): "the Content-Security-Policy (`default-src 'self'`) is set by both the local server and the desktop app, so any accidental remote load fails in tests." NFR-7 (L1012): reject every HTTP and WebSocket request whose `Host` or `Origin` "is not the server's own loopback origin". PLAT-4 (L169): URL printed is `http://127.0.0.1:<port>/`. Architecture L1421: `default-src 'self'; connect-src 'self' ws://127.0.0.1:*`; L684: allow `127.0.0.1:<port>` / `localhost:<port>`. CR-7.7: NFR-7 stays as written. [OBS]
**(b) Already answered?** No for all three. [OBS]
**(c) Recommendation.**
- **CSP.** Treat `default-src 'self'` as a floor: the policy must contain it and must not allow any non-loopback origin. Added directives (such as `connect-src` for the WebSocket) are allowed. The test asserts the floor plus "no remote scheme or host". [INF] Whether `connect-src 'self'` alone covers `ws://` same-origin in all four supported browsers is [UNV]; the architecture's explicit `ws://127.0.0.1:*` avoids relying on it. The Tauri CSP text is a design item [UNV].
- **`localhost`.** Both `127.0.0.1:<port>` and `localhost:<port>` are the server's own loopback origin; the server binds loopback only, and `localhost` is not attacker-controlled. Reject every other Host/Origin, including a rebinding name. [INF]
- **No `Origin`.** Browsers send no `Origin` on a same-origin GET navigation, so rejecting every origin-less request would block loading the page. Rule: `Host` must match on every request; `Origin`, if present, must match; a WebSocket upgrade without `Origin` is rejected (browsers always send it). A local non-browser client can send a forged `Origin`, which is outside the threat model (ASM-1, CR-7.7). Tests that use a non-browser WebSocket client must set `Origin`. [INF]
**(d) Needs.** Interpretations of NFR-7 and PLAT-3 for Brian to correct. No text change. The server SDD should cite this answer.

### Q-SE-14 Port hint
**(a) Requirements say.** DAT-4 (L611 to L617): "the refusal shall name the front-end that has it open"; the rationale example is "this game is open in the CLI". No requirement mentions a port. PLAT-4 limits the port setting to `chess serve`. [OBS]
**(b) Already answered?** Yes: the front-end name is required, a port is not. [OBS]
**(c) Recommendation.** No port hint in v1 and no store column. The refusal names the front-end kind (CLI, desktop, web server). Residual gap: two `chess serve` processes on different ports are indistinguishable by kind; acceptable for a one-person Mac. [INF]
**(d) Needs.** Nothing. If a hint is wanted later, it is a new requirement plus a G3 column.

### Q-SE-15 Who writes `config.toml`
**(a) Requirements say.** PLAT-4 (L154, L159): the port can be set "with a command-line option or in a config file"; `config.toml` in the data directory; a missing file means defaults; an invalid one stops `chess serve` (L162). UI-1 (L716): settings are stored per Player in the data store. NFR-5 (L1010): documentation covers "the `config.toml` keys and their errors". The L165 note: `--config`/`CHESS_CONFIG` are test-only. No requirement has the app write the file. [OBS]
**(b) Already answered?** Yes, by absence plus NFR-5. [INF]
**(c) Recommendation.** State it as hand-edited and read-only in v1. Unknown-key preservation is moot. Strike the architecture sentence "created only when the user saves a setting" (AMD-9). Document the file in NFR-5. [INF]
**(d) Needs.** Nothing at the requirement level.

### Q-SE-16 `functional_decomp.md` F-8.1, F-8.2, F-8.6 to F-8.10
**(a) Requirements say.** `functional_decomp.md`: F-8.1 allocates to three processes (L209); F-8.2 allocates to "axum HTTP + WebSocket" only (L210) although it reads configuration, handles flags and opens the browser; F-8 header (L208) and CR-4.17 (`requirements.md` L1349): F-8.6 to F-8.10 and F-3.9 are enabling-system functions of the build and release view (UC-29). F-8.11 is allocated to "CLI front-end" (L219); F-8.12 to the server (L220). [OBS]
**(b) Already answered?** F-8.6 to F-8.10: yes (no SDD by design). F-8.1 and F-8.2: no one-line split. [OBS]
**(c) Recommendation.** One-line statements. F-8.1: realised by three launch processes (`chess` CLI front-end, `chess serve` as the same binary, `Chess.app` desktop shell); each SDD claims its own launch. F-8.2: split into a CLI half (flags, config read and errors, browser open, signal handling, exit status) and a server half (bind loopback, serve, Host/Origin). The enabling-system blocks are listed once as "out of SDD, covered by UC-29 and scripts". [INF]
**(d) Needs.** An amendment to `functional_decomp.md` (SE-owned, G0, Decision Log and RELEASE_NOTES entry). No requirement change.

### Q-SE-17 CLI scope not in requirements
**(a) Requirements say.** CLI-1 (L891): terminal restored on quit, Ctrl-C, panic, SIGTERM, SIGHUP; unfinished game resumes. CLI-5 (L948 to L955) and TBD-21 (L1198): headless exit codes 0 result, 1 usage or setup, 2 engine unavailable, 3 store error, 4 stopped by a signal. PLAT-4 (L164): `chess serve` stops on SIGINT/SIGTERM with exit 0. PLAT-6: `install-cli` failure says why and changes nothing. CLI-4 (L932, L942, L945): named scripting flags (`--matchup --mode --white --black --clock --no-color`, `--data-dir`); data commands and `--on-duplicate`, `--yes` are design and "their names are public once released". UI-4 (L757) and CLI-1 (L895): plain mode serves screen readers and pipes. Use cases: the "Channels to actors" line of UC-2, 3, 5, 6, 7, 8, 9, 10 to 26, 28 lists "CLI TUI or plain mode"; UC-1, 4, 27, 29 do not. [OBS]
**(b) Already answered?** Plain mode scope: yes in effect, every interactive use case lists it. Exit status for SIGTERM/SIGHUP in the TUI and for subcommands, and further setup flags: no. [OBS/INF]
**(c) Recommendation.**
- **Plain mode.** Reachable in plain mode: every function of the interactive use cases (profile, registration, setup, play, resume, statistics, settings, analysis, import/export, help), as line prompts and commands. The SDD should say so. [INF]
- **Signals.** TUI and plain mode on SIGTERM/SIGHUP: restore the terminal, suspend the game, exit 4 (the CLI-5 class "stopped by a signal"). Do not copy `chess serve`'s exit 0, which PLAT-4 fixes. [INF]
- **Subcommands.** Reuse the TBD-21 classes: 0 success, 1 usage or refusal, 3 store error, 2 engine unavailable where an engine is involved. [INF]
- **Flags.** Add none beyond CLI-4, PLAT-4 (`--port`, `--no-open`), CLI-1 (`--plain`), D-36 (`--yes`, `--on-duplicate`, `--data-dir`). Any other flag is public surface.
**(d) Needs.** **Brian** (CLAUDE.md: G3 covers "public commands/exit codes"; extending code 4 and the 0/1/3 classes beyond headless runs extends CLI-5/TBD-21). Requirement impact: extends CLI-5 scope by CR; no existing code changes meaning.

### Q-SE-18 Name guard at 4096 code points
**(a) Requirements say.** PLY-6 (L291, L299): up to 128 user-perceived characters (grapheme clusters); verification (L305): a 128-cluster name built from 256 code points is accepted, 129 clusters rejected. The rejection message "says which rule failed: empty, too long, not-allowed character, or already used" (L303). [OBS]
**(b) Already answered?** No. A code-point bound is not mentioned. [OBS]
**(c) Recommendation.** Accept a guard at 4096 code points as a robustness bound (a few stacked combining marks per cluster are normal; thousands per cluster are an attack on the segmenter and the UI). Report it as the "too long" rule, so no fifth message is needed. No realistic name is affected. Note the verification case (256 code points) stays inside the bound. [INF]
**(d) Needs.** Interpretation for Brian to confirm. It technically rejects a 128-cluster name over 4096 code points that PLY-6's wording would accept, so record it in the Decision Log. No text change.

### Q-SE-19 DAT-5 "newest backup"
**(a) Requirements say.** DAT-5 (L624): "when no other front-end process is running and the newest backup is older than 24 hours ... it keeps the newest 3 automatic backups and deletes older automatic ones. Backups written before a schema upgrade (DAT-6) or before an import (DAT-2) are never deleted by this rule." The automatic copy is checked to open. DAT-6 (L633 to L637): complete backup before upgrade, verified by opening at the old version. Pre-import backup: DAT-2 L591 ("one exclusive transaction after a backup"). UC-2 ext 2g. [OBS]
**(b) Already answered?** The text says "the newest backup" without qualifying it as automatic, while the retention clause says "automatic". So, no. [OBS]
**(c) Recommendation.** Yes: any complete backup copy of the store that is verified to open counts, whichever event created it. A manual full-data export (DAT-2 archive) is not a store backup and does not count. Consequence: right after an upgrade or import the next automatic backup is not due for 24 h; the pre-upgrade/pre-import copy is complete, so DAT-5's restore offer still has a recent copy. A pre-upgrade copy is at the old schema, so restoring it triggers the migrate step again (R-ST-6 order: backup, migrate, replay). The pre-import backup needs the same "opens" check before it counts. [INF]
**(d) Needs.** Interpretation of DAT-5 for Brian to correct. No text change.

### Q-SE-20 GUI items: bundle budget, TBD-18, key bindings, `BoardProps`
**(a) Requirements say.** No requirement sets a bundle size. NFR-1 (L1006) bounds start-up (3 s; CLI 1 s). TBD-18 (L1195): "Open (design)", owner architect, "with the user's review of the sample wording". UI-6 (L775 to L780): announce moves, check, game end, rejected move without moving focus. GUI-2 (L965 to L981): mouse, keyboard (arrows, Space, Enter), typed SAN, touch; Esc cancels. ENG-5/GUI-4: key F2 and a "Move now" button. UI-9: help lists the screen's key bindings. CLI-4 key rules apply to the TUI. `BoardProps` is internal (UR-52: UF-UI-4). [OBS]
**(b) Already answered?** Mostly: no requirement exists for the bundle; the other items are design values. [OBS]
**(c) Recommendation.**
- **Bundle budget.** No requirement. Set a design ESTIMATE only to protect NFR-1; do not add a requirement. [INF]
- **TBD-18.** Proposed sample wording for Brian's review: move "White knight to f3", capture "Black bishop takes e5", castling "White castles kingside", check "check", mate "checkmate, White wins", rejected move "illegal move: <reason>" (NFR-9 text); SAN text in the move list. [UNV: not tested with VoiceOver]
- **Key bindings.** GUI developer's choice within GUI-2 and UI-9. Constraint: no shortcut may consume a character that begins a typed move while the move input has focus (parallel to CLI-4). F2 and Esc are fixed. Every binding appears in the help (UI-9 test). [INF]
- **`BoardProps`.** Internal, no SE input.
**(d) Needs.** Brian's review of the TBD-18 sample wording (register condition). No requirement change.

### Q-SE-21 Single instance of the desktop app
**(a) Requirements say.** No requirement states single instance (grep of requirements, functional_decomp and use cases for single instance, second launch, already running: only IF-2 matched, unrelated). Related: DAT-1 (concurrent access by all three clients), DAT-4 (one front-end per live game; refusal names the front-end), PLAT-5 (refuse a store or running front-end of another version), DAT-5 and DAT-2 (backup and import run when no other front-end is running). Architecture L1034 (6.3) states "a second launch focuses the existing window" as design; desktop SDD L306 links it to the first-launch roster race. [OBS]
**(b) Already answered?** Yes: it is not a requirement. [OBS]
**(c) Recommendation.** Not a requirement. A second desktop instance is valid under DAT-1/DAT-4 (it can play other games; the live-game lease refuses duplicates). The first-launch roster race must be solved by the store's own locking, not by a UI rule, because the CLI and `chess serve` can race the desktop app anyway. Keeping architecture 6.3's behaviour is a UX choice; the cost is a dependency (triage decision 11, G2). Recommend deciding decision 11 without relying on a requirement: accept the dependency only if Brian wants the focus behaviour for usability. [INF]
**(d) Needs.** Nothing at the requirement level. Decision 11 stays Brian's.

### Q-SE-22 FEN en-passant field on output
**(a) Requirements say.** IF-3 (L1068): valid FENs include en-passant fields; FEN export is out of scope (L1229). IF-1 (L1037): `SetUp "1"` with `FEN` for a custom start. PLAT-1 (L120): the same scripted game gives identical PGN in all three front-ends. RULE-1: threefold repetition. No requirement states the output convention. [OBS]
**(b) Already answered?** No. [OBS]
**(c) Recommendation.** Always write the en-passant target square after a double pawn push (the SA ruling). The output FEN appears only in the PGN `FEN` tag of a custom start and in internal `fen_after`; one code path in `chess-rules` makes PLAT-1 byte-identity hold in every front-end whichever convention is chosen, so no external expectation is created. Keep position identity for repetition separate from the output string: the FIDE repetition rule treats positions as equal only if the same moves are possible, including en passant, so the repetition key should use "capture legal" semantics. [UNV: FIDE article text for this point not read in this pass; TBC-2 covers Art. 9.2 only] Check the FEN/PGN standard text when the rules crate is written, as the triage says.
**(d) Needs.** Nothing at the requirement level; if a user imports a FEN with a different convention it is accepted as given.

### Q-SE-23 Elo chart in the TUI
**(a) Requirements say.** UI-2 (L741): "The TUI draws each scale's history as a line chart in character cells (block or braille glyphs, with an ASCII fallback under `NO_COLOR`/plain-glyph style), next to a table of the latest points ... Plain mode prints the table and the sparkline. The GUI chart has a text or table alternative (UI-4). The chart library is architecture D-21." UC-16 L35, L54 agree. [OBS]
**(b) Already answered?** Yes. [OBS]
**(c) Recommendation.** Confirm: the CLI owns the character-cell renderer (domain exports series data only, per UR-25); the GUI chart is separate. A CLI renderer satisfies the CLI half of UI-2; the GUI half is verified in the GUI suite.
**(d) Needs.** Nothing.

### Q-SE-24 Custom-start ECO and opening name
**SUPERSEDED 2026-10-31 by Brian Prascher's ruling (requirements CR-9.8, FIND-JR-012).** For an imported PGN that is a custom start and carries an `ECO` tag, the file's code is kept and no name comes from the table; a custom start without a tag stays unclassified. The recommendation (c) below, "no ECO and no name" for every custom start, no longer holds for the tagged case. The text below is kept as written. A second ruling the same day (CR-9.11) closes the two edge cases: a custom start that later reaches a named position IS classified, and a FEN equal to the standard start counts as a standard start. The paragraph (c) "do not classify custom starts" and the "standard-start check" reading are replaced accordingly. A third ruling (CR-9.12) decides the tag-versus-table conflict: on a mismatch for any imported PGN with an `ECO` tag, the app asks the player to keep the PGN's ECO or the position classification. CR-9.13 then closes the rest: interactive imports ask per game with apply-to-all; non-interactive imports need `keep-pgn` or `keep-table` or exit 1 with the mismatches listed; the stored name is empty when the PGN's ECO is kept; the choice is not remembered in v1. CR-9.14 (2026-11-01) then decides the points the domain SDD left open as **OQ-DOM-009**: (3) a kept PGN ECO is stored normalised (trimmed, upper-cased) if well-formed (A00 to E99), a malformed tag is treated as no tag, and the stored name is empty; (4) the start position of a custom start (ply 0) is itself looked up in the ECO table. **OQ-DOM-009 (a) and (c) are answered by CR-9.14 (3) and (4).** CR-9.14 (1) also corrects "CLI plain mode" in CR-9.13: plain mode asks when stdin is a terminal.

**(a) Requirements say.** DAT-3 (L607, CR-8): classification is by position, deepest named position reached; "A game with no named position (for example a custom start) has no ECO code and no name." Imported PGN with `ECO` tag: the file's code is kept, the name comes from the table; no tag: the table supplies both. UI-2 marks custom-start games in history. [OBS]
**(b) Already answered?** Yes for the intent. One edge: classification "by position" could in principle match a custom start that later reaches a named position. [INF]
**(c) Recommendation.** Confirm: custom-start games store no ECO and no name, per the DAT-3 example and rules FIND-RUL-7; show "unclassified" (or nothing) in the UI; that string is display wording, not a requirement. Do not classify custom starts even if play transposes into a named position. Standard-start check: a FEN that equals the standard start position counts as a standard start. [INF]
**(d) Needs.** Nothing, apart from recording the "do not classify custom starts" reading in the SE interpretation list if Brian wants it explicit.

### Q-SE-25 Clock units
**(a) Requirements say.** RULE-4 (L459) names presets `1+0, 3+2, 5+0, 10+5, 15+10, 30+0`; CLI-4 example `--clock 5+0`; IF-1 (L1037) `TimeControl` is "`seconds` for sudden death, `seconds+increment`". No requirement text states the unit of the preset numbers (grep for "minutes" in requirements and UC-5/UC-27 found nothing). Domain 3.11.5 (SDD) reads them as minutes plus seconds; XC-23 asks who converts. [OBS]
**(b) Already answered?** No. [OBS]
**(c) Recommendation.** Confirm the chess convention: `base+increment` = base in minutes, increment in seconds (`5+0` is 5 minutes; `3+2` is 3 minutes with 2 s per move). PGN `TimeControl` is in seconds (`300`, `180+2`). The domain owns the conversion (`ClockSpec::from_minutes_plus_seconds`, triage XC-23). The custom clock in the GUI and TUI should ask for minutes and seconds explicitly rather than reuse the `+` shorthand. Document units in help (UI-9) and NFR-5 so the shorthand is not ambiguous. [INF: convention from general chess practice, not from a source]
**(d) Needs.** Interpretation for Brian to confirm; wording clarification of RULE-4/CLI-4 by CR if he wants the unit in the requirement.

### Q-SE-26 "Computer accepts a draw" and "sidecar" wording
**(a) Requirements say.** RULE-2 (L429, L438): the draw rule is "from its own side, <= 0". ENG-1 review note (L509): "Stockfish runs only as a bundled native child process of the Rust core (architecture §2, D-9)"; IF-4 (L1071): bundled with every entry point; PLAT-1 (L117): `Chess.app` contains the `chess` CLI and Stockfish. The word "sidecar" does not appear in the requirements (grep). Triage UR-37: "sidecar" means a bundled helper binary only; `chess-engine` spawns `<exe dir>/stockfish`. [OBS]
**(b) Already answered?** Yes. [OBS]
**(c) Recommendation.** Confirmed: neither phrase changes a requirement. The SDD wording "Computer accepts a draw" is a shorthand for RULE-2; "sidecar" is the Tauri name for the bundled binary and is consistent with ENG-1 and IF-4 as long as the Rust core, not the shell, spawns it (a shell-spawn path would be a G2 design change, not a requirement change).
**(d) Needs.** Nothing.

## 4 Proposed follow-up entries (not applied)

| Item | Kind | Trigger | Needs Brian |
|---|---|---|---|
| Interpretation list for a future CR-9 (Q-SE-1, 2, 3, 4, 5, 12, 13, 18, 19, 22, 25) | Decision Log / CR | After Brian's review | Confirm or correct |
| Clarify ANA-1/ANA-3 evaluation point of view | CR amendment | Q-SE-2 | Yes |
| New EST: custom clock limits (proposed TBD-34) | TBD register | Q-SE-4 | Confirm |
| New EST: archive import size limit (proposed TBD-35), and the DAT-2 reading | TBD register + CR | Q-SE-8 | Yes |
| Extend CLI-5 / TBD-21 scope to TUI signals and subcommands | CR amendment | Q-SE-17 | Yes |
| Branch-after-last-ply rule for ANA-2/IF-1 | CR amendment | Q-SE-9 (h) | Yes |
| F-3.7, F-8.1, F-8.2 wording | `functional_decomp.md` amendment (G0) | Q-SE-12, 16 | No (SE-owned) |
| Strike "created only when the user saves a setting" | Architecture AMD-9 | Q-SE-15 | Per SA triage |
| New verification case: terminal-position FEN rejected for a new game | Test strategy / UC-5 | Q-SE-5 | No |

## 5 Assumptions, open items, not verified

**Assumptions.** A-01: the architecture text quoted here is accepted design but ranks below the requirements (CLAUDE.md precedence). A-02: ratings, evaluations and journals have no released data, so G3 format choices are still cheap (triage ASM-T1). A-03: one person per Mac account (ASM-1, CR-7.7) bounds the NFR-7 reading.

**Open items.** TBD-34 (custom clock limits) and TBD-35 (archive limit) proposed; the PGN 10 MB (TBD-25) and TBD-18 remain open (design); the `games.json` field list and the termination string list are SA items.

**Not verified.** FIDE Laws text for clock start, repetition with en passant and flag-versus-mate precedence; PGN Standard text for `TimeControl` increment semantics; browser support of `connect-src 'self'` for same-origin `ws://`; VoiceOver behaviour of the sample wording; archive and latency sizes; the contents of SDDs beyond the cited passages; UC-7, 8, 10, 13, 17 to 21, 28, 29 beyond greps.

**Next step.** Brian reviews section 2. Then the SE drafts CR-9 (interpretations plus the three amendments in section 4); the architect applies the SDD edits listed in the triage.

## 6 Change log

| Version | Date | Change |
|---|---|---|
| 0.1 DRAFT | 2026-10-31 | First desk answers for Q-SE-1 to Q-SE-26. |
| 0.1 DRAFT, approver updated | 2026-10-31 | Approver field and this log updated to record Brian Prascher's resolution (requirements Amendment 7, CR-9). The answers above are kept as written. In CR-9 the TBD proposed here as TBD-34 and TBD-35 are numbered TBD-34 and TBD-35, and the deferred Q-SE-17 is TBD-36. |
| 0.1 DRAFT, Q-SE-24 superseded | 2026-10-31 | Note added at Q-SE-24 and in the answer table: superseded by Brian Prascher's ruling on the joint review (requirements CR-9.8). The original answer text is kept. |
| 0.1 DRAFT, Q-SE-24 edge cases closed | 2026-10-31 | Q-SE-24 note extended for requirements CR-9.11 (custom start reaching a named position is classified; standard-start FEN is a standard start). Original text kept. |
| 0.1 DRAFT, Q-SE-24 / Q-SE-5 mismatch ruling | 2026-10-31 | Q-SE-24 note extended for requirements CR-9.12 (ECO tag versus table mismatch is flagged and the player chooses). The Q-SE-5 wording "keep the file's code, name empty" is superseded for the mismatch case only. Original text kept. |
| 0.1 DRAFT, Q-SE-24 / Q-SE-5 CR-9.13 | 2026-10-31 | Q-SE-24 note extended for requirements CR-9.13 (ECO mismatch handling decided; D-59 accepted). Original text kept. |
| 0.1 DRAFT, Q-SE-24 / OQ-DOM-009 CR-9.14 | 2026-11-01 | Q-SE-24 note extended for requirements CR-9.14: OQ-DOM-009 (a) and (c) are answered by ruling (3) (normalise a kept tag, malformed tag is no tag, empty name) and ruling (4) (ply-0 lookup of a custom start). Original text kept. |
