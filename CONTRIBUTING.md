# Contributing to SnapLingo

## Development setup

Use Node.js 22, npm, and a current stable Rust toolchain. The project declares its Rust version in `src-tauri/Cargo.toml`; CI currently verifies the stable toolchain rather than a separate minimum-version matrix.

Native dependencies:

- macOS: Xcode Command Line Tools. Official builds use Vision OCR. Tesseract is optional; see [Tesseract setup](docs/TESSERACT_SETUP.md).
- Windows: the Rust MSVC toolchain, Visual Studio C++ Build Tools, and WebView2 Runtime.
- Ubuntu: the packages installed by [.github/workflows/ci.yml](.github/workflows/ci.yml), including GTK 3, WebKitGTK 4.1, Ayatana AppIndicator, Tesseract/Leptonica, PipeWire, Clang, and libxdo.

```bash
npm ci
npm run tauri:dev
```

## Making changes

Read [AGENTS.md](AGENTS.md), [ARCHITECTURE.md](ARCHITECTURE.md), [CONTEXT.md](CONTEXT.md), and the relevant [ADRs](docs/adr/). Keep changes focused on the reported problem.

Application modules own workflow policy and declare the ports they consume. Composition selects concrete adapters. Views own rendering and browser mechanics. Provider protocol rules remain with the Provider module. Tests should exercise the same interface as callers and verify observable outcomes.

SQLite schema changes need a new ordered migration. History and Favorites keep independent storage models. Update current documentation when behavior or ownership changes. Keep lasting requirements and decisions in the product specifications and ADRs; remove temporary implementation plans, progress notes, and review transcripts after completion. Historical product specifications belong in [the archive](docs/archive/README.md).

## Verification

```bash
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets
```

Run the relevant focused tests while developing, then complete the checks above before submitting. Native CI covers macOS Apple Silicon, macOS Intel, Ubuntu, and Windows. Platform-specific behavior also needs the acceptance scenarios in [Releasing](docs/RELEASING.md). The root Cross/Docker configuration is experimental and is not part of that native CI verification.

## Pull requests

Explain the concrete problem, resulting behavior, and how it was verified. Include screenshots when a visual change benefits from them. State which native platforms you exercised. Preserve existing public behavior unless the change explicitly intends to alter it.

Contributions are distributed under the project's [MIT License](LICENSE).
