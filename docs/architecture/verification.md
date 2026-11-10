# Architecture verification

This is the maintained verification guide for the native workspace. Current module
ownership is documented in [ARCHITECTURE.md](../../ARCHITECTURE.md). Completed
migration checklists are removed; Git history retains their implementation record.

## Automated checks

```sh
cargo fmt --all --check
cargo check --locked --workspace --all-targets
cargo check --locked -p snaplingo --features test-support --all-targets
cargo test --locked -p snaplingo --features test-support
cargo test --locked -p snaplingo-core -p snaplingo-storage -p snaplingo-platform -p snaplingo-ui --features snaplingo-ui/rendering,snaplingo-ui/accessibility --all-targets
cargo test --locked -p snaplingo-providers --no-default-features --all-targets
cargo test --locked -p snaplingo-providers --features local-ocr --all-targets
npm test
npm run reference:build
```

The workspace guard checks Cargo dependency direction, default Provider inference
isolation, Core source ownership and UI foundation/render imports. The application
guard checks injected feature dependencies, model/workflow direction, frozen
capture coordinates and native acceptance targets linking the compiled library.
Synthetic invalid examples ensure these rules actually reject forbidden coupling.

Behavioral tests use the same interfaces as production callers. Settings tests
cover pending confirmations, cancellation, stale/repeated replies and real stored
mutations. Capture completion tests exercise real Core workflows with frozen image
bytes, SQLite and recording effects, including output failure/retry and stale
native dialogs. Database tests cover persistence and transactions; Provider tests
cover external protocols, dynamic configuration and inference fallback.

## Native acceptance

Native scenarios are separate main-thread processes linked through the optional
application `test-support` entry. CI explicitly enables this feature; normal
product builds omit the runner and probes. Run a scenario independently with:

```sh
cargo test --locked -p snaplingo --features test-support --test native_app_smoke
cargo test --locked -p snaplingo --features test-support --test native_ui_workflows
```

The ten retained native targets cover application lifecycle, shared Settings,
result/input/IME, capture and annotation output, pinning, scrolling, selected text,
hotkeys and UI-thread cleanup. General/Features/Hotkeys reference scenarios retain
independent AppKit baselines; they supplement production shared-UI acceptance.
React fixtures and collectors under `reference/web` and `tools/reference` are
independent visual/behavior evidence and are not included in native bundles.

The application workflow fixture joins its worker before releasing state and the
temporary database directory. No production source is copied into integration
tests, and build scripts do not generate a second composition module graph.

## Models, platforms and packaging

Real bundled inference is separate from the ordinary fake-engine tests:

```sh
npm run ocr:prepare
SNAPLINGO_OCR_SMOKE_ROOT="$PWD/app/resources/ocr" cargo test --locked -p snaplingo-providers --features local-ocr bundled_medium_smoke -- --ignored --nocapture
```

The explicit resource path prevents the library from guessing the installed app's
layout. `local-ocr` is optional for Providers and enabled by the desktop product.
Native runtime dependencies and models must be present for the smoke test.

```sh
cargo check --locked -p snaplingo --target x86_64-pc-windows-gnu --all-targets
cargo check --locked -p snaplingo --features test-support --target x86_64-pc-windows-gnu --all-targets
SNAPLINGO_SIGNING_MODE=adhoc npm run native:build -- --debug
```

Windows compilation does not establish GUI/IME/UIA or installer behavior. Windows
NSIS/MSI/portable execution is tested on native runners. macOS Intel and minimum
OS-version acceptance are also distinct from Apple Silicon checks. Release and
manual acceptance requirements remain in [Releasing](../RELEASING.md).

The 2026-11-10 native architecture changes passed the macOS native scenarios,
real-model smoke, ordinary/test-support builds, Windows cross compilation, Node
tests and reference build. The debug App and DMG passed resource, native-dependency,
ad-hoc-signature and checksum verification. No installation or publication was
performed by those checks. Generated package evidence lives in
`target/native/debug/verification.json`; it must be regenerated for a new build.

The subsequent legacy-code cleanup on 2026-11-10 passed 223 application checks
and all ten native harnesses, 684 library checks, 63 protocol-only checks, ten
standalone Settings preview tests and 927 browser/script tests. The existing
manual capture benchmark, explicit real-model test and one browser scenario
remain opt-in/skipped in their normal suites. Reference/preview builds, Windows
cross compilation and the ordinary macOS App/DMG build also passed. Unused
dependency removal eliminated 86 lockfile packages without dependency upgrades;
license inventory was regenerated from the resulting lockfiles.
