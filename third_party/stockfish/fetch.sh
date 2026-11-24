#!/usr/bin/env bash
# third_party/stockfish/fetch.sh - obtain the pinned Stockfish 19 as a macOS universal binary
# and record the facts ASM-3 needs (chess-a0r.3.3; architecture 8.2 "Stockfish", D-9).
#
# Location: architecture 3.1 names third_party/stockfish/ for "pinned version, build script,
# LICENSE, source offer"; 8.2 names this file fetch.sh.
#
# THE LIVE RUN NEEDS THE INTERNET ONCE (SWE-001). Tell Brian before running without --dry-run.
#
# Usage:
#   third_party/stockfish/fetch.sh --dry-run          print the plan; no network, no writes
#   third_party/stockfish/fetch.sh                    live run, verified against PIN_ASSET_SHA256
#   third_party/stockfish/fetch.sh --trust-on-first-use
#                                                     live run while the pin in use is empty (see below)
#   third_party/stockfish/fetch.sh --source           fallback (8.2 step 2): build from source (UNTESTED)
#   --force      ignore cached downloads and extractions
#   -h|--help
#
# Checksum policy (fail closed):
#   * PIN_ASSET_SHA256 is the recorded SHA-256 of the release archive and is SET (confirmed by
#     Brian Prascher against the release page, 2026-11-09). A live run computes the SHA-256 of the
#     archive and, on any mismatch, exits 2 and leaves nothing extracted. This is the ONLY
#     integrity check the script enforces.
#   * PIN_SOURCE_SHA256 is the same kind of pin for the tag SOURCE archive, used only by --source
#     (the untested fallback build). It is EMPTY [UNV] because the source path has not been used;
#     --source refuses (exit 4) until a human sets it after a manual comparison.
#   * PIN_COMMIT is the sf_19 tag commit. It is documentation only and is NOT enforced: nothing
#     in this script compares a download or a checkout against it (no network check).
#   * --trust-on-first-use is the bootstrap for an EMPTY asset pin (not needed while it is set,
#     as now). It still refuses unless the computed SHA-256 equals the "digest" the GitHub release
#     API reports for the same asset. That is two answers over TLS from the same origin, weaker
#     than a signature, and not a substitute for the human check against the release page. A
#     human then copies the printed SHA-256 into the pin as a reviewed diff.
#
# Safety: no sudo; writes only under third_party/stockfish/build/ (untracked, see .gitignore
# beside this file); the archive is listed and rejected if it has absolute or ".." paths before
# extraction; the downloaded binary is only run after the checksum passes.
#
# Exit codes: 0 all facts recorded; 2 checksum mismatch; 3 incomplete facts (e.g. not universal,
# no COPYING, no UCI_Elo line); 4 refused: pin empty / cannot cross-check; 1 other error.
#
# Needs: bash 3.2+, curl, tar, shasum, lipo, jq, sed, awk, find (all present on macOS with
# Xcode Command Line Tools; jq ships with macOS 15+). The default path needs NO compiler.
# --source additionally needs Xcode CLT (clang++, make) and builds both slices; see build_source.

set -euo pipefail
umask 077

# ---------------------------------------------------------------- pins (edit via reviewed diff)
PIN_TAG="sf_19"                       # architecture D-9; release published 2026-09-05 per requirements TBD-26
# sf_19 tag commit (lightweight tag; git ls-remote 2026-11-09), confirmed by Brian Prascher 2026-11-09 (relayed by coordinator).
# DOCUMENTATION ONLY, NOT ENFORCED: the script never compares anything against it. The asset SHA-256 below is the only check.
PIN_COMMIT="edb0d9db6731067ec50ce619ff372b463bc4dd5d"
PIN_REPO="official-stockfish/Stockfish"
ASSET_NAME="${SF_ASSET_NAME:-stockfish-macos-universal.tar.gz}"   # name per architecture 8.2 [UNV: not checked against the release]
PIN_ASSET_SHA256="a1f0e3bcc5a6927a11fe6fc8e54a779754645f3c2bae2cf13420fd1957adaa77"  # stockfish-macos-universal.tar.gz; confirmed by Brian Prascher against the release page, 2026-11-09 (relayed by coordinator)
PIN_SOURCE_SHA256=""                  # [UNV] SHA-256 of the tag source archive; used only by --source; empty = --source refuses
SF_ARM_ARCH="${SF_ARM_ARCH:-apple-silicon}"        # [UNV] Stockfish Makefile ARCH names; check `make help` in the source
SF_X86_ARCH="${SF_X86_ARCH:-x86-64-sse41-popcnt}"  # [UNV]
# -------------------------------------------------------------------------------------------

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HERE="$ROOT/third_party/stockfish"
BUILD="$HERE/build"
DL="$BUILD/download"
EX="$BUILD/extract"
OUT="$BUILD/out"
RES="$BUILD/results.txt"

REL_URL="https://github.com/${PIN_REPO}/releases/download/${PIN_TAG}/${ASSET_NAME}"
API_URL="https://api.github.com/repos/${PIN_REPO}/releases/tags/${PIN_TAG}"
SRC_URL="https://github.com/${PIN_REPO}/archive/refs/tags/${PIN_TAG}.tar.gz"
GIT_URL="https://github.com/${PIN_REPO}.git"
COPYING_RAW_URL="https://raw.githubusercontent.com/${PIN_REPO}/${PIN_TAG}/Copying.txt"

DRY=0; TOFU=0; SOURCE=0; FORCE=0
for a in "$@"; do
  case "$a" in
    --dry-run) DRY=1 ;;
    --trust-on-first-use) TOFU=1 ;;
    --source) SOURCE=1 ;;
    --force) FORCE=1 ;;
    -h|--help) sed -n '2,44p' "$0"; exit 0 ;;
    *) echo "unknown argument: $a" >&2; exit 1 ;;
  esac
done

say()  { printf '%s\n' "$*"; }
die()  { printf 'ERROR: %s\n' "$*" >&2; exit "${2:-1}"; }
note() { printf '%s\n' "$*" | tee -a "$RES" >/dev/null; printf '%s\n' "$*"; }

if [ "$SOURCE" = 1 ]; then
  PIN_FOR_ARCHIVE="$PIN_SOURCE_SHA256"; ARCHIVE_URL="$SRC_URL"; ARCHIVE_FILE="$DL/${PIN_TAG}-source.tar.gz"
else
  PIN_FOR_ARCHIVE="$PIN_ASSET_SHA256"; ARCHIVE_URL="$REL_URL"; ARCHIVE_FILE="$DL/$ASSET_NAME"
fi

# ---------------------------------------------------------------- dry run
if [ "$DRY" = 1 ]; then
  say "DRY RUN: no network access, no files written."
  say "Mode            : $([ "$SOURCE" = 1 ] && echo 'source build fallback (8.2 step 2, UNTESTED)' || echo 'official release asset (8.2 step 1, preferred)')"
  say "Repo / tag      : $PIN_REPO @ $PIN_TAG   (tag commit, documentation only, not enforced: ${PIN_COMMIT:-<EMPTY [UNV]>})"
  say "Archive URL     : $ARCHIVE_URL"
  say "Release API URL : $API_URL   (cross-check of the asset digest)"
  say "COPYING fallback: $COPYING_RAW_URL   (only if the archive has no COPYING file)"
  say "Output dir      : $BUILD   (untracked; nothing is written outside the repo)"
  say "Archive SHA-256 pin: ${PIN_FOR_ARCHIVE:-<EMPTY [UNV]>}"
  if [ -z "$PIN_FOR_ARCHIVE" ]; then
    say "Checksum handling: pin is empty, so a live run WITHOUT --trust-on-first-use downloads,"
    say "                   prints the computed SHA-256, and exits 4 before extracting."
    say "                   WITH --trust-on-first-use it proceeds only if the computed SHA-256 equals"
    say "                   the digest the release API reports for the asset; otherwise exit 4."
  else
    say "Checksum handling: computed SHA-256 must equal the pin, else exit 2."
  fi
  say "Steps           : 1 download (curl, https only)  2 SHA-256 check  3 tar listing safety check"
  say "                  4 extract  5 locate Mach-O 'stockfish'  6 lipo -info / -archs (need arm64 AND x86_64)"
  say "                  7 uci / setoption probe (UCI_Elo min/max)  8 read GPL version from COPYING"
  say "                  9 list *.nnue files  10 SHA-256 of the binary  11 write $RES"
  say "Tool presence (informational):"
  for t in curl tar shasum lipo jq sed awk find git; do
    if command -v "$t" >/dev/null 2>&1; then say "  ok      $t"; else say "  MISSING $t"; fi
  done
  if [ "$SOURCE" = 1 ]; then
    for t in clang++ make; do
      if command -v "$t" >/dev/null 2>&1; then say "  ok      $t (needed by --source)"; else say "  MISSING $t (needed by --source; install Xcode CLT)"; fi
    done
  fi
  say "Live-run command (tell Brian first; Internet needed once):"
  say "  $HERE/fetch.sh$([ -z "$PIN_FOR_ARCHIVE" ] && echo ' --trust-on-first-use')$([ "$SOURCE" = 1 ] && echo ' --source')"
  exit 0
fi

# ---------------------------------------------------------------- live run
[ "$(uname -s)" = "Darwin" ] || die "macOS only (lipo, Mach-O)"
for t in curl tar shasum lipo jq sed awk find; do command -v "$t" >/dev/null 2>&1 || die "missing tool: $t"; done
[ "$(id -u)" -ne 0 ] || die "do not run as root"

mkdir -p "$DL" "$OUT"
: >"$RES"
note "# Stockfish ${PIN_TAG} fetch results ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
note "mode: $([ "$SOURCE" = 1 ] && echo source || echo release-asset)"
note "archive_url: $ARCHIVE_URL"
note "host: $(uname -m) macOS $(sw_vers -productVersion 2>/dev/null || echo '?')"

sha256_of() { shasum -a 256 "$1" | awk '{print $1}'; }

curl_get() { # url dest
  curl --proto '=https' --tlsv1.2 -fsSL --retry 2 --max-time 600 -o "$2.part" "$1" || { rm -f "$2.part"; return 1; }
  mv "$2.part" "$2"
}

# 1. download (idempotent: reuse a cached archive only if it still passes the checks below)
if [ ! -s "$ARCHIVE_FILE" ] || [ "$FORCE" = 1 ]; then
  say "Downloading $ARCHIVE_URL"
  curl_get "$ARCHIVE_URL" "$ARCHIVE_FILE" || die "download failed (asset name/URL may differ from architecture 8.2 [UNV]; list assets at $API_URL)"
else
  say "Using cached $ARCHIVE_FILE"
fi

# 2. checksum, fail closed
GOT="$(sha256_of "$ARCHIVE_FILE")"
note "archive_sha256: $GOT"
if [ -n "$PIN_FOR_ARCHIVE" ]; then
  if [ "$GOT" != "$PIN_FOR_ARCHIVE" ]; then
    note "checksum: MISMATCH (pin $PIN_FOR_ARCHIVE)"
    die "archive SHA-256 differs from the pin; nothing extracted" 2
  fi
  note "checksum: matches pin (source of pin: the reviewed value in fetch.sh)"
else
  if [ "$TOFU" != 1 ]; then
    note "checksum: pin EMPTY [UNV]; refusing to extract. Compare the value above with the release page, then rerun with --trust-on-first-use."
    die "pin empty and --trust-on-first-use not given" 4
  fi
  [ "$SOURCE" = 0 ] || die "--trust-on-first-use has no independent digest for tag source archives; set PIN_SOURCE_SHA256 by hand after a manual comparison" 4
  say "Fetching release metadata for the digest cross-check"
  curl_get "$API_URL" "$BUILD/release-$PIN_TAG.json" || die "cannot fetch release metadata" 4
  API_DIGEST="$(jq -r --arg n "$ASSET_NAME" '[.assets[]? | select(.name==$n) | .digest // empty][0] // empty' "$BUILD/release-$PIN_TAG.json")"
  API_DIGEST="${API_DIGEST#sha256:}"
  note "release_asset_names: $(jq -r '[.assets[]?.name] | join(", ")' "$BUILD/release-$PIN_TAG.json")"
  note "release_api_digest: ${API_DIGEST:-<none reported>}"
  [ -n "$API_DIGEST" ] || die "release API reports no digest for $ASSET_NAME; verify by hand, set the pin, rerun" 4
  [ "$API_DIGEST" = "$GOT" ] || die "download SHA-256 differs from the release API digest" 2
  note "checksum: TOFU accepted; equals the release API digest (two TLS answers, no signature). PIN_ASSET_SHA256=$GOT must now be reviewed into fetch.sh."
fi

# 3. archive safety: no absolute or parent-relative members
tar -tzf "$ARCHIVE_FILE" >"$BUILD/archive-listing.txt" || die "not a readable tar.gz"
if grep -Eq '(^/|(^|/)\.\.(/|$))' "$BUILD/archive-listing.txt"; then die "archive has absolute or .. paths; rejected" 2; fi
note "archive_members: $(wc -l <"$BUILD/archive-listing.txt" | tr -d ' ') (listing: $BUILD/archive-listing.txt)"

# 4. extract into a fresh directory
rm -rf "$EX"; mkdir -p "$EX"
tar -xzf "$ARCHIVE_FILE" -C "$EX" --no-same-owner --no-same-permissions

build_source() {
  # UNTESTED [UNV]: architecture 8.2 step 2. Builds each slice, joins with lipo. The Stockfish
  # Makefile arch names, the cross-compile flag handling and net embedding are NOT verified.
  command -v clang++ >/dev/null && command -v make >/dev/null || die "--source needs Xcode CLT (xcode-select --install)"
  local srcdir; srcdir="$(find "$EX" -maxdepth 2 -type d -name src | head -n 1)"
  [ -n "$srcdir" ] || die "no src/ directory in source archive" 3
  ( cd "$srcdir" && make -j"$(sysctl -n hw.ncpu)" build ARCH="$SF_ARM_ARCH" COMP=clang && mv stockfish "$OUT/stockfish-arm64" && make clean \
    && make -j"$(sysctl -n hw.ncpu)" build ARCH="$SF_X86_ARCH" COMP=clang CXXFLAGS="-arch x86_64" LDFLAGS="-arch x86_64" && mv stockfish "$OUT/stockfish-x86_64" ) \
    || die "source build failed (see make output; arch names and flags are [UNV])" 3
  lipo -create "$OUT/stockfish-arm64" "$OUT/stockfish-x86_64" -output "$OUT/stockfish"
}

# 5. locate the binary
BIN=""
if [ "$SOURCE" = 1 ]; then
  build_source; BIN="$OUT/stockfish"
else
  while IFS= read -r f; do
    if file -b "$f" | grep -q 'Mach-O'; then BIN="$f"; break; fi
  done < <(find "$EX" -type f -perm -u+x -iname 'stockfish*' | sort)
  [ -n "$BIN" ] || die "no Mach-O stockfish executable found in the archive (see $BUILD/archive-listing.txt)" 3
  chmod u+x "$BIN"
fi
note "binary_path: ${BIN#"$ROOT"/}"
note "binary_sha256: $(sha256_of "$BIN")"

STATUS=0

# 6. lipo (TBC-7)
LIPO_INFO="$(lipo -info "$BIN" 2>&1 || true)"
LIPO_ARCHS="$(lipo -archs "$BIN" 2>&1 || true)"
note "lipo_info: $LIPO_INFO"
note "lipo_archs: $LIPO_ARCHS"
case " $LIPO_ARCHS " in
  *" arm64 "*) HAS_ARM=1 ;; *) HAS_ARM=0 ;;
esac
case " $LIPO_ARCHS " in
  *" x86_64 "*) HAS_X86=1 ;; *) HAS_X86=0 ;;
esac
if [ "$HAS_ARM" = 1 ] && [ "$HAS_X86" = 1 ]; then
  note "TBC-7: universal confirmed (arm64 and x86_64 both listed)"
else
  note "TBC-7: NOT universal. Architecture 8.2 step 2 applies: rerun with --source (fallback), and open the risk chess-a0r.11."
  STATUS=3
fi

# 7. uci / setoption probe (TBC-8)
probe() { # binary outfile "input lines..."
  local bin="$1" out="$2" input="$3" pid i=0
  { printf '%b' "$input"; sleep "${PROBE_WAIT:-0}"; printf 'quit\n'; } | "$bin" >"$out" 2>&1 &
  pid=$!
  while kill -0 "$pid" 2>/dev/null && [ "$i" -lt 120 ]; do sleep 0.5; i=$((i+1)); done
  if kill -0 "$pid" 2>/dev/null; then kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; return 1; fi
  wait "$pid" 2>/dev/null || true
}

UCI_OUT="$OUT/uci-native.txt"
probe "$BIN" "$UCI_OUT" 'uci\n' || { note "uci probe: TIMEOUT"; STATUS=3; }
ID_NAME="$(grep -m1 '^id name' "$UCI_OUT" || true)"
ELO_LINE="$(grep -m1 'option name UCI_Elo ' "$UCI_OUT" || true)"
LS_LINE="$(grep -m1 'option name UCI_LimitStrength ' "$UCI_OUT" || true)"
note "uci_id_name: ${ID_NAME:-<none>}"
note "uci_option_UCI_Elo: ${ELO_LINE:-<none>}"
note "uci_option_UCI_LimitStrength: ${LS_LINE:-<none>}"
ELO_MIN="$(printf '%s' "$ELO_LINE" | sed -n 's/.* min \([0-9][0-9]*\).*/\1/p')"
ELO_MAX="$(printf '%s' "$ELO_LINE" | sed -n 's/.* max \([0-9][0-9]*\).*/\1/p')"
if [ -n "$ELO_MIN" ] && [ -n "$ELO_MAX" ]; then
  note "TBC-8: UCI_Elo min=$ELO_MIN max=$ELO_MAX (from the uci reply)"
  SET_OUT="$OUT/setoption-native.txt"
  PROBE_WAIT=3 probe "$BIN" "$SET_OUT" "uci\nsetoption name UCI_LimitStrength value true\nsetoption name UCI_Elo value $ELO_MIN\nisready\nsetoption name UCI_Elo value $ELO_MAX\nisready\nsetoption name UCI_Elo value $((ELO_MIN-1))\nisready\nsetoption name UCI_Elo value $((ELO_MAX+1))\nisready\nposition startpos\ngo depth 1\n" || true
  note "setoption_probe: readyok count=$(grep -c '^readyok' "$SET_OUT" || true) (expect 4), bestmove lines=$(grep -c '^bestmove' "$SET_OUT" || true); raw output: ${SET_OUT#"$ROOT"/}"
  note "setoption_probe_note: quit is sent 3 s after the last command; out-of-range value handling is whatever the raw output shows, not asserted here"
else
  note "TBC-8: UCI_Elo min/max NOT found in the uci reply"
  STATUS=3
fi

# optional: run the x86_64 slice under Rosetta, if available
if [ "$HAS_X86" = 1 ] && [ "$(uname -m)" = "arm64" ]; then
  if arch -x86_64 /usr/bin/true >/dev/null 2>&1; then
    printf 'uci\nquit\n' | arch -x86_64 "$BIN" >"$OUT/uci-x86_64.txt" 2>&1 || true
    note "uci_x86_64_rosetta_id_name: $(grep -m1 '^id name' "$OUT/uci-x86_64.txt" || echo '<none>')"
    note "uci_x86_64_rosetta_UCI_Elo: $(grep -m1 'option name UCI_Elo ' "$OUT/uci-x86_64.txt" || echo '<none>')"
  else
    note "x86_64 slice run: Rosetta not available; not run"
  fi
fi

# 8. COPYING (TBC-9)
COPYING_FILE="$(find "$EX" -type f -iname 'copying*' | sort | head -n 1)"
COPYING_SRC="archive"
if [ -z "$COPYING_FILE" ]; then
  COPYING_FILE="$OUT/Copying.txt"; COPYING_SRC="$COPYING_RAW_URL (not in the archive)"
  curl_get "$COPYING_RAW_URL" "$COPYING_FILE" || COPYING_FILE=""
fi
if [ -n "$COPYING_FILE" ] && [ -s "$COPYING_FILE" ]; then
  GPL_HDR="$(grep -m1 -i 'GNU GENERAL PUBLIC LICENSE' "$COPYING_FILE" | sed 's/^[[:space:]]*//' || true)"
  GPL_VER="$(grep -m1 -i '^[[:space:]]*Version [0-9]' "$COPYING_FILE" | sed 's/^[[:space:]]*//' || true)"
  note "copying_source: $COPYING_SRC"
  note "copying_sha256: $(sha256_of "$COPYING_FILE")"
  note "copying_header: ${GPL_HDR:-<none>}"
  note "copying_version_line: ${GPL_VER:-<none>}"
  [ -n "$GPL_VER" ] && note "TBC-9: GPL version line read from COPYING (human to confirm compatibility with GPL-3.0-or-later, D-10)" \
    || { note "TBC-9: version NOT read"; STATUS=3; }
else
  note "TBC-9: no COPYING file found and the tag fallback failed"; STATUS=3
fi

# 9. NNUE files
NNUE="$(find "$EX" -type f -iname '*.nnue' 2>/dev/null | sed "s#^$ROOT/##" | sort | tr '\n' ' ')"
note "nnue_files_in_archive: ${NNUE:-<none: nets are expected to be embedded in the binary [UNV]>}"
note "nnue_note: net names and licences are not derivable here; read them from the engine source tree for the tag and record them by hand"

# 10. pointers for the human
note "tag_commit_cmd: git ls-remote $GIT_URL refs/tags/${PIN_TAG} refs/tags/${PIN_TAG}^{}   (not run by this script; PIN_COMMIT is documentation, not enforced)"
note "next: copy archive_sha256 into PIN_$([ "$SOURCE" = 1 ] && echo SOURCE || echo ASSET)_SHA256 via a reviewed diff; fill docs/engineering/stockfish19-fetch-results.md from $RES"
note "exit_status: $STATUS"
say "Results written to $RES"
exit "$STATUS"
