# Recommendation: `Engine` is not dyn-compatible, `EngineFactory` is a `dyn` seam (FIND-E4)

Architect stance, 2026-11-09 (baseline B6). Author text. **Not decided, not reviewed, not approved.** The decision belongs to Brian Prascher while the delegation of G2 decisions is unconfirmed. Companion to the SDD amendment notes AN-EN-01 to AN-EN-07 in [SDD-chess-engine.md](../design/SDD-chess-engine.md) 3.10.13. Tags: `[OBS]` read, `[INF]` inferred, `[UNV]` unverified.

## 1. The question

`chess-engine` declares `Engine` with native async-fn-in-trait (`fn search(&self, ..) -> impl Future<..> + Send`) `[OBS crates/chess-engine/src/engine.rs]`. Such a trait is not dyn-compatible, so `Box<dyn Engine>` and `Arc<dyn Engine>` do not compile. The app design keeps `Seams { engine: Arc<dyn EngineFactory>, .. }` and says `dyn` is kept where two real implementations exist (UCI engine and `FakeEngine`) `[OBS SDD-chess-app.md DD-APP-17 text; architecture 4.1 item 4]`. The return type of the factory is **not written** in either document (OQ-APP-014 is about constructor names only) `[OBS]`. If the factory returns `Box<dyn Engine>`, it cannot. Nobody has coded the factory yet.

Facts that bound the options:
- Today's implementors of `Engine`: `EngineHandle` (an enum over a `UciEngine` or an arbiter lane) and `FakeEngine` (test-support) `[OBS step2-engine-evidence.md 2]`. The set is closed at two for v1.
- `EngineFactory` itself stays `dyn`-compatible only if its methods are plain (non-async, non-generic, no associated types). The pool creates instances lazily and `warm()` starts the process, so a sync factory method is natural. If an async factory method were wanted, it would hit the same problem.
- Calls are one per move or per about 100 ms update stream, not per line `[INF]`, so one heap allocation per `search` is not a performance driver. Not measured `[UNV]`.
- Whether the Rust 1.99.1 toolchain supports `dyn` for async-fn-in-trait directly: **not verified** here. Do not rely on it; check the release notes before any option that assumes it.

## 2. Options

| # | Option | What changes | `chess-app` design | Reversibility | G-level (my reading) |
|---|---|---|---|---|---|
| A | **Enum dispatch in `chess-app`.** The app defines `AppEngine { Real(EngineHandle), Fake(Arc<FakeEngine>) }` (the `Fake` variant under the app's `test-support` feature), implements `Engine` for it by forwarding three methods, and the factory returns `AppEngine`. | Only `chess-app` (`seams.rs`, factory files). `chess-engine` is untouched. | **Intact.** `Arc<dyn EngineFactory>` and `Seams` unchanged; the fake factory still wraps `FakeEngine`; the gateway holds an `AppEngine` value. | Two-way door. Moving to C later changes the factory return type and the gateway field only. | G1 inside `chess-app` (engine contract and edges unchanged; the factory return type is not in a baselined text). Strict reading of "touches an interface" could make it G2: Brian's call. |
| B | **Generics.** `Core<F: EngineFactory>` with an associated `type Engine: Engine`. | The type parameter spreads through `Core`, sessions, services and the front-ends. `Seams.engine` can no longer be `Arc<dyn EngineFactory>`. | **Broken:** D-39, architecture 4.1 and 5.7 text, app SDD DD-APP-17 and the front-end SDDs change. | Hard to reverse once four crates name the type. | G2 (architecture text and four crates). Not recommended. |
| C | **Dyn-compatible wrapper in `chess-engine`.** A second trait `DynEngine` with `fn search(&self, req) -> Pin<Box<dyn Future<Output = Result<..>> + Send + '_>>`, a blanket `impl<T: Engine> DynEngine for T`; the factory returns `Arc<dyn DynEngine>`. No new dependency (hand-written, not `async-trait`). | New public item in `chess-engine`; app uses it. | **Intact** in shape (still trait objects, open set of engines). | Two-way (additive; removable if unused). | G2 by rule: touches two crates and the interface the app consumes. |
| D | **Change `Engine` itself to return a boxed future.** | Signature change of the trait; every implementor and test of the crate being reworked now. | Intact in shape. | Two-way but churns the crate under concurrent rework; the code already passes 114 tests on the current signature. | G2 (the interface changes). Not recommended over C. |
| E | **Native `dyn` async traits**, if the toolchain supports them. | Unknown. | Unknown. | Unknown. | `[UNV]` Not assessable until checked. |

## 3. Recommendation (Medium confidence)

**Option A**, with the constraint that factory methods are synchronous. Reasons:
1. It is the only option that changes nothing in `chess-engine`, which the software engineer is reworking now, and nothing in any baselined text. It keeps `Arc<dyn EngineFactory>` as the app SDD wrote it.
2. The set of implementations is closed and small (two). The app SDD's own rule for `dyn` is "more than one real implementation", and enum dispatch is the plain answer for a closed pair. It adds no dependency and no allocation.
3. It is cheap to undo: option C is a mechanical later step if a third engine or an out-of-crate implementor appears.

Cost `ESTIMATE` (no measurement): a few dozen lines in `chess-app` (the enum, three forwarding methods, a feature-gated variant) `[INF]`, plus one test that `AppEngine::Fake` and `AppEngine::Real` pass the same engine-contract suite (SDD AN-EN-04 item 6). Weakness: the type differs between a test build and a release build (the `Fake` variant exists only under `test-support`), so a `match` on it is built differently in the two. Keep matches inside the forwarding impl only.

**Revisit trigger for C:** a second engine implementation outside `chess-app`, or a front-end crate needing to hold an engine without naming `AppEngine`.

## 4. What the decision needs

- **Decision for Brian Prascher** (G2 if he reads "touches two crates" strictly; G1 desk review under my reading of A). I do not assume delegation. Confidence: Medium, because the factory signature is unwritten and the app SDD owner may already have a shape in mind.
- If A is accepted: an app SDD note (G0/G1, app owner) stating the factory return type, closing the "dyn Engine" ambiguity, and rewording D-61's "implements the same `Engine` trait" to "wrapped by `AppEngine`". The architecture text is not changed by A.
- If C is chosen: a G2 interface addition in the engine SDD (3.6.1) and a matching app note.
- Either way, `FakeEngine` stays behind `test-support`, never in a release build.

## 5. Not analyzed

Compile-time and binary-size cost of each option; whether the Tauri or server crates would prefer a different shape; the toolchain's `dyn` async support (E); the app owner's intent for the factory signature.
