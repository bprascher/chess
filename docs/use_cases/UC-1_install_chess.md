# UC-1: Install Chess

| Document | Version | Date | Status | Owner | Approver |
|---|---|---|---|---|---|
| docs/use_cases/UC-1_install_chess.md | **0.3 (draft)** | 2026-10-28 | **DRAFT. Not baselined.** Derived from requirements v1.0 Amendment 3 (CR-5), architecture v1.0 amendment 3 and functional_decomp v0.3 | Systems engineer role (Claude), for Brian Prascher | Brian Prascher (user): approval **required**, not given |

**Goal in Context:** The User wants Chess.app and the `chess` command installed on the Mac, as one version, so that all three entry points can be used.

**Scope:** Chess application as delivered (bundle `Chess.app`, the `chess` CLI binary inside it, and the bundled Stockfish). Black box, delivery view.

**Level:** User goal

**Primary Actor:** User, acting as installer and operator

**Supporting Actors:** None.

**Stakeholders and Interests:**
- User: a working install by following the documented steps, with no Internet needed to run it.
- Project owner: one application version for the CLI and the desktop app (PLAT-5), unsigned local build (PLAT-1).

**Preconditions:** The Mac runs macOS Tahoe or a later supported release (PLAT-2). The `Chess.app` bundle is available on the Mac (it is built locally from source by the User; v1 has no installer or download, PLAT-6).

**Minimal Guarantees:** No partial install leaves the CLI and desktop app at different versions with a shared store in use: a mismatched front-end refuses to use the store (PLAT-5).

**Success Guarantees:** `Chess.app` is installed, `chess` is on `PATH` through a symlink, and both belong to one version.

**Trigger:** The User wants to install or update Chess.

**Main Success Scenario:**

1. User reads the install and run steps (NFR-5).
2. User places `Chess.app` in a folder of the User's choice, by default `/Applications` (PLAT-6).
3. User runs `chess install-cli` from the bundle (PLAT-6). The System creates the `chess` link on `PATH` and prints the link's path.
4. User starts one entry point (UC-2).
5. System checks that the entry point, the data store and any other running front-end belong to its own version (PLAT-5) and shows the profile list.

**Extensions:**

- **2a.** The Mac does not meet PLAT-2 (unsupported macOS):
  - 2a1. macOS itself refuses to open the bundle, because the bundle declares macOS 26 as its minimum (PLAT-6 (e)). The SuD adds no check. Use case fails.
- **2b.** macOS blocks the first launch because the copy came from another Mac or a download:
  - 2b1. User opens the app once by the steps in the install documentation (PLAT-1, NFR-5). The SuD adds no workaround. Use case resumes at step 3.
- **3a.** The symlink cannot be created (for example the target exists or permission is denied):
  - 3a1. System says why, changes nothing, and tells the User that the CLI still runs by its full path (PLAT-6 (b), NFR-9). A file that is not a link made by `install-cli` is never replaced. Use case fails until the User resolves it.
- **3b.** `/usr/local/bin` does not exist or is not writable:
  - 3b1. System puts the link in `~/.local/bin` (created if absent) and, when that folder is not on `PATH`, prints the line to add to the shell profile (PLAT-6 (b)). Use case resumes at step 4.
- **5a.** The store or another running front-end belongs to a different version:
  - 5a1. System refuses to use the store and shows a message naming both versions and telling the User to install the matching version (PLAT-5, NFR-9).
  - *Then:* Use case fails; store bytes are unchanged.
- **\*a.** The install is an update over an earlier version:
  - \*a1. User quits every running front-end and any `chess serve`, then replaces `Chess.app` with the new version, so CLI and desktop update together (PLAT-5, PLAT-6 (c)). User runs `chess install-cli` again only if the bundle path changed. The store is upgraded by UC-2 extension 2d. There is no update check (PLAT-3).
- **\*b.** The User uninstalls:
  - \*b1. User runs `chess install-cli --remove`, which removes only the link it made, then deletes `Chess.app`. The data directory stays. The documentation warns that deleting it removes every Player, game and rating, and advises exporting all data first (UC-24) (PLAT-6 (d), NFR-5). Use case ends.

**Technology and Data Variations:**
- Desktop packaging is a local build, ad-hoc signed, with no Developer ID and no notarisation (PLAT-1, TBC-10). A locally built app opens normally; a copy that macOS has quarantined needs the one-time step in extension 2b.
- Universal build: one `Chess.app` runs on Intel and Apple silicon (PLAT-2, D-25).
- Install layout: `Chess.app` is the only artifact; it contains `chess` and Stockfish (D-18).

**Special Requirements** (rules that live in the requirements, linked here, not restated):
- PLAT-3: installing and running need no Internet connection after the bundle is on the Mac.
- NFR-6: licence compliance is a release activity, not a User step (UC-29).

**Related Information:**

| Item | Value |
|---|---|
| Priority | Must (every agreed requirement is Must, requirements Q56). Use case ranking: TBD |
| Frequency | TBD (no usage data exists) |
| Performance target | None of its own. System-wide limits NFR-1 and NFR-8 apply. |
| Channels to actors | Finder or shell, and the install documentation. |
| Includes (`<<include>>`) | None |
| Extends (`<<extend>>`) | None |
| Extended by / included by | UC-2 follows |
| Open issues | None. UG-4 closed 2026-10-28 (CR-4). (register: [README.md](README.md) section 8) |

**Traceability:**

| Kind | IDs |
|---|---|
| Requirements | PLAT-1, PLAT-2, PLAT-5, NFR-5, PLAT-3, PLAT-6 |
| Functions (functional_decomp v0.3) | F-8.1, F-8.5, F-8.6, F-8.8, F-8.11 |
| Architecture | See requirements allocation, architecture 10.1. |

## Change log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-10-27 | First draft. |
| 0.2 | 2026-10-28 | CR-4: install source, `chess install-cli`, update, uninstall, blocked app and unsupported macOS decided (UG-4 closed); PLAT-6 added to the trace. |
| 0.3 | 2026-10-28 | Consistency pass (requirements Amendment 3, CR-5): baseline references updated to architecture amendment 3 and functional_decomp v0.3; version aligned across the set. |
