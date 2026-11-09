# ECO opening data: source record

Data for `chess-rules` (docs/architecture.md D-20, D-75; requirements DAT-3, NFR-6, PLAT-3).
The five files are a one-time snapshot, copied unchanged. They are never downloaded at run time or at build time.

| Field | Value |
|---|---|
| Upstream | https://github.com/lichess-org/chess-openings ("Chess opening names") |
| Commit | `3ba6b20d31b2e6125cb0e6a2e792f400717d9d39` (master head on the retrieval date; commit message "Merge pull request #417 from jdart1/C12-fix", authored 2026-11-05T21:00:07Z) |
| Retrieved | 2026-11-06 (UTC), with `curl` from `https://raw.githubusercontent.com/lichess-org/chess-openings/<commit>/<file>` |
| Files | `a.tsv` `b.tsv` `c.tsv` `d.tsv` `e.tsv` (columns `eco`, `name`, `pgn`; one header line), and the upstream licence text `COPYING.txt` |
| Licence | CC0 1.0 Universal (public domain dedication). The upstream README says: "As a collection of facts, this data set is in the public domain. Considerable effort was spent curating and cleaning the data. Insofar as that qualifies for copyright, the work is released under the CC0 Public Domain Dedication." The full legal text is `COPYING.txt` (upstream file `COPYING.txt` at the same commit) |
| Checksums | `SHA256SUMS` (SHA-256 of each file as retrieved). Verified in CI by `ci/check-eco.mjs` (CI-RUL-001); `build.rs` does not verify them |

Retrieval record `[RAN]`: `GET https://api.github.com/repos/lichess-org/chess-openings/commits/master` returned the commit above; each file was fetched with `curl -sSf` at that commit; `shasum -a 256` produced `SHA256SUMS`. Line counts: a.tsv 824, b.tsv 782, c.tsv 1251, d.tsv 645, e.tsv 368 (3870 lines, 3865 data rows after five header lines).

To refresh the snapshot: fetch the five files at a new commit, replace them, regenerate `SHA256SUMS`, update this file, and record it as a decision (the table is the input of every game's opening classification).
