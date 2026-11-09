# SE confirmation of the chess-domain mode-policy readings (bead chess-g1h, condition C-D4 chess-1vm)

| Field | Value |
|---|---|
| ID | SE-CONF-POLICY-1 |
| Version | 1.0 (draft for Brian Prascher) |
| Date | 2026-11-09 |
| Owner | systems-engineer agent, Reviewer/analyst stance |
| Baseline | B4 |
| Scope | Requirement intent only. No code, SDD or requirement was edited. Nothing here is a decision; items for Brian are listed in section 4. |

Evidence tags: `[OBS]` read in a file this session, `[INF]` my inference, `[UNV]` not verified.

## 1 Sources read

`[OBS]` docs/requirements.md (PLY-5, GAME-1..4, RULE-2, RULE-3, RULE-4, ANA-1..4, ENG-4, ENG-5, Decision Log lines on Strict analysis and clocks); docs/architecture.md 4.4, 4.5, 4.7 and the protocol command list (line 817); SDD-chess-domain 3.10, 3.11.4, 3.11.5; desk-review-G1-step1.md D3; step1-domain-conditions-evidence.md section 4; crates/chess-domain/tests/data/policy_independent_table.txt; review-chess-domain-code.md DF-08; `GameCommand::EngineFailed` in crates/chess-domain/src/game.rs:973-984; the (f) scenario test.

## 2 Verdict table

Governing text is requirement IDs first, then architecture. "Silent" means no requirement states the point, so the verdict rests on the architecture text and on consistency with requirement intent. Where silent, the "wording needed" column is a recommendation, not a decision.

| Item | Verdict | Requirement / architecture refs | Reason (one line) | Wording needed |
|---|---|---|---|---|
| (a) `LiveAnalysis` on a finished game is `game_over` (`voided` if Voided); `PostGameAnalysis` and `WhatIf` exempt | CONFIRMED WITH WORDING CHANGE | ANA-1 (during play, Training only), ANA-3 and ANA-4 (after the game: PostGameAnalysis and what-if), arch 4.5 (`LiveAnalysis` "Training only"; `WhatIf` "Training live, or any finished game") | Live analysis is defined only during play; the post-game need is met by the two exempt purposes, so refusing the live kind after the end loses no required capability | SDD 3.11.5 last row says "except analysis"; the Voided row says "except `PostGameAnalysis` and `WhatIf`". Make the last row identical to the Voided row |
| (b1) `Suspended` is not InProgress: Rewind refused there | CONFIRMED | GAME-3 ("during play"), arch 4.4 line 714 (a suspended game is not open in any session), SDD 3.11.4 (Rewind "in progress") | Rewind is a client view of an attached game; a suspended game has no viewer | Define "in progress" vs "unfinished", see F-3 |
| (b2) `LiveAnalysis`, Training `WhatIf` refused in `Suspended` (`state_not_allowed`) | CONFIRMED | ANA-1 (human's turn, during play), ANA-4 ("Unfinished games are resumed, not analysed"), arch 4.5 | A suspended game is unfinished and must be resumed first | None beyond F-3 |
| (b3) Strict + `Suspended` for `Takeback`, `LiveAnalysis`, `WhatIf` reports `strict_mode` (PD-3) | CONFIRMED WITH WORDING CHANGE | PLY-5 (Strict: no analysis during play, "only after the game ends"; takeback "never"), RULE-3, arch 4.4 ("until the game ends"), arch 4.5 | The Strict prohibition is mode-based and lasts until the game ends, so it holds in `Suspended`; `strict_mode` is the accurate reason. No requirement fixes the reason code | SDD cells "strict_mode (S, in progress)" should read "(S, game not finished)". Arch 4.5 "in-progress Strict game" should say "unfinished" |
| (c) Human `MakeMove` reports `not_human_turn` before `state_not_allowed` | CONFIRMED (requirements silent) | RULE-2 ("No move can be made while it is pending"), GAME-3, SDD 3.11.5 row lists `not_human_turn` first. No requirement orders reasons | The more specific cause (wrong side or computer side) is shown first; a move by the right human in a wrong state still gets `state_not_allowed` | State the order in one sentence in 3.11.5 (recommendation) |
| (d) `SelectClock` allowed in any non-finished Strict game | NOT CONFIRMED | GAME-1 (clock collected at game start), RULE-4 (clock chosen up front; Strict only), PLY-5 (mode "fixed for the game"), arch 4.4 clock row, protocol command list line 817 (no select-clock command) | No requirement lets a running game change its clock, no command or fact exists for it, and arch 4.4 says front-ends render controls from `allowed_actions`, so the reading advertises a control that does nothing. The "Strict only / `training_untimed`" part is confirmed | See F-1 and recommendation R-1 |
| (e) `SetMoveDelay` allowed in any non-finished c-c state | CONFIRMED WITH WORDING CHANGE | GAME-2 (watcher can set the delay; no state limit), SDD 3.11.4 (delay is session state) | GAME-2 states no state condition; a delay change while Paused or in EngineError is harmless and useful. In `Suspended` there is no watcher session (app SDD line 423 routes it through the `GameSession`), so the case is allowed by Policy but cannot arrive | Say in 3.11.5 "any non-finished c-c state, including Suspended; the app refuses it when no session is attached". Silent; recommendation |
| (f) `EngineFailed` drops a draw offer made to a computer | CONFIRMED WITH WORDING CHANGE | RULE-2 ("If the engine fails before it answers, the offer is dropped and ENG-4 applies"; verification "I: fake engine that fails, the pending offer is dropped and the engine-failure message appears"), ENG-4, arch 4.4 in-play rules ("An engine failure drops the offer (ENG-4)") | Required by requirement. Implementation `[OBS]`: drop to `AwaitingMove`, clocks restart, no `offer_blocked_for`, and `EngineFailed` in an h-h `DrawOffered` is refused | Add `DrawOffered --EngineFailed--> AwaitingMove` (offer to a computer) to the T4 list and diagram, and say the app emits `EngineUnavailable` (ENG-4 message). See F-5 |
| PD-1 `MakeMove{Engine}` with `by` not the side to move | CONFIRMED WITH WORDING CHANGE (silent; recommendation) | Arch 4.4 and 3.11.5 name `by` rules only for the human actor. ENG-4 and arch 4.5 (illegal engine output is a failure) | No requirement covers it. The engine's move is always for the side to move; anything else is an internal fault, never user-visible. `not_human_turn` would be wrong (the actor is not human) | Recommend 3.11.5 say: Engine `MakeMove` needs `by` = side to move, a computer, in `EngineThinking`; otherwise `state_not_allowed`. Resolves A-MM7 (`deny`) and A-MM8 (`?`) to `state_not_allowed` |
| PD-2 `RespondDraw` by the wrong side or wrong actor kind | CONFIRMED WITH WORDING CHANGE (reason silent) | RULE-2 (offer to a human: that human accepts or declines; offer to a computer: the computer answers by its own evaluation, cannot be withdrawn), arch 4.4 draw rows and the five `draw_*` reasons (all for offers) | The refusals themselves are required: the offerer must not answer its own offer and a human must not answer for a computer. No requirement or reason code covers the response, so `state_not_allowed` (the row's only listed reason) stands | 3.11.5 should say that a wrong answerer or actor kind gets `state_not_allowed`. Do not add a new reason (see FOR BRIAN 3) |
| PD-3 Strict + `Suspended` reason | See (b3) | | | |
| PD-4 Precedence of `voided` and `game_over` rows over per-action reasons | CONFIRMED WITH WORDING CHANGE (silent; recommendation) | Arch 4.4 (`GameContext` carries game state); SDD 3.11.5 last two rows ("Any action on ..."); ANA-4 | A finished or voided game dominates: "game is over" is the correct and more useful answer than a mode or turn reason. Requirements do not order reasons | Add one sentence to 3.11.5: global rows are tested first (`voided` for Voided, `game_over` for Completed, Finalized, Aborted), except for `PostGameAnalysis` and `WhatIf`; then per-action rules in table order |

Summary: of the 11 distinct rows (PD-3 duplicates b3), 3 are CONFIRMED ((b1), (b2), (c)), 7 are CONFIRMED WITH WORDING CHANGE ((a), (b3), (e), (f), PD-1, PD-2, PD-4), and 1 is NOT CONFIRMED ((d), in-progress part). Only (d) conflicts with requirement intent. Requirement text is unchanged by any verdict.

## 3 Findings

Format: ID | Severity | Rule or characteristic | Evidence | Impact | Recommended action.

| ID | Severity | Rule / characteristic | Evidence | Impact | Recommended action |
|---|---|---|---|---|---|
| F-1 | Minor | Reading contradicts requirement intent (GAME-1, RULE-4, PLY-5; 29148 necessity and consistency) | Table row FD6d-1 and `Policy::check` (`SelectClock => allow_if(strict, TrainingUntimed)`, policy.rs:325) allow `SelectClock` in A, ET, EE, DO, P, SU. No clock-change command exists in the protocol list (arch line 817); `GameSetup` carries the clock and T1 starts it at `Game::new` `[OBS]` | `allowed_actions` of an in-progress Strict snapshot includes `SelectClock`, so a front-end that follows arch 4.4 must draw a control with no command behind it; also the desk review's alternative ("no ply yet") is no better, since the clock is already running after `Game::new` (T1) | Recommendation R-1. Severity Minor: no behaviour path is wrong today, because no command exists |
| F-2 | Minor | Consistency of the analysis rules (ANA-1 rationale, ANA-2) | Table row A-WI3: Training `WhatIf` is `ok` in state `A,ET,EE,DO,P` with a human to move. In an h-c Training game `DrawOffered` has the offering human as side to move, so what-if is allowed there `[INF from the table; implementation agrees per section 4 of the evidence]`, while `LiveAnalysis` needs `AwaitingMove`. ANA-1 gives the reason for human's-turn-only: analysis "never competes with the computer's own search"; during `DrawOffered` the computer is running `ComputerDrawDecision` | A what-if search can start while the draw decision is pending (shared analysis instance, interactive lane). ANA-2 ties what-if to analysis ("Training mode during play") but does not list states, so requirement intent is not violated outright | Recommendation R-2. Not one of the readings (a) to (f); raised because it sits in the same table |
| F-3 | Minor (wording) | Unambiguous terms (INCOSE Guide to Writing Requirements: defined terms) | Arch 4.4 line 688 ("whether the game is still in progress"), arch 4.5 ("in-progress Strict game"), SDD 3.11.4 ("Rewind ... in progress"), and the state `InProgress` (3.10) are used for three different sets. Reading (b) needs `Suspended` to be unfinished but not `InProgress` | A reader can take "in progress" in 4.4/4.5 to include `Suspended` (so (b3) `strict_mode` follows) or to mean `InProgress` only (so PD-3 could read `state_not_allowed`) | Use "unfinished" for A..SU and keep `InProgress` for the 3.10 super-state only |
| F-4 | Editorial | Consistency | SDD 3.11.5: row "Any action on a Voided game ... except `PostGameAnalysis` and `WhatIf`" vs row "Any action on a Completed, Finalized or Aborted game ... except analysis" | "Analysis" could be read to include `LiveAnalysis` | Make both rows list the two kinds |
| F-5 | Minor (open intent) | Completeness of ENG-4 handling | RULE-2 says the dropped offer is followed by "ENG-4 applies" (message, retry or exit). Implementation drops straight to `AwaitingMove` with no `EngineError` state `[OBS]` | If "retry" is meant to be `RetryEngine`, it is not offered in `AwaitingMove`. My reading `[INF]`: retry means the human offers again (RULE-2 blocks a repeat offer only after a decline, and a drop is not a decline), and the game stays resumable. The app must raise `EngineUnavailable` so the message appears | SE intent: accept this reading. The app SDD must show the message and treat re-offering as the retry; confirm with the UI owner [TBC] |

### Recommendations (not decisions)

- **R-1 (F-1).** Make `SelectClock` a setup-time check only: in any existing game state it is not an allowed in-game action (omit from `allowed_actions`, `state_not_allowed` if sent), and keep `check_setup` as the single owner of the clock rules (`training_untimed`, `clock_invalid`). If Brian prefers to keep the kind in `Policy` for the setup screen, it should be evaluated against a setup context, not a game state.
- **R-2 (F-2).** Restrict Training `WhatIf` on a live game to `AwaitingMove` with a human to move, matching `LiveAnalysis`. Finished games stay unconditional (ANA-4).
- **R-3.** Apply the wording items in the table (3.11.5 precedence sentence; PD-1, PD-2, PD-3 sentences; "unfinished" term; identical exemption rows; T4/diagram addition for (f); (e) Suspended sentence).

## 4 FOR BRIAN (decisions that are not mine)

1. **(d)/F-1/R-1:** whether `SelectClock` leaves the in-game policy (changes baselined arch 4.4 clock row reading and SDD 3.11.5, so an amendment with a Decision Log and RELEASE_NOTES entry). Gate: G1 if it stays inside `chess-domain` and the SDD text; G2 if the protocol or arch 4.4 table changes.
2. **F-2/R-2:** whether Training what-if is allowed while a draw offer to the computer is pending. This is a requirement-intent call on ANA-1/ANA-2; I recommend no.
3. **PD-2:** whether to add a response-specific denial reason. Recommendation: no; an additive reason touches the protocol (arch 5.6) and the five `draw_*` reasons already name offer problems only.
4. **Disposition of DF-08 and C-D4:** this confirmation closes the "SE confirmation" half of C-D4 only if Brian accepts the verdicts above and the SDD wording amendment (A-D3 plus the items here). The (d) verdict means the implementation and the independent table share a reading I do not confirm, so the table row FD6d-1 and the policy code would need a change if R-1 is adopted. I make no change.

## 5 Assumptions

- A-01 `[INF]` Requirement intent is read from the AGREED/amended text in docs/requirements.md as it stands at B4; the Decision Log was read only by search on the policy topics.
- A-02 `[INF]` Reason codes (`strict_mode`, `state_not_allowed`, `not_human_turn`, `game_over`, `voided`) are architecture and SDD vocabulary; no requirement names them, so verdicts on reasons rest on user-facing sense and architecture consistency.
- A-03 `[INF]` A computer never answers a draw offer by command; it answers through the engine path (arch 4.4, RULE-2).

## 6 Open items and not verified

- `[UNV]` docs/test-strategy.md and the UI SDD were not read; I did not check whether any test or UI text depends on `SelectClock` in game or on what-if during `DrawOffered`.
- `[UNV]` I did not run any test or re-run the independent table; the "agrees" statements about implementation are the evidence file's `[RAN]` claim plus my reading of `game.rs:973-984` and `policy.rs:325`.
- `[UNV]` Full decision table `Policy::check` body was not audited; I judged the table rows and the readings listed only.
- `[UNV]` No full read of docs/requirements.md; sections relevant to the readings only. Requirements outside these (for example UI-level wording on controls) could add constraints.
- `[UNV]` TBC for F-5 (UI owner): whether re-offering is an acceptable "retry" after a dropped offer.
- `[UNV]` Review-independence: this note is the SE's confirmation of intent; it does not replace the Reviewer's re-check of the domain rework.

## 7 Next step

Brian rules on FOR BRIAN items 1 to 4. Then the SDD wording amendment (A-D3 plus this note's items) and, if R-1 or R-2 are adopted, a G1 code change to `policy.rs` and the independent table with the evidence re-run. The SE note becomes the cited confirmation in the C-D4 closure.
