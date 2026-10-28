# UC-26: Recover from Damaged Data Store

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-26_recover_from_damaged_data_store.md | **1.0** | 2026-10-28 | **Baselined v1.0, 2026-10-28** (approved by the user). Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v1.0 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): **approved 2026-10-28** |

**Goal in Context:** The User wants to get Chess working again when the data store fails its integrity check, losing as little as possible.

**Scope:** Chess application (the system under discussion, SuD): CLI front-end, GUI (browser tab and desktop webview), `chess-app` core and the data store, as named in the architecture context diagram. Black box.

**Level:** User goal

**Primary Actor:** User, acting as a human Player

**Supporting Actors:** None.

**Stakeholders and Interests:**
- User: the data is recovered from the latest backup, and the damaged file is not destroyed (DAT-5).
- Project owner: no silent data loss.

**Preconditions:** Chess is starting (UC-2) and the store fails the integrity check, or the store is missing.

**Minimal Guarantees:** The damaged file is left unmodified (DAT-5).

**Success Guarantees:** A usable store is in place (restored from the latest backup, new, or imported) and the User continues at UC-2 step 3.

**Trigger:** System detects a failed integrity check at start-up (UC-2 extension 2c).

**Main Success Scenario:**

1. System tells the User that the store is damaged and leaves the damaged file unmodified.
2. System offers to restore the most recent backup.
3. User confirms the restore (UI-12).
4. System restores the backup into a usable store, replays every move journal that is still held over it (the journal wins), and finalises again, in order of end time, each game that finished after the backup (DAT-5, NFR-2).
5. System reports how many games and moves were recovered from journals, and continues start-up (UC-2 step 3).

**Extensions:**

- **2a.** No backup exists:
  - 2a1. System offers to start fresh, or to import from a file (UC-25) (F-6.7, D-17). Use case resumes at step 3 with the User's choice.
- **5a.** A game that finished after the backup involves a Player who is not in the restored store (for example, registered after the backup):
  - 5a1. System cannot finalise that game. It keeps the game's journal, does not delete it, and the report names the game (DAT-5, architecture R-19). v1 does not recover that game. Use case resumes at step 5.
- **3a.** The User declines the restore:
  - 3a1. System leaves the damaged file and does not open the store. Use case fails.
- **4a.** The restore fails:
  - 4a1. System shows the failure (NFR-9), writes it to the local diagnostic log (NFR-10, F-6.14) and leaves the damaged file and the backup unmodified. Use case fails.

**Technology and Data Variations:**
- The "most recent backup" is normally an automatic one, written at start-up when the newest was older than 24 hours (F-6.15, DAT-5, TBD-33), or the copy taken before a schema upgrade or an import. Changes other than games, made after that backup (registration, rename, reset, settings), are lost with the damaged store (DAT-5).
- The store is also "missing" in some flows (F-6.7); for a clean first launch see UC-2 extension 2a.

**Related Information:**

| Item | Value |
|---|---|
| Priority | Must (every agreed requirement is Must, requirements Q56). Use case ranking: TBD |
| Frequency | TBD (no usage data exists) |
| Performance target | None of its own. System-wide limits NFR-1 and NFR-8 apply. |
| Channels to actors | CLI TUI or plain mode, browser tab, or desktop window. |
| Includes (`<<include>>`) | None |
| Extends (`<<extend>>`) | UC-2 |
| Extended by / included by | None |
| Open issues | None. UG-21 closed 2026-10-28 (CR-4). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | DAT-5, DAT-6, NFR-2, NFR-9, UI-12, NFR-10 |
| Functions (functional_decomp v1.0) | F-6.6, F-6.7, F-6.3, F-6.14, F-6.15 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: journals are replayed after a restore (DAT-5, UG-21 closed); diagnostic log (NFR-10), confirmation (UI-12). |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. Restore limit (extension 5a) and automatic backup (F-6.15) added. |
| 1.0 | 2026-10-28 | Baselined: approved by the user. Content unchanged from 0.3; references to functional_decomp updated to v1.0. |
