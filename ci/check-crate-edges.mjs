#!/usr/bin/env node
// Fails if a workspace crate depends on another workspace crate along an edge that
// docs/architecture.md section 3.2 (D-38) does not allow. Covers normal, build and
// dev dependencies. Needs only node and cargo; no third-party package.
//
// Usage: node ci/check-crate-edges.mjs [path/to/Cargo.toml]
import { execFileSync } from "node:child_process";

// Allowed edges: crate -> crates it may depend on (architecture 3.2, D-38).
const ALLOWED = {
  "chess-rules": [],
  "chess-domain": ["chess-rules"],
  "chess-engine": ["chess-rules"],
  "chess-store": ["chess-domain"],
  "chess-app": ["chess-engine", "chess-store", "chess-domain", "chess-rules"],
  "chess-server": ["chess-app"],
  "chess-cli": ["chess-app", "chess-server"],
  "chess-desktop": ["chess-app"],
};

const manifest = process.argv[2];
const args = ["metadata", "--format-version", "1", "--no-deps", "--locked"];
if (manifest) args.push("--manifest-path", manifest);
let meta;
try {
  meta = JSON.parse(
    execFileSync("cargo", args, { encoding: "utf8", maxBuffer: 64 * 1024 * 1024, stdio: ["ignore", "pipe", "pipe"] }),
  );
} catch (e) {
  console.error(`FAIL cargo metadata failed (a broken manifest also fails this check):\n${e.stderr ?? e.message}`);
  process.exit(1);
}

const members = new Map(meta.packages.map((p) => [p.name, p]));
const errors = [];

for (const name of members.keys()) {
  if (!(name in ALLOWED)) {
    errors.push(`unknown workspace crate "${name}": add it to ALLOWED in ci/check-crate-edges.mjs (architecture 3.1 change, G2)`);
  }
}
for (const name of Object.keys(ALLOWED)) {
  if (!members.has(name)) errors.push(`expected workspace crate "${name}" is missing`);
}

let edges = 0;
for (const pkg of members.values()) {
  for (const dep of pkg.dependencies) {
    if (!members.has(dep.name)) continue; // third-party: not this check
    edges += 1;
    const kind = dep.kind ?? "normal";
    const allowed = ALLOWED[pkg.name] ?? [];
    if (!allowed.includes(dep.name)) {
      errors.push(`disallowed edge: ${pkg.name} -> ${dep.name} (${kind})`);
    }
  }
}

if (errors.length > 0) {
  for (const e of errors) console.error(`FAIL ${e}`);
  console.error(`crate-edge check FAILED (${errors.length} problem(s))`);
  process.exit(1);
}
console.log(`crate-edge check OK: ${members.size} crates, ${edges} workspace edges, all allowed`);
