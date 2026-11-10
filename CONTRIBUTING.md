# Contributing to SnapLingo

## Development setup

Use Node.js 22, npm, and a current stable Rust toolchain. The project declares its Rust version in the root `Cargo.toml` workspace metadata; CI currently verifies the stable toolchain rather than a separate minimum-version matrix.

Native dependencies:

- macOS: Xcode Command Line Tools. Official builds use Vision OCR. Tesseract is optional; see [Tesseract setup](docs/TESSERACT_SETUP.md).
- Windows: the x64 Rust MSVC toolchain and Visual Studio C++ Build Tools. Packaging additionally needs NSIS and WiX 3 (`choco install nsis wixtoolset`).
- Linux: portable crates can be developed independently; the complete native application host is not supported.

```bash
npm ci
npm run native:dev
```

## Making changes

Read [AGENTS.md](AGENTS.md), [ARCHITECTURE.md](ARCHITECTURE.md), [CONTEXT.md](CONTEXT.md), and the relevant [ADRs](docs/adr/). Keep changes focused on the reported problem.

Core features co-locate models, workflow policy and the ports they consume. Desktop bootstrap selects concrete adapters and supplies feature dependencies. UI owns presentation and interaction; native hosts own operating-system mechanics. External protocol and local inference implementations live in `crates/providers`; product coordination stays in Core. Tests should exercise the same interface as callers and verify observable outcomes.

SQLite schema changes need a new ordered migration. History and Favorites keep independent storage models. Update current documentation when behavior or ownership changes. Keep lasting requirements and decisions in the product specifications and ADRs; remove temporary implementation plans, progress notes, and review transcripts after completion. Historical product specifications belong in [the archive](docs/archive/README.md).

## Verification

```bash
npm test
npm run reference:build
cargo fmt --all --check
cargo test --locked -p snaplingo --features test-support
cargo test --locked -p snaplingo-core -p snaplingo-storage -p snaplingo-platform -p snaplingo-ui --features snaplingo-ui/rendering,snaplingo-ui/accessibility --all-targets
cargo test --locked -p snaplingo-providers --no-default-features --all-targets
cargo test --locked -p snaplingo-providers --features local-ocr --all-targets
cargo check --locked --workspace --all-targets
cargo check --locked -p snaplingo --features test-support --all-targets
```

Native acceptance targets require `--features test-support`; default product builds
keep their scenario runner and probes disabled. Each native target remains runnable
individually with `cargo test -p snaplingo --features test-support --test <name>`.

Run the relevant focused tests while developing, then complete the checks above before submitting. Native CI covers macOS Apple Silicon, macOS Intel, and Windows. `npm run native:build` creates the platform packages; `npm run release:verify` checks final artifacts. The React/Vite tree is retained only for isolated reference scenes and regression tests; use `npm run reference:dev` to open it. Platform-specific behavior also needs the acceptance scenarios in [Releasing](docs/RELEASING.md). The root Cross/Docker configuration is experimental and is not part of that native CI verification.

## Pull requests

Explain the concrete problem, resulting behavior, and how it was verified. Include screenshots when a visual change benefits from them. State which native platforms you exercised. Preserve existing public behavior unless the change explicitly intends to alter it.

Contributions are distributed under the project's [MIT License](LICENSE).
