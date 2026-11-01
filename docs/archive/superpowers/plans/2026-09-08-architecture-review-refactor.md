# Architecture review refactor — 2026-09-08

Based on `architecture-review-20260908-091622.html` (reviewed HEAD `da7ce90`).
Preserve user-visible workflows, accepted ADRs, and platform-specific native behavior.
The release workflow, release scripts, and release documents already have unrelated
uncommitted changes and are outside this refactor.

## Steps and verification

1. **Result Window state and scheduling** — move translation session state,
   request validity, deduplication, and 150/500 ms scheduling into the existing
   Application runtime. Zustand subscribes to snapshots, as it does for Settings.
   Verify through runtime intents and controlled Provider promises/timers; retain
   View rendering tests and migrate Store policy tests to the runtime interface.
2. **Frozen capture coordinates** — keep frozen monitor geometry and the Windows
   single-primary-scale convention with the capture session. Pass that convention
   to window, cursor, and control adapters. Verify negative origins, mixed DPI,
   changed live layouts, and selection-to-frozen-pixel correspondence.
3. **Frontend Capture ownership** — move the existing runtime, state, and pure
   workflow policy into `application/capture-workspace`; remove synonymous
   platform forwarding and inject DOM printing. Keep React, Canvas, decoding,
   rendering, and input conversion in Views. Retain runtime interface tests and
   focused DOM tests; extend architecture guards to the moved implementation.
4. **Backend Capture orchestration** — make startup serialization and Pin delivery
   runtime responsibilities. Commands make one Application call; Composition
   connects the existing Pinned Image module. Preserve failure cleanup and
   Pinned Image state retention. Verify overlapping startup, retry after failure,
   and exactly-once delivery of frozen pixels.

Finish with frontend type/build checks, the full frontend and host-native Rust
test suites, and architecture checks. Windows/Linux native UI acceptance remains
separate from macOS-hosted mathematical and interface tests.

## Baseline

- Frontend focused suite: 54 files, 562 tests passed.
- Rust Capture: 74 tests passed; screenshot geometry: 5 passed.
- Rust architecture: 6 tests passed.

## Scope decisions

- Keep the existing state library, OS adapters, Windows scale rule, and Capture
  editing model. Introduce no global scheduling, geometry, or error framework.
- Overlapping window-open intents are ignored at the Capture runtime for every
  entry point. Direct Session creation returns a busy error during startup because
  it must return a new Session. Completed and failed startups release the guard.
- A Provider retry belongs to its current translation session and must not
  invalidate other Providers in that session.
- Reopening a Result Window without replacement text retains completed results
  without another automatic request, and retained Provider errors remain
  retryable. Interrupted requests cannot update a reopened session.
- Capture preparation and reveal carry the adopted Session ID through IPC and
  read its frozen geometry, including after refresh. A failed Session load may
  reveal its error without supplying geometry; native adapters retain the
  existing frame in that case.
- Legacy History/Favorites pages mentioned as a separate follow-up in the report
  are outside these four changes; their live Stores are retained.

## Progress

- [x] Result Window — 148 focused tests and TypeScript check passed
- [x] Frozen capture coordinates — 77 Capture, 5 geometry, and 54 native window tests passed
- [x] Frontend Capture ownership — 457 focused tests and TypeScript check passed
- [x] Backend Capture orchestration — startup overlap, failure/retry, idempotent hide, and Pin delivery verified
- [x] Integration verification and architecture documentation

## Final verification

- Full frontend suite: **120 files, 891 tests passed** (`npm test`; final run
  used the dot reporter and silent output).
- Production frontend build: **passed** (`npm run build`, TypeScript + Vite).
  Vite still reports a minified chunk larger than 500 kB; this does not fail the
  build and code splitting is outside this refactor.
- Full current-host Rust suite: **552 tests passed**
  (`cargo test -p snaplingo --quiet`: 539 unit tests and 13 integration tests,
  including 8 architecture checks).
- Rust formatting: **passed** (`cargo fmt --all -- --check`).
- Diff whitespace: **passed** (`git diff --check`).
- Confirmed all 44 removed Capture View files have Application replacements;
  mixed DOM/policy files were split at the existing host seams.
- Updated `ARCHITECTURE.md`, the runtime map, and the module ownership document,
  including the implemented Windows System Speech adapter.

These are automated checks on the current macOS host. Native Windows and Linux
desktop acceptance was not run. Windows acceptance should compare the overlay,
cursor/control selection, and exported pixels across mixed DPI and negative
monitor origins, then repeat after changing the layout and refreshing the
session. Linux X11/Wayland and native multi-monitor behavior require a real
desktop environment.
