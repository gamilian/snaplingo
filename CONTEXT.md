# Domain Language

## Project Name

**SnapLingo** is a cross-platform screenshot, OCR, and translation desktop application.

## Core Concepts

### Provider

A pluggable implementation of translation or OCR. A Provider owns its provider-specific request, authentication, response parsing, and credential validation rules. The current built-in implementations include Google Translate, DeepL, Baidu Translation, Tesseract, macOS and Windows System OCR, and Baidu OCR.

### Coordinator

The Application module that owns Provider registration, active-provider state, persistence through its narrow store port, and request coordination.

- `TranslationCoordinator` supports multiple active Providers and runs them concurrently.
- `OcrCoordinator` supports one active Provider.

Provider execution enters a Coordinator. Native administration actions call `ProviderAdministration`; entry adapters do not read Coordinator internals, choose implementations, or persist Provider state themselves.

### Workflow Application

An Application module that owns sequencing and fallback policy across narrower modules. `OcrFavoriteApplication` coordinates Provider, History, and Favorites through local seams. `ProviderAdministration` coordinates Provider configuration, introspection, prompt configuration, and Coordinator state. Native action adapters call these interfaces without duplicating workflow policy.

### Application Port

A narrow trait declared beside the Application module that consumes it. It describes a capability, not a concrete technology. Composition injects an Infrastructure implementation.

Examples:

- Settings, Hotkeys, and Providers own independent configuration/credential store ports.
- Providers own HTTP transport, LLM runtime, and event-sink ports.
- History owns repository and event-source ports.
- Capture, Result Window, and Pinned Image own their runtime-host ports.

An Application module must not import Infrastructure or crate-root startup adapters in production code.

### Platform Adapter

A concrete operating-system, storage, network, or native-library implementation of an Application port. The retained browser references inject isolated fixture ports and have no desktop IPC adapter.

### Composition

The only place that chooses concrete adapters and injects them into a runtime.

- `apps/desktop/src/bootstrap/mod.rs` and its builders create native Application runtimes with Infrastructure adapters.
- `apps/desktop/src/shell/macos.rs` and `apps/desktop/src/shell/windows.rs` own the platform startup/event loops. `reference/web/App.tsx` only opens isolated visual-reference fixtures.
- Infrastructure may conditionally compile platform implementations, but only Composition selects which concrete adapter satisfies an Application seam.

### Capture Session

A frozen snapshot of the desktop used to produce screenshot output, OCR input, and capture-window effects. Capture policy is owned by `crates/core/src/capture`; OS screenshot and window mechanics are Infrastructure adapters.

The selection surface displays the same frozen monitor images used by preview, magnification, OCR, and output. It waits for every image to decode before reveal; selection completion never replaces the session pixels with another desktop capture. Windows uses one primary-display coordinate scale for the capture surface, monitor snapshots, and cursor/window geometry. Physical OS window positions are converted once at that boundary, including pin origins on mixed-DPI displays.

On macOS, the session pins the native selection window and monitor view to the display under the pointer before capture begins. A spanning NSWindow is clipped to one display when displays have separate Spaces; the session retains all monitor layouts for native cursor coordinates and output.

### File OCR

The workflow that reads one image file, uses those same bytes for both OCR input and its preview, and returns recognition metadata. File access is supplied by an adapter; Provider selection and execution remain with the OCR Coordinator.

### Selection Translation

The workflow that reads the current selection-text mode, acquires selected text, and presents it in the Result Window with Selection origin and automatic translation. Hotkey and native menu entry points share the same sequencing; failed acquisition leaves the current result intact.

### Capture OCR Delivery

The handoff of a Capture Session OCR result to the OCR window, translation window, or clipboard. Window handoffs reserve a Result Window request identity; a newer request supersedes older results and errors. Clipboard delivery does not replace a pending Result Window request.

### Capture Editing

The editing state within a Capture Session: selection geometry, annotation gestures, selected annotations, text drafts, styles, and undo/redo history. Pointer, keyboard, and toolbar intents share its state transitions. An edit commit publishes a consistent snapshot; Capture Session loading, cancellation, and output remain separate workflows.

### Scrolling Capture

A sequence of images from one selected viewport while the user scrolls its content. Matching overlapping content produces one long image, retaining fixed headers and footers once. The latest appended segment can be undone without discarding earlier content. A Scrolling Capture is distinct from the frozen desktop image of an ordinary Capture Session. Complete copies and saves a PNG; reaching its configured height limit invokes the same output. A separate transparent Preview observes the task and passes mouse input through. A transparent viewport outline indicates the original capture region and also passes mouse input through.

### Result Window and Pinned Image

Application runtimes that own their workflows and request runtime-host effects. Result Window owns one read-only View-facing state projection, editable text and language intents, translation/OCR favorite sequencing, OCR Provider fallback, and clipboard intents. Their native window, clipboard, and notification mechanics remain in Infrastructure. Windows pin placement preserves physical origins and centers/cascades against the virtual desktop without changing render DPI.

### Hotkey

`crates/core/src/hotkeys/model.rs` owns the supported category/action vocabulary. `crates/core/src/hotkeys` owns configuration validation, registration state, display-key parsing, and pressed/released policy. The concrete global-shortcut registrar is an Infrastructure adapter. `apps/desktop/src/shell/actions/shortcuts.rs` only maps a valid category/action to an `AppAction` for startup composition.

### Settings

`crates/core/src/settings` owns durable setting defaults, normalization and section updates. The portable Settings session owns hydration, mutation ordering, Provider reload, Hotkey snapshots and invalidation through narrow ports. Both native hosts use it; Composition injects concrete workflows and native effects. Shared UI projects snapshots into controls. Navigation remains presentation state; Services route restoration uses a dedicated UI namespace rather than adding fields to General settings. The retained frontend configuration/stores exercise the original browser contracts only.

### System Speech

The System Speech Application module owns text normalization, persisted voice and rate policy, and locale-based voice selection. A voice exposes an opaque platform ID separately from its display name. Platform adapters own native voice discovery, speech execution, rate conversion, and interruption mechanics; Composition selects the adapter for the current operating system.

### History

The History module turns Application events into stored records and serves history queries. It depends only on its event-source and repository ports.

### Library

The Library is the Settings Window's unified browsing surface for History and Favorites across screenshot, translation, and OCR content. Its application adapter owns cross-source filtering, ordering, pagination and mutation sequencing; shared UI owns rendering and local interaction state. The portable Library Index performs lightweight global ordering before the adapter loads only the final page's source records and thumbnails.

Library hydrates the selected records by stable ID rather than source offsets. Its history-page projection includes favorite membership for only the visible content identities, without loading all Favorite records or thumbnails.

History and Favorites remain separate backend Application modules with independent persistence and mutation rules. The read-only Library Index combines ordering metadata without merging their storage models.

### Favorite Capacity

Favorite Capacity is the backend Application module that owns the global maximum across translation, OCR, and screenshot Favorites. All favorite insertion workflows share its atomic in-process gate; its repository port reports the combined persisted count without leaking one favorite storage model into another.

### Release Verification

`script/release-verification.mjs` is the shared release module used by local npm commands and native CI. It requires npm/Cargo manifests and locks to agree on the version, discovers Cargo's target directory, invokes `native-build.mjs`, and verifies macOS app/DMG or Windows NSIS/MSI/portable artifacts under `target/native/`. `desktop.json` owns stable application and installer identity; it does not duplicate the version. Release filenames and SHA-256 manifests remain compatible with the update selector.

macOS verification mounts the final DMG and checks signing identity, dependencies, architecture and deployment targets. The release workflow imports one persistent self-signed identity into a temporary keychain without changing system trust; ordinary CI uses ad-hoc signing. Windows packaging verifies the native PE and bundled OCR/CRT/license tree, and native runner smoke tests cover installation and startup. Compilation/linking success is distinct from those runtime checks. See `docs/RELEASING.md` for setup and manual acceptance.

### Permissions and Software Updates

`application/required_permissions` reads current OS grants and targets explicit requests/repairs to one permission. The system adapter detects temporary macOS installs, scopes TCC resets to this app and one service, and handles user-requested restarts. The first-use guide is confined to the Settings Window; the persistent General permissions section shares its native controller.

`application/updates` selects newer stable GitHub releases and verified installers for the running platform/architecture. Its system adapter downloads to a private temporary directory, checks size and SHA-256, marks the internet origin, and opens the system installer. The native General controller checks once per settings session and serializes explicit checks/downloads. This uses the existing free release packages and requires no Apple Developer ID; it does not silently replace the app. Stable self-signed releases keep the same code identity; migrating from historical ad-hoc builds may require granting permissions again, and macOS controls permission retention.

## Dependency Direction

```text
Desktop bootstrap -> Core + UI + Platform + Storage
Platform / Storage -> consumer-owned Core contracts
Desktop features -> Core workflows + UI scenes/intents
Native hosts -> shared UI presentation + optional render/accessibility modules
Browser references -> original Views/Application + isolated fixture ports
```

Core feature-local `model.rs` files contain business data and vocabulary. They do not depend on workflows, UI, Platform, Storage or the desktop application.

## Verification Rules

- Browser reference Views and Application modules do not import desktop Platform modules or Tauri packages; the old production adapters are deleted.
- Core production modules do not import concrete adapters or desktop startup code. Desktop features receive explicit dependencies and do not import AppState or bootstrap.
- Architecture tests enforce these rules; no migration allowlist is retained.
- Application CI targets macOS Apple Silicon, macOS Intel and Windows and runs native build/verification plus installer smoke. The React reference build and tests remain independent tools. Native runtime acceptance is not inferred from Windows cross compilation or PE linking.
