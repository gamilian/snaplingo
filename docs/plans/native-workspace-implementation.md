# Native workspace implementation

Status: implemented and locally verified 2026-11-10, from `cebc309a`.
Windows runtime acceptance remains a native-runner/release responsibility;
this session verified Windows compilation, not GUI execution.

This implements the [native workspace design](../architecture/native-workspace-design.md):
one `app/` product, feature-owned Core/UI, capability-owned Platform/Providers/Storage,
and conventional Cargo source layout. Application identity and behavior are preserved.
Earlier refactor records describe their own baseline, not this verification run.

## Implementation

- [x] Move `apps/desktop` to `app`, retaining package/binary/install identities;
  update source/resources, Cargo, scripts, CI and active documentation paths.
- [x] Extract external protocols, HTTP and local inference into `snaplingo-providers`.
  Keep product coordination and pure strategy wrappers in Core. Remove forwarding
  modules and the old provider `impls` directories.
- [x] Gate ORT/MNN and associated dependencies behind `local-ocr`; enable it in
  the product. Keep dynamic network settings and inference fallback behavior.
- [x] Make Core's Tesseract contract and pure strategy available on every target.
- [x] Give UI foundation a pure drawing Scene and Appearance; keep interaction,
  scrolling, IME and semantic metadata in feature frames.
- [x] Move retained AppKit reference implementations to Settings `test_support`,
  with explicit imports distinct from production shared-page acceptance.
- [x] Move web reference entry/config/public files into `reference/web`; retain
  root npm commands and independent script/reference Vitest coverage.
- [x] Centralize production package metadata and repeated dependency versions,
  preserving per-package feature flags. Build/release scripts resolve Cargo metadata.
- [x] Enforce package edges, protocol-only inference isolation and lower UI module
  imports, including negative examples for root aliases and source mounts.
- [x] Update CI for the new Provider smoke-test owner and the web reference build.
- [x] Complete whole-product, local native and cross-compilation verification.
- [x] Audit current documentation, resource/license inventory and final diff.

## Verification observed during this refactor

Before implementation, Core's 409 and UI's 104 unit tests passed.

After extraction, Core's 377 tests and Providers' 63 protocol tests passed.
Providers with `local-ocr` passed 73 tests; its real-model smoke remained ignored
in that ordinary suite and requires a separate explicit run. Five Core Tesseract
fake-engine tests now run without enabling native Tesseract.

The Providers Windows GNU check with `local-ocr --all-targets` passed. Its default
normal/build dependency closure excludes ORT, MNN, ndarray, oar and Foundation.
The macOS MNN build encountered a network download error; reuse of an existing
valid prebuilt cache completed the build and tests, without changing dependencies.

UI passed 104 unit tests and 23 integration/architecture tests with rendering and
accessibility enabled; examples compiled. Existing independent layout/color
references were retained. The workspace guard contributes five of those tests.

Node passed 117 test files: 941 tests passed and one remained skipped.
`npm run reference:build` passed license-lock checking, TypeScript and Vite.
Reference development and preview servers served the entry, three fixture pages
and their assets successfully. Safari 15 / Chrome 105 targets are preserved.

## Final integration gates

All commands below passed. Provider protocol/default and local-ocr suites ran
separately; the real model smoke test also ran explicitly and passed. Windows
compilation and actual desktop acceptance are different checks.

```sh
cargo fmt --all --check
cargo check --locked --workspace --all-targets
cargo test --locked -p snaplingo
cargo test --locked -p snaplingo-core -p snaplingo-storage -p snaplingo-platform -p snaplingo-ui --features snaplingo-ui/rendering,snaplingo-ui/accessibility --all-targets
cargo test --locked -p snaplingo-providers --no-default-features --all-targets
cargo test --locked -p snaplingo-providers --features local-ocr --all-targets
cargo check --locked -p snaplingo --target x86_64-pc-windows-gnu --all-targets
SNAPLINGO_OCR_SMOKE_ROOT="$PWD/app/resources/ocr" cargo test --locked -p snaplingo-providers --features local-ocr bundled_medium_smoke -- --ignored --nocapture
npm test
npm run reference:build
```

The combined Core/Platform/Storage/UI suite passed 611 tests, with one existing
manual screenshot benchmark ignored. Platform's actual single-instance subprocess
test also passed. Product tests passed 115 unit tests, 16 architecture checks,
80 Core workflows and three adapter integration tests, followed by all native
harnesses: application lifecycle, shared Settings, result/input/IME, capture and
annotation tools, pinning, scrolling, selected text, hotkeys and UI-thread cleanup.
Retained AppKit reference harnesses passed separately from production shared UI.

`cargo check --locked --workspace --all-targets` passed on macOS, and
`cargo check --locked -p snaplingo --target x86_64-pc-windows-gnu --all-targets`
passed. Existing dead-code/unused-import warnings in the application remain;
the shared UI suite compiled without warnings.

The full local packaging command passed with ad-hoc signing:

```sh
SNAPLINGO_SIGNING_MODE=adhoc npm run native:build -- --debug
```

It produced and verified `target/native/debug/SnapLingo.app` and
`target/native/debug/bundle/dmg/snaplingo-v0.1.23-macos-aarch64.dmg`, including
resource presence, native dependencies, signature checks and DMG checksum.
The generated `target/native/debug/verification.json` records the artifact hash.
No release publication or installed-application replacement was performed.

Windows GUI/IME/UIA and NSIS/MSI/portable execution were not run on this macOS
machine. Their existing CI/release harnesses remain configured for the new paths;
the Provider real-model command now selects the correct package and explicit
resource root. Minimum-version macOS and Intel runtime acceptance are also
separate from the local Apple Silicon verification.
