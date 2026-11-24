# Decision package: CI product and action pinning (chess-a0r / bead chess-85m)

Prepared by: software-engineer (Engineer stance), 2026-11-09. Gate: G0 (decision input, no code or doc edited). Evidence tags: `[RAN]`, `[OBS]`, `[INF]`, `[UNV]`.

## 1. Where this stands

- Brian's relayed decision of 2026-11-06, recorded as a bead comment `[OBS bd show chess-85m]`: **keep GitHub Actions; SHA-pin actions before the first CI run.** The bead is still OPEN; its acceptance criterion is "Brian decides CI product and pinning; workflow run once".
- So the open work is: choose the exact SHAs, and run the workflow once. This package asks Brian to ratify the pin values, and offers the alternatives in case he wants to revisit the product choice.
- Facts `[OBS]`: one workflow, `.github/workflows/ci.yml` (on push to `main` and on pull requests; `permissions: contents: read`; one job on `macos-latest`). It uses `actions/checkout@v4` and `actions/setup-node@v4` by tag, then `cargo install cargo-deny --version 0.20.2` and `ci/check.sh`. Remote is `https://github.com/bprascher/chess.git`. The workflow has never run `[UNV]` (step1-scaffolding-evidence.md, finding 2).
- `ci/check.sh` is the real gate (fmt, build, clippy, test, crate-edge check, ECO checksums, cargo-deny, npm audit) and runs identically locally; `CHESS_CI_OFFLINE=1` skips the network parts. A local offline run was green in the step-1 evidence `[OBS step1-scaffolding-evidence.md section 3]`; not re-run for this package.

## 2. Options

| | A. GitHub Actions, actions pinned by SHA (current decision) | B. Local-only `ci/check.sh` as the gate | C. A plus B: local script is the gate of record, Actions is a second opinion | D. Other CI (self-hosted runner, other SaaS) |
|---|---|---|---|---|
| Fits offline-first, single developer | Partly: needs GitHub and network, but the product itself stays offline | Best fit; no external service | Good | Poor: more to run and secure |
| Independent clean-machine check | Yes (fresh macOS runner) | No; passes only prove "works on my Mac" | Yes | Yes |
| Supply-chain exposure | Third-party actions run with repo checkout; mitigated by SHA pins and `contents: read` | None added | As A | Larger |
| Cost / effort | Free minutes on public repos; macOS minutes on a private repo are billed at a multiplier `[UNV, check plan]`; upkeep of pins | Zero | A plus discipline | High |
| Enforcement | Can be a required check on `main` (needs branch protection, `[UNV]` on this plan) | Honour system; nothing blocks a merge | Local gate plus optional required check | Varies |
| Intel / other macOS coverage | Runner is arm64 `macos-latest` `[UNV]`; no Intel run either way | Dev Mac only | Same | Possible, at cost |
| Reversibility | Two-way: workflow is one file | Two-way | Two-way | Two-way, more rework |

All options are two-way doors `[INF]`.

## 3. Pin proposal (for A and C)

Read-only `git ls-remote` on 2026-11-09 `[RAN]`. The tags below are lightweight (no peeled `^{}` line was returned for them), so the SHA is the commit `[INF]`.

| Action | Tag in use today | SHA that tag resolves to | Newer tags exist |
|---|---|---|---|
| `actions/checkout` | `v4` = `v4.4.0` | `11d5960a326750d5838078e36cf38b85af677262` | `v5.1.0`, `v6.1.0` (`d23441a48e516b6c34aea4fa41551a30e30af803`) |
| `actions/setup-node` | `v4` = `v4.4.0` | `49933ea5288caeca8642d1e84afbd3f7d6820020` | `v5.0.0`, `v6.5.0` (`249970729cb0ef3589644e2896645e5dc5ba9c38`) |

Proposed diff (not applied; the workflow is not a baselined document but the change is Brian's to approve):

```diff
-      - uses: actions/checkout@v4
+      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0
...
-      - uses: actions/setup-node@v4
+      - uses: actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020 # v4.4.0
```

Pinning v4.4.0 changes no behaviour (same code the tag already points to). Moving to v6 might be wise if v4 is deprecated on the runner; I did not check release notes or runtime deprecation `[UNV]`. Also consider `runs-on: macos-15` (or the current named image) instead of `macos-latest`, since "latest" moves under the gates; the available image names were not checked `[UNV]`. The SHAs were obtained from the same network source as the tags; Brian can confirm them on the actions' release pages (a SHA lookup by tag trusts GitHub, the same trust the action has).

Also noted, not part of the decision: `cargo install cargo-deny` without `--locked` is deliberate (known vulnerabilities in the published lock, step-1 lookup F5), takes build time each run, and fetches from crates.io `[OBS ci.yml]`.

## 4. Recommendation

**Option C, with the pins in section 3 (v4.4.0 SHAs), and `ci/check.sh` stated as the gate of record.**
Reasons: it honours Brian's 2026-11-06 decision (Actions, SHA-pinned); the script already runs locally so a single developer is never blocked by GitHub; the hosted run adds the only clean-machine check available, and the supply-chain risk is small with `contents: read` and SHA pins. Revisit if macOS minutes cost money or the run is flaky.

Consequences:
- Pins go stale. Someone (Brian or an agent) must bump them deliberately; Dependabot for actions is an option and would be a new file `[UNV, not set up]`.
- Until the first hosted run, the workflow is unproven: expect first-run fixes (toolchain install from `rust-toolchain.toml`, cargo-deny install time, npm audit against the registry, which has never been run for real `[UNV]`).
- Local green is not a release proof; hosted green still does not cover Intel or other macOS versions.
- If Brian picks B: delete or archive `ci.yml`, and record that no independent check exists; "never run" findings then close as not applicable.

## 5. What Brian decides

1. Confirm option C (or choose A, B, D).
2. Approve the pin values in section 3, or ask for v5/v6 (then I re-resolve and you re-confirm).
3. Whether to name a macOS image instead of `macos-latest`.
4. Authorise the first hosted run (a push or PR to GitHub, which is a side effect I will not do myself) and who applies the diff. After the run, the bead can be closed on the evidence of the run URL and result.
5. Whether to make the check required on `main` (branch protection), if the plan allows.
