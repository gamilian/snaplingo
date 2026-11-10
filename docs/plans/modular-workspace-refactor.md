# Modular native workspace refactoring

Status: completed. Verified 2026-11-10.

This plan replaces the earlier layer-oriented directory cleanup. The goal is a
cohesive native Rust codebase that can extend platforms and reuse UI through
explicit interfaces, without a Cargo package for each technical layer. The
current design is documented in [ARCHITECTURE.md](../../ARCHITECTURE.md).

There are six production packages: the desktop application, Core, UI, Platform,
Storage and Assets. The independent reference collector is a development package.
All use Cargo's conventional source layout. Package/executable identities and
existing native behavior remain acceptance constraints.

## Implementation and verification

- [x] Move Core to feature-owned modules: models, workflows and consumed ports
  live together; remove public `domain`/`application` module-tree aliases.
- [x] Consolidate presentation foundations, feature UI, optional rendering and
  accessibility in `snaplingo-ui`; move Settings behavior out of its root file.
- [x] Extract native capabilities into `snaplingo-platform` with actual
  Mac/Windows implementations; remove application-state/runtime dependencies.
- [x] Move single-instance subprocess acceptance to Platform; replace native
  shortcut tests' source mounting with a feature-gated event injection entry.
- [x] Put product assembly in `apps/desktop`, separate bootstrap/shell/features,
  and group Settings presenters/native hosts under their feature.
- [x] Inject feature dependencies instead of passing AppState or calling
  bootstrap from native feature hosts.
- [x] Supply Settings system paths through composition; remove ambient path
  discovery from portable configuration workflows.
- [x] Complete application compilation and verify all moved production/test/example
  entry paths, feature flags and resource paths.
- [x] Update and run all 16 desktop source architecture checks against the current
  feature trees, including provider/capture/frozen-coordinate policy checks.
- [x] Run the Cargo dependency/source workspace guard and desktop architecture
  target through the final application build.
- [x] Finish Capture presentation consolidation against native macOS controls,
  preserving frozen pixels, Core Editing, native text input and output behavior.
- [x] Verify shared UI behavior plus independent geometry/color references and
  native macOS feature workflows after integration.
- [x] Verify Windows all-target application compilation after integration;
  record actual Windows GUI/IME/accessibility acceptance separately.
- [x] Verify reference/script tests, resource/license generation and build/package
  contracts after all paths settle.

The portable verification gate enables the UI features explicitly, leaving the
optional Tesseract engine disabled unless its system dependencies are installed:

```bash
cargo test --locked -p snaplingo-core -p snaplingo-storage -p snaplingo-platform -p snaplingo-ui --features snaplingo-ui/rendering,snaplingo-ui/accessibility --all-targets
```

## Confirmed evidence

Platform extraction passed `cargo test -p snaplingo-platform --all-targets
--features test-support --offline`: 78 unit tests passed, one manual WindowServer
benchmark remained intentionally ignored, and the actual subprocess
`single_instance_native` test passed. `cargo check -p snaplingo-platform --target
x86_64-pc-windows-gnu --all-targets --features test-support --offline` passed. These
checks cover the extracted adapters and shared color dialog compilation; they do
not prove desktop application integration or GUI acceptance.

The 16 desktop source architecture checks passed using the actual test source
compiled independently with the cached Syn dependency and the desktop manifest
root. This validates source ownership while application integration is in
progress; the final Cargo/application gate remains separate.

After removing ambient Settings path discovery from Core, all 409 Core library
tests passed, including a regression that binding a notifier preserves injected
default paths and tilde expansion. Production Core and Platform library checks
also passed without enabling test-support. Platform supplies the existing
Pictures/home/temporary fallback; bootstrap and the preview inject its result.

Final integration evidence (2026-11-10):

- The portable verification gate above passed against the final sources: Core
  409, Platform 78 (one manual probe ignored), Storage 29, UI 104 unit tests;
  real single-instance subprocess acceptance, all 21 UI integration checks and
  all renderer/presentation example targets passed.

- `cargo check --locked --offline --workspace --all-targets` passed, including
  the independent Reference collectors in their conventional `src/bin` layout.
  App's `--no-default-features --lib` check also passed.
- `cargo check --offline -p snaplingo --target x86_64-pc-windows-gnu --all-targets`
  passed after the final platform imports and Capture assets were connected.
- App library tests: 151 passed, one manual probe ignored. Real-adapter workflow
  tests: 80 passed; infrastructure integration: 3; desktop architecture: 16;
  isolated Settings preview: 10. Settings factory tests verify that feature
  dependencies release the application owner and create independent sessions.
- Real macOS tests passed: `native_app_smoke`, `native_ui_workflows`,
  `selected_text_native`, `ui_thread_native`, `settings_general_native`,
  `settings_features_native`, `settings_hotkeys_native`, `pinned_window_native`,
  `scrolling_controls_native` and `scrolling_overlays_native`. These exercised
  seven-page navigation, persistent zoom/geometry, native text/IME, shortcuts,
  capture/edit/output recovery, Result, pins, scrolling and graceful disposal.
- Actual Capture AppKit snapshots exactly matched the retained independent
  baseline: 28 ordinary-toolbar and 32 text-toolbar controls and states, six
  palette swatches, three palette actions and popup size. The native toolbar
  placement assertion `[120, 314, 700, 112]` passed. Frozen capture/output and
  native text composition remained covered by the real workflow tests.
- UI's all-feature suite passed 104 unit and 21 integration tests, including
  independent geometry references and workspace architecture checks; all 12
  examples built. Rendering-only and accessibility-only checks passed separately.
  The default UI dependency tree contains only `unicode-segmentation`.
- `npm test`: 941 passed, one intentional platform-dependent skip.
  `npm run reference:build`, regenerated third-party notices and their lock check,
  Cargo formatting, whitespace checks and a scan of literal Rust resource/source
  includes all passed. Script tests cover the moved native packaging contracts.

HTTP/local-listener, keychain and AppKit tests ran outside the filesystem sandbox
where their native facilities are available. Sandbox-denied attempts are not
counted as passing evidence. No Windows GUI/IME/Narrator or installer was executed
on this macOS machine; Windows cross compilation is reported only as compilation.

## Constraints

- Core and UI remain independent of concrete platform/storage/window types.
- UI layout and interaction are shared; native hosts retain IME, surrounding text,
  clipboard, focus, accessibility, dialogs and event-loop lifetime.
- Mac appearance/interaction is the reference for existing surfaces.
- Windows retains one event loop, frozen capture coordinates and explicit alpha
  presentation; device DPI does not alter saved interface zoom.
- Production resources are independent of retained browser sources.
- No package identity, installed data format or executable rename is implied by
  moving source files.
