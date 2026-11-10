# SnapLingo

A cross-platform screenshot, OCR, and translation desktop application built with Tauri 2, React, TypeScript, and Rust.

## Features

- Screenshot capture and editing: curved arrows, rounded shapes, resizable text with system fonts, mosaic, blur, region highlights, and spotlights.
- Manual scrolling screenshots with live preview, segment undo, copy/save/pin output, and bounded image sizes.
- Local and remote OCR Providers.
- Multiple Translation Providers with side-by-side results and custom LLM configuration.
- Global hotkeys, configurable editor shortcuts and UI scale, selected-text translation, and pinned images with border and placement settings.
- History and Favorites in one Library browsing surface.

See [Features](docs/FEATURES.md) for current behavior and [Product specifications](docs/prd/README.md) for scope and design references.

## Platform support

| Target | Implementation and verification | Official release packages |
| --- | --- | --- |
| macOS 12+ / Apple Silicon and Intel | Native capture, selection, Vision OCR, and speech; CI builds both architectures. Minimum-version compatibility still requires manual acceptance. | DMG |
| Windows 10/11 x64 | Native capture, selection, system OCR, and speech; CI verifies NSIS/MSI installation and portable startup on Windows Server. Desktop acceptance remains part of release verification. | NSIS EXE, MSI, and portable ZIP |
| Linux | XCap capture, shortcut-copy selection, and Tesseract OCR; Ubuntu CI tests and builds packages. X11/Wayland environments require manual acceptance. System speech and installer-based updates are not implemented. | Not in the official release matrix |

macOS releases use a persistent self-signed identity. Ordinary CI uses ad-hoc signing. See [Releasing](docs/RELEASING.md) for installation, permissions, signing, and acceptance checks.

For scrolling capture, select a region in the screenshot editor and press **S** (configurable), then scroll the original page. **Complete / Enter** copies and saves a PNG; the configurable height limit automatically completes capture. The separate transparent preview passes mouse and wheel input through. Windows portable packages require an installed WebView2 Runtime; see [Windows portable](docs/WINDOWS_PORTABLE.md).

## Project structure

```text
src/
  domain/                  Portable data and domain vocabulary
  application/             Workflows, state transitions, and their ports
  platform/tauri/           Typed IPC, events, and window adapters
  views/                   Window rendering and browser mechanics
  components/common/       Shared UI
  stores/                  Application projections and invalidation signals
  styles/                  Shared styles and themes
src-tauri/src/
  domain/                  Portable Rust domain types
  application/             Capture, OCR, translation, settings, and Library workflows
  infrastructure/          OS, Tauri, SQLite, HTTP, and native-library adapters
  commands/                IPC conversion and Application calls
  composition/             Concrete adapter selection and runtime assembly
docs/
  adr/                     Accepted architectural decisions
  architecture/            Current runtime and ownership maps
  prd/                     Product specifications index
  releases/                Versioned release notes
  archive/                 Historical product requirements
designs/                   Current visual explorations and previews
script/                    Development and release verification
```

Frontend tests live beside the behavior they verify. Rust integration tests live in `src-tauri/tests/`. See [Architecture](ARCHITECTURE.md) and [Domain language](CONTEXT.md) for dependency and ownership rules.

## Development

Use Node.js 22 and a current stable Rust toolchain. Install the native desktop dependencies for your platform as described in [Contributing](CONTRIBUTING.md).

```bash
npm ci
npm run tauri:dev
```

Run frontend verification and the Rust suite:

```bash
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets
```

Build and verify native desktop bundles with `npm run tauri:build`. See [Build scripts](script/README.md) and [Releasing](docs/RELEASING.md) for details.

## Contributing

Read [Contributing](CONTRIBUTING.md) before opening a pull request. Include the problem, resulting behavior, and relevant verification. Current architecture references are [ARCHITECTURE.md](ARCHITECTURE.md), [CONTEXT.md](CONTEXT.md), and [ADRs](docs/adr/).

## License

[MIT](LICENSE). Bundled third-party assets retain their own license notices, including [Noto Sans](src-tauri/assets/fonts/LICENSE-NotoSans.txt).
