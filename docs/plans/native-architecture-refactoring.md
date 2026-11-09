# Native Rust architecture refactoring

Approved direction: 2026-11-09. Baseline commit: `ef69b24`.

The architecture review found a sound Core / Storage / UI / Render split, but
workflow ownership and production resource ownership still reflect the migration.
This plan makes existing modules deeper: common behavior has one owner, platform
adapters retain operating-system mechanics, and tests exercise the same interface
as callers. It implements ADR 0012 rather than replacing it.

## Acceptance policy

- **macOS is the reference for overall UI consistency.** Preserve its layouts,
  appearance, controls and interaction semantics when consolidating presentation.
  Adapt the shared implementation and Windows presentation to that reference.
- Architecture remains portable: Core owns business workflows; UI owns shared
  presentation state and geometry; Render owns pixels; native adapters own input,
  windows, dialogs, accessibility and presentation mechanics.
- Preserve macOS IME, surrounding-text services, clipboard behavior, shortcuts,
  window lifecycle and accessibility. A shared canvas must retain those adapter
  capabilities rather than replacing them with incomplete generic behavior.
- Preserve one application event loop on Windows, physical/logical coordinate
  contracts, frozen capture pixels and explicit alpha presentation.
- Real Windows GUI/IME/Narrator/install acceptance is separate from cross checks.
  macOS exact-font fixtures are not Windows font baselines.
- Each step is independently reviewable and verified before proceeding. Do not
  change installation identity, executable names, persistence formats or package
  identity as part of a source-directory move.

## Target directory structure

```text
crates/
  app/                 composition, lifecycle, actions, platform adapters
  core/                domain, workflows, inward ports
  storage/             SQLite and persisted-file adapters
  settings/            common Settings session and projections
  ui/                  presentation state, scene, interaction, theme
  render/              text shaping and CPU rasterization
  accessibility/       shared semantic projection
  desktop/             existing winit window adapter
  assets/              production resource embedding
assets/
  fonts/
  defaults/
  text/
  licenses/
reference/
  web/                 retained React views, behavior tests and fixtures
docs/
script/
```

Cargo package names remain `snaplingo-*`; the app package remains `snaplingo`.
Directories do not repeat the product prefix. Rust entry files and modules live
directly in each crate; no extra `src/` layer is used. Business-window adapters are
organized by domain, with the two real platform adapters adjacent:

```text
crates/app/
  composition/
  lifecycle/
  actions/
  window/
    settings/
    capture/
    result/
    pinned/
    scrolling/
      mod.rs
      session.rs
      macos/
      windows/
  infrastructure/      screenshot acquisition, selection, OCR, speech, HTTP, etc.
```

This is a responsibility map, not a requirement to create empty directories or
single-use forwarding modules. Source moves follow the corresponding ownership
change. Existing native acceptance fixtures stay available until their equivalent
interface tests protect the replacement.

## Execution checklist

### 0. Source directories

- [x] Rename all nine crate directories to the short names above.
- [x] Put Rust entry files/modules directly in each crate and remove `src/`.
- [x] Update workspace members, path dependencies, cross-crate source/fixture
  references, scripts, CI, resource ignores and current documentation.
- [x] Verify Cargo metadata, formatting, architecture tests, script contracts,
  reference build, macOS compilation and Windows cross compilation.

### 1. One Settings workflow owner

Evidence: Windows uses `SettingsWindowSession`; macOS `SharedFeaturePage` separately
owns dispatch, Hotkey mutation queues, refresh ordering and disposal. Reopening
Settings without a route restores the Mac page but selects General on Windows.

- [x] Move portable session/projection logic into the existing Settings module.
- [x] Remove AppState and operating-system construction knowledge from that
  module; Composition injects existing workflows and concrete adapters.
- [x] Make both platform adapters use the same session for navigation, edit/save
  ordering, refresh, stale-result rejection and disposal.
- [x] Keep native input, UI-thread delivery, dialogs and AX/UIA in adapters.
- [x] Preview reuses the common workflow; its temporary database/registrar setup
  belongs to its composition, not the production workflow interface.
- [x] Run common SQLite persistence and controlled-delay race tests on both
  platforms, without constructing the full OS AppState. Preserve existing
  Library, Services and General behavior coverage and Mac native input tests.

### 2. Scrolling Capture lifetime

Evidence: both hosts implement sampler state/action sequencing. Windows owns an
abortable task and Drop-based sampling restoration; Mac stop only marks closed
and sampling restoration follows the ordinary await path.

- [x] Consolidate common sampling/action/lifetime policy in one session module.
- [x] Use the same failure/cancellation restoration path on both adapters.
- [x] Preserve Core stitching/output workflows and platform-specific sampling,
  UI-thread delivery, visibility and file dialogs.
- [x] Verify start-once, close-during-sample, error pause/resume, cancellation
  restoration, late-result rejection and native preview/output behavior.

### 3. Core seam and test ownership

Evidence: public `AppError` contains Reqwest and SQLite error types; Core tests
compile App/Storage sources through `#[path]` and `include!`.

- [x] Convert technical errors inside their adapters while retaining diagnostic
  causes and existing observable error classification.
- [x] Remove production Core dependencies required only by concrete error types.
- [x] Keep portable behavior tests at existing workflow interfaces; locate real
  SQLite/network integration tests outside Core and remove source-file imports.
- [x] Narrow migration re-exports as callers/tests move to their owning crate.
- [x] Preserve transaction, credential rollback, concurrency and Provider tests;
  verify dependency direction using Cargo metadata and production source graphs.

### 4. Production assets and independent Reference

Evidence: Core includes JSON from `src/domain`; native notices and packaging read
the same tree; macOS production parses reference CSS for colors while UI stores
another Rust palette. Browser fixtures are split between root src and App tests.

- [x] Move production defaults, text and notice outputs into neutral assets.
- [x] Make shared UI theme own production semantic colors; Mac adapters perform
  native color conversion without parsing CSS.
- [x] Move retained browser source/fixtures into `reference/web`; preserve its
  independent CSS/DOM/AppKit baselines, not renderer-generated expected values.
- [x] Update collectors, Vite, tests, notice generation and native packaging.
- [x] Verify native compilation without Reference sources in a temporary copy,
  plus reference build/tests, notice generation and native bundle contracts.

### 5. Shared window presentation, macOS reference

Evidence: PinUi is Windows/test-only; Mac duplicates Action/Preferences, shortcuts
and toolbar presentation. Result shares a controller but still has a separate
AppKit presentation. Capture and Scrolling also retain native presentation paths.

- [ ] Pinned Image: unify presentation state, shortcuts and toolbar geometry,
  preserving Mac appearance and interactions. Keep decoding/image preparation
  outside pure UI; retain transforms, sampling, grouping and DPI tests.
- [ ] Scrolling controls/preview: align shared presentation with Mac and connect
  both adapters to it, preserving the common lifetime from step 2.
- [ ] Result: retain the shared controller, unify presentation and retain native
  text composition, selection, surrounding text and accessibility capabilities.
- [ ] Capture: retain frozen pixels and Core Editing; unify presentation against
  Mac controls without moving OS acquisition or output policy into UI.
- [ ] Reorganize each window's common implementation and platform adapters under
  `app/window/<domain>` as ownership settles.
- [ ] Run interface behavior tests, independent geometry/color references and
  real macOS window workflows for each completed vertical slice.

## Verification record

Record implementation commits, commands, results and platform limits below as
steps finish. Compilation is evidence of compilation, not desktop acceptance.

- Baseline: `ef69b24` commits the native migration and source-platform font gates.
  Prior native/shared/browser tests and macOS package verification are recorded
  in the migration research documents. No refactoring is included in that commit.

- Step 0: renamed all nine crate directories and updated 56 textual path consumers.
  Cargo metadata, all-target macOS/Windows checks, three architecture guards,
  shared Rust all-target tests, 941 browser/script tests and reference build pass.
  Cargo package names and native artifact identity are unchanged.

- Step 1: both native hosts now use `settings::window::SettingsWindowSession`.
  Portable General/Library/Services/capacity/permission behavior and race tests
  live in Settings; construction lives in `app/composition/settings`.
  Desktop no longer has a production dependency on Settings construction.
  Preview SQLite/registrar setup and its ten behavior tests live beside its
  example; projection and hotkey mutation remain shared. Recording IDs are
  unique across recreated Mac pages; Windows restores its active page on reopen.
  Portable tests (23), preview behavior (10), Desktop input (3), real application
  composition and macOS seven-page/zoom/modal/quit smoke pass. Mac layout,
  native text input, AX and dialogs are retained. Windows all-target and portable
  session-test cross checks run separately from real Windows acceptance.

- Step 2: Scrolling Capture now owns one shared sampler/action session under
  `app/window/scrolling`, with adjacent Mac/Windows adapters. Both use the
  same Drop-based restoration, including cancellation before the hide response.
  Mac window controls and native save dialog are retained. Shared geometry and
  sampling tests (7), the complete real Mac capture/editor/result/pin/scrolling
  workflow and Windows all-target checks pass. Architecture guards (11 app,
  3 workspace) pass against the relocated portable Settings sources as well.
  Services navigation now has an injected store adapter; Settings has no
  production Storage dependency. Portable and preview regression tests pass.

- Step 3a: Core errors retain opaque diagnostic causes while Storage and HTTP
  adapters convert concrete errors explicitly. Reqwest and SQLite are now Core
  dev-dependencies only, used by the existing adapter fixtures. The attempted
  wholesale test relocation was discarded: private implementation tests need
  individual interface-level replacement, not enlarged production visibility.
  Existing Core test coverage remains in place; test ownership/re-export cleanup
  is an explicit follow-up within step 3.

- Step 3b: completed test ownership without exposing private Core state. Portable
  tests use memory configuration/credential/event adapters; 80 real-adapter tests
  now live in `app/tests/core_workflows`, retaining their injected dependencies
  explicitly. Removed Core's `test_infrastructure.rs` and all cross-crate source
  mounts. Core tests (408) and integration tests (80) pass; the old aggregate
  Core count included duplicate Storage/network adapter tests now run in their
  owner crates. Reqwest/SQLite are no longer Core dependencies even in tests.
  Existing returned Result auto-copy data is explicitly re-exported; production
  workflow implementation fields remain private.

- Step 4 and source flattening: removed the `src/` layer from all nine crates.
  Production defaults, UI text and generated notices live under `assets/`;
  retained React sources and fixtures live under `reference/web`. Native Mac
  theme adapters consume the shared Rust palette, with all semantic tokens
  checked against the independent CSS reference. Packaging, collectors and
  TypeScript/Vite paths use the new owners. A source guard rejects production
  embedding of Reference resources. A temporary copy with no `reference/`
  passes `cargo check --workspace --lib --bins --locked --offline`.
  macOS and Windows all-target checks, all 759 Rust library tests, 941 browser
  and script tests, reference build and regenerated notice checks pass. Mac
  seven-page application smoke and native capture/editor/result/pin/scrolling
  workflows pass; the full native workflow needed one rerun after a pointer
  detection timeout. Local HTTP fixtures now inject clients with proxy disabled
  so their loopback requests cannot be redirected by system proxy settings.
