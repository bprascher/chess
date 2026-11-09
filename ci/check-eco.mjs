#!/usr/bin/env node
// CI-RUL-001: verifies that third_party/eco/ holds the recorded snapshot: every file listed in
// SHA256SUMS exists and has the recorded SHA-256, the five TSV files and the CC0 licence text
// (COPYING.txt) and SOURCE.md are present, and no TSV file is unlisted. Needs only node.
//
// Usage: node ci/check-eco.mjs [path/to/third_party/eco]
import { createHash } from "node:crypto";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const dir = process.argv[2] ? resolve(process.argv[2]) : join(root, "third_party", "eco");
const required = ["a.tsv", "b.tsv", "c.tsv", "d.tsv", "e.tsv", "COPYING.txt", "SOURCE.md", "SHA256SUMS"];
const failures = [];

for (const f of required) {
  if (!existsSync(join(dir, f))) failures.push(`missing ${f}`);
}
const listed = new Map();
if (existsSync(join(dir, "SHA256SUMS"))) {
  for (const line of readFileSync(join(dir, "SHA256SUMS"), "utf8").split("\n")) {
    if (!line.trim()) continue;
    const m = /^([0-9a-f]{64})\s+\*?(\S+)$/.exec(line);
    if (!m) failures.push(`bad SHA256SUMS line: ${line}`);
    else listed.set(m[2], m[1]);
  }
}
for (const f of ["a.tsv", "b.tsv", "c.tsv", "d.tsv", "e.tsv", "COPYING.txt"]) {
  if (!listed.has(f)) failures.push(`${f} is not listed in SHA256SUMS`);
}
for (const [f, want] of listed) {
  if (!existsSync(join(dir, f))) {
    failures.push(`${f} is listed but missing`);
    continue;
  }
  const got = createHash("sha256").update(readFileSync(join(dir, f))).digest("hex");
  if (got !== want) failures.push(`${f}: SHA-256 is ${got}, recorded ${want}`);
}
if (existsSync(dir)) {
  for (const f of readdirSync(dir)) {
    if (f.endsWith(".tsv") && !listed.has(f)) failures.push(`${f} is not listed in SHA256SUMS`);
  }
}
if (existsSync(join(dir, "COPYING.txt")) && !readFileSync(join(dir, "COPYING.txt"), "utf8").includes("CC0 1.0 Universal")) {
  failures.push("COPYING.txt is not the CC0 1.0 text");
}
if (failures.length) {
  for (const f of failures) console.error(`FAIL ${f}`);
  process.exit(1);
}
console.log(`eco snapshot check OK: ${listed.size} files match SHA256SUMS`);
