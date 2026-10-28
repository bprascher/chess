# UC-11: Offer Draw

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-11_offer_draw.md | **1.0** | 2026-10-28 | **Baselined v1.0, 2026-10-28** (approved by the user). Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v1.0 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): **approved 2026-10-28** |

**Goal in Context:** The User wants to propose a draw to the opponent, so that a game with no winning chances can end by agreement.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a human Player

**Supporting Actors:** Stockfish engine (block `stockfish`, bundled and pinned; driven over UCI)

**Stakeholders and Interests:**
- Offering Player: a clear answer.
- Human opponent: may accept or decline.
- Computer opponent: accepts only when its own evaluation says it is not better (RULE-2).

**Preconditions:** A human–human or human–computer game is in progress, in either mode. It is the offerer's turn to move, and no search or other offer is pending. A computer never offers a draw.

**Minimal Guarantees:** The game continues unchanged if the offer is declined or cancelled.

**Success Guarantees:** If accepted, the game ends as a draw (½-½) with the reason agreement, and is finalised as in UC-6 step 8.

**Trigger:** The User enters `draw` (CLI) or uses the draw control (GUI).

**Main Success Scenario:**

1. User offers a draw.
2. System stops all clocks, blocks moves while the offer is pending, and passes the offer to the opponent: a human is asked on the same screen, a computer is asked by evaluating the position from its own side within its maximum think time (RULE-2, ENG-3).
3. Opponent accepts: a human says so, a computer accepts because its evaluation is zero or worse.
4. System ends the game as a draw (1/2-1/2) with the reason "draw agreed" and finalises it.
5. System offers full-game analysis (UC-15).

**Extensions:**

- **1a.** It is not the offerer's turn, a search is running, an offer is pending, or the offerer's last offer was declined and the offerer has not moved since:
  - 1a1. System refuses the offer with the reason (RULE-2, NFR-9). Use case fails.
- **2a.** The engine is unavailable when a computer is asked:
  - 2a1. System drops the offer and continues as UC-6 extension 5b (ENG-4).
- **3a.** The human opponent declines:
  - 3a1. The clocks restart with the same side to move, and the game continues. The offerer can offer again only after completing another move (RULE-2). Use case ends.
- **3b.** The computer's evaluation from its own side is better than zero:
  - 3b1. System declines for the computer and does not show the evaluation (F-3.8). The clocks restart and the game continues. Use case ends.

**Technology and Data Variations:**
- Human–human: the other human decides; in hot-seat either human may offer.
- Human–computer, Training or Strict: computer rule applies.
- Computer–computer: no draw offer (policy, architecture 4.4).

**Related Information:**

| Item | Value |
|---|---|
| Priority | Must (every agreed requirement is Must, requirements Q56). Use case ranking: TBD |
| Frequency | TBD (no usage data exists) |
| Performance target | None of its own. System-wide limits NFR-1 and NFR-8 apply. |
| Channels to actors | CLI TUI or plain mode, browser tab, or desktop window. |
| Includes (`<<include>>`) | None |
| Extends (`<<extend>>`) | UC-6, UC-7 |
| Extended by / included by | None |
| Open issues | None. UG-6 closed 2026-10-28 (CR-4). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | RULE-2, PLY-5, ENG-4 |
| Functions (functional_decomp v1.0) | F-2.5.2, F-3.8, F-3.3, F-2.6.5 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: draw-offer rules decided (RULE-2, UG-6 closed). |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
| 1.0 | 2026-10-28 | Baselined: approved by the user. Content unchanged from 0.3; references to functional_decomp updated to v1.0. |
