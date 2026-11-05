# OQ-16 limited re-review (systems engineer, requirements and verification side)

| Field | Value |
|---|---|
| ID | OQ16-RR-SE |
| Task | chess-a0r.1.34 (CR-3 task T-C2), baseline B3 |
| Date | 2026-11-04 |
| Stance | Reviewer, fresh invocation; did not author any reviewed text |
| Scope (Brian's cap) | Are the nine Majors closed by the changed text; defects introduced by the changed text. No full review pass |
| Decides | Brian Prascher. This report approves nothing and closes no gate. OQ-16 stays open |
| Evidence tags | OBS read; RAN executed; INF inferred; UNV not verified |

## 1. Verdict

**PASS WITH CONDITIONS.** Majors not closed: 0. Majors pending Brian: 3 limbs (M-1, the D-78 limb of M-4, the NewGame.started_at field of F-02). New Majors: 0. New findings: 9 Minor, 1 Info.

## 2. Per-finding verdicts

| Bead | Finding | Verdict | Evidence |
|---|---|---|---|
| .1.21 | F-01 writer after failed append | CLOSED (verifiability; crash mechanics are the architect reviewer's) | D-82 architecture.md:2070; UT-ST-087, CT-ST-087 (store SDD 1005, 1011), UT-APP-35 (app 1326): each has a concrete expected result and traces to NFR-2 [OBS] |
| .1.22 | F-02 one start stamp | PENDING BRIAN (field only) | Invariant, row-without-journal path, UT-ST-089 (store 1007) applied. UT-APP-36 (app 1327) defers the GAME.started_at equality to OQ-26; D-84 row marks the mechanism PROPOSED, G2 [OBS] |
| .1.23 | F-03 rebase | CLOSED | D-83; UT-ST-088 (deterministic, byte-for-byte, seq continuity), CT-ST-088 (kill at four named failpoints, one damaged file, idempotent second restart) (store 1006, 1012) [OBS] |
| .1.24 | M-1 D-78 / DAT-3 | PENDING BRIAN | Not applied, as intended. D-78 row says NOT RATIFIED (architecture.md:2066); CR-11 states no choice made (requirements.md:1422); OQ-28 open. See section 4 on the proposal |
| .1.25 | M-2 ASM-10 | CLOSED | ASM-10 closed on design text: domain 3.11.12 (line 683), UT-DOM-014 (1029), DR-DOM-004 (1168) [OBS]. Runtime fact stays UNV and is stated as such (architecture.md:2700). XC-DOM-A8-1 and XC-APP-A8-3 re-labelled "Applied, open" (domain 1232; app 1516) [OBS]. The closure evidence is the same design that created the assumption (see RR-S-8) |
| .1.26 | M-3 D-74 case | CLOSED | UT-APP-38 (app 1329) and CT-APP-34 (1363): live snapshot, get_game fallback, unknown game and store error give Denied or Internal with 0 engine queries, stale-by-one-handler, no-provider purposes. Traced to ANA-1, PLY-5, D-74; cited in DD-APP-11 (1117) and the matrix (1382, 1408) [OBS]. This meets the desk-review condition C-3 |
| .1.27 | M-4 ratification | CLOSED for D-79 and D-80; D-78 limb PENDING BRIAN | D-79 and D-80 "RATIFIED by Brian, 2026-11-04 (chess-a0r.1.30)" in architecture.md:2067-2068, CR-11 item 2, RELEASE_NOTES.md:7, domain 1233, app 10. D-78 is "ruled under Brian's delegation, NOT RATIFIED" everywhere I checked. Nothing claims G-2 passed as a review, OQ-16 closed, or D-78 ratified (grep of added lines, RAN) |
| .1.28 | JR5-001 DurabilityUnsupported | CLOSED | Exit class 3, "accepted by Brian, G3": CLI A3 row (cli 398), DD-CLI-2 (440), UT-CLI-08 (689, includes the code, headless fixed), UT-CLI-018, CT-CLI-021, E2E-CLI-22; UI DD-UI-016 and UT-UI-014, CT-UI-018; Tauri 3.7 and CT-TAU-021; server 5.3 and CT-SRV-037. All referenced IDs exist (RAN grep). No new code number [OBS] |
| .1.29 | JR5-002 FF-3 / FF-4 | CLOSED | architecture.md:1781-1782: FF-3 states its ctx.wall_ms precondition, the voided-hash exemption and a restore clause; FF-4 limited to completed or aborted games, voided excluded, known gap OQ-27 stated. Tests CT-APP-18, UT-APP-26, UT-APP-16, UT-ST-089 [OBS] |

Requirement-text integrity [RAN git diff -U0 docs/requirements.md]: seven hunks only (header row, change-history row, RULE-8 sub-line, NFR-3 verification cell, Decision Log row CR-11). No shall-statement, ID or TBD status changed. DAT-3 untouched. functional_decomp F-3.9 is wording only.

## 3. New findings (changed text only)

| ID | Severity | Rule | Evidence | Impact | Action |
|---|---|---|---|---|---|
| RR-S-1 | Minor | Bidirectional traceability (se-rvtm) | Requirement-to-test matrices lag the new tests: store NFR-2 row (store 968) omits UT-ST-083, 085 to 089, CT-ST-085, 087, 088; app NFR-2 (1412) lists only CT-APP-04; app RULE-8 (1395) omits UT-APP-29; app and domain DAT-3 rows (app 1400; domain 1148) omit UT-APP-26, 32, UT-DOM-122, 123 | Forward trace exists in each test row's trace column, but an RVTM reader sees NFR-2 crash evidence missing | Add the IDs to the matrices (G0) |
| RR-S-2 | Minor | Verification case fits requirement | RULE-8 line (requirements.md:510) cites UT-ST-084, which tests CHECK rejections, not the four-start table. Its fourth row (imported game with standard FEN tag) sits beside OQ-27, which is open (G3) | Weak trace; pass criterion may shift when OQ-27 is decided | Cite UT-DOM-115 and UT-APP-29 only, or note OQ-27 dependence |
| RR-S-3 | Minor | Measurable criterion | NFR-3 pass criterion (requirements.md:1036) gives no case count; no SDD sets one (grep, RAN) | "No panic in any generated case" is not decidable until N is fixed | Fix N in an SDD test; name owner and date |
| RR-S-4 | Minor | Internal consistency | E2E-CLI-22 (cli 716) asserts TUI exit 3; cli 388 says the TUI row "neither decides nor narrows TBD-36" | A test asserts what the text calls a placeholder | Reword one of them |
| RR-S-5 | Minor | Open item needs owner and date (principle 3) | CR-11 item 9 (requirements.md:1422): no date, "no TBD minted" | Open item can drift | Mint a TBR with a close-by date |
| RR-S-6 | Minor | Status accuracy | CR-11 and RELEASE_NOTES say store, app, domain are at 0.4.2; app is 0.4.3 (app 6). architecture.md:21 says "addenda A to D below" though E exists. Domain Approver row says the revision "adds no new decision" while 0.4.2 carries D-85, D-87 | Version and scope claims off | Correct the three statements |
| RR-S-7 | Minor | Truthful claims | D-78 (3) and domain 3.11.13 say the unfinished turn is lost "because no record carries it". The proposal's own model shows the wall_ms gap carries it for a clean stop (1359 of 1359) | Brian reads D-78 with a statement the proposal contradicts | Add the qualification to the D-78 note |
| RR-S-8 | Minor | Evidence tagging | Proposal section 3: `[RAN]` model is a simulation of assumed records; the crash result (0 of 1321) is true by construction; script is not committed (82 lines, text says 70). I re-ran it from scratchpad and the table reproduces [RAN]. "Often time the player was away" (section 4) is untagged INF. ASM-10 closure is design text closing a design assumption | Evidence reads stronger than it is | Tag as simulation; commit or attach the script |
| RR-S-9 | Minor | Consistent status | RELEASE_NOTES.md:39 (older section) still says "Brian may reverse D-78 to D-80" with no supersession note; architecture 2362/2469 do carry one | Reader of the older section sees D-79, D-80 as unratified | Add a pointer |
| RR-S-10 | Info | Truthful status | FF-3 (architecture 1781) asserts started_at equality for a restore without marking OQ-26 pending; store minors D-78 (5) and CT-ST-086 trace to unratified D-78 | If Brian picks B or C, (5) must be re-homed | Mark both |

## 4. The D-78 / DAT-3 proposal

It reads DAT-3 honestly: it admits option A needs a shall-text amendment (a CR), states the loss is unbounded in time, quotes the premise failure of ASM-10, and marks the sleep behaviour UNV. The recommendation A (confidence Medium) follows from the cost table, but the evidence in section 3 points at a fourth path it does not offer Brian as a choice: narrowing for a clean stop at no format cost. Writing the exclusion into DAT-3 would later forbid that narrowing. Brian should see that trade.

## 5. Not checked

Crash and ordering mechanics (second invocation). Rules, engine SDDs. Mermaid rendering of changed diagrams. Beads records (Brian's rulings taken from document text, UNV). Full architecture text outside the changed hunks. No tests or code exist to run.

## Assumptions, open items, next step

Assumption: the uncommitted working tree is baseline B3. Open: OQ-26, OQ-27, OQ-28, TBD-36, NFR-2 / D-71 constraint question. Next step: Brian decides OQ-26 and OQ-28; author fixes RR-S-1 to RR-S-9 (G0 text).
