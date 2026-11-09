# SnapLingo

A screenshot, OCR, and translation desktop application for macOS and Windows. Rust owns the application workflows, storage, shared UI drawing and native platform hosts. The previous React views remain isolated visual references and regression tests.

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
| Linux | Portable Rust libraries retain Linux support; the complete native application host is not implemented. | Not supported |

macOS releases use a persistent self-signed identity. Ordinary CI uses ad-hoc signing. See [Releasing](docs/RELEASING.md) for installation, permissions, signing, and acceptance checks.

For scrolling capture, select a region in the screenshot editor and press **S** (configurable), then scroll the original page. **Complete / Enter** copies and saves a PNG; the configurable height limit automatically completes capture. The separate transparent preview passes mouse and wheel input through. Windows portable packages include OCR and runtime dependencies; see [Windows portable](docs/WINDOWS_PORTABLE.md).

## Project structure

```text
src/
  domain/                  Portable data and domain vocabulary
  application/             Workflows, state transitions, and their ports
  views/                   Retained React visual references
  components/common/       Shared UI
  stores/                  Application projections and invalidation signals
  styles/                  Shared styles and themes
crates/
  snaplingo-assets/        Shared bundled font bytes and licenses
  snaplingo-core/          Portable Rust Domain/Application workflows and ports
  snaplingo-storage/       SQLite repositories/migrations and persisted image assets
  snaplingo-ui/            Shared Rust Settings layout, input state, scenes and actions
  snaplingo-render/        CPU drawing with tiny-skia and cosmic-text
  snaplingo-accessibility/ Shared scene semantics and validated native actions
  snaplingo-settings/      Persistent settings workflows and shared UI projections
  snaplingo-desktop/       Native window/input/DPI host with winit and softbuffer
  snaplingo-app/src/       Native application entry and platform integrations
    infrastructure/        OS, HTTP and native-library adapters
    composition/           Concrete adapter selection and runtime assembly
docs/
  adr/                     Accepted architectural decisions
  architecture/            Current runtime and ownership maps
  prd/                     Product specifications index
  releases/                Versioned release notes
  archive/                 Historical product requirements
designs/                   Current visual explorations and previews
script/                    Development and release verification
```

Frontend tests live beside the behavior they verify. Rust integration tests live in `crates/snaplingo-app/tests/`. See [Architecture](ARCHITECTURE.md) and [Domain language](CONTEXT.md) for dependency and ownership rules.

The Rust application lives under `crates/snaplingo-app`; historical research and ADRs may refer to its former `src-tauri` path.

## Development

Use Node.js 22 and a current stable Rust toolchain. Install the native dependencies
in [Contributing](CONTRIBUTING.md), then start the native debug application:

```bash
npm ci
npm run dev
```

`dev` prepares local OCR resources, builds `snaplingo-native`, and runs it. Use
`npm run native:run` to rerun the existing debug executable. The application uses
the normal database and single-instance lock; an existing instance is activated.

Build application packages with `npm run native:build` (also `npm run build`).
macOS produces a signed app and DMG. Windows requires NSIS and WiX 3 and produces
NSIS/MSI installers and a portable ZIP. Local macOS package checks can use
`SNAPLINGO_SIGNING_MODE=adhoc npm run native:build -- --debug`.
`npm run release:build` adds the complete release verification, including the
final DMG contents. See [Build scripts](script/README.md) and
[Releasing](docs/RELEASING.md) for signing and installer acceptance.

The complete app includes the tray, global shortcuts, capture/result/pinned
windows, seven Settings pages and capacity dialogs. General, Screenshot,
Services, History and Favorites reuse portable workflows and storage. The shared
CPU UI is checked against the retained React and AppKit references. Native runner
checks and manual desktop acceptance are recorded separately; a successful cross
compile does not run the Windows UI.

The React browser build is a reference harness with isolated fixtures and no
desktop IPC. Run it separately:

```bash
npm run reference:dev
npm run reference:build
```

Queries such as `/?page=services&route=tts&appearance=dark` select the original
Settings scenes. `/?surface=result&mode=ocr` and `/?surface=permission` open other
references. Native packages include no browser assets. See [reference usage](src/README.md).

Run the relevant regression suites:

```bash
npm test
cargo fmt --all --check
cargo test --locked -p snaplingo --lib
cargo test --locked -p snaplingo --test native_app_smoke
cargo check --locked -p snaplingo --all-targets
cargo test --locked -p snaplingo-core -p snaplingo-storage -p snaplingo-ui -p snaplingo-render -p snaplingo-accessibility -p snaplingo-settings -p snaplingo-desktop --all-targets
```

`native_app_smoke` uses a temporary database. On macOS and Windows it exercises
the native application lifecycle and real Settings adapter. Renderer examples
write visual comparison artifacts, for example
`cargo run --locked -p snaplingo-render --example settings_snapshot`.

## Contributing

Read [Contributing](CONTRIBUTING.md) before opening a pull request. Include the problem, resulting behavior, and relevant verification. Current architecture references are [ARCHITECTURE.md](ARCHITECTURE.md), [CONTEXT.md](CONTEXT.md), and [ADRs](docs/adr/).

## License

[MIT](LICENSE). Bundled third-party assets retain their own license notices, including [Noto Sans](assets/fonts/LICENSE-NotoSans.txt).
