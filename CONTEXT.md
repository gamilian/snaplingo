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

Execution Commands may call a Coordinator. Provider administration Commands call `ProviderAdministration`; Commands do not read Coordinator state, select an implementation, or persist Provider state themselves.

### Workflow Application

An Application module that owns sequencing and fallback policy across narrower modules. `OcrFavoriteApplication` coordinates Provider, History, and Favorites through local seams. `ProviderAdministration` coordinates Provider configuration, introspection, prompt configuration, and Coordinator state. Commands only convert IPC data and call these interfaces.

### Application Port

A narrow trait declared beside the Application module that consumes it. It describes a capability, not a concrete technology. Composition injects an Infrastructure implementation.

Examples:

- Settings, Hotkeys, and Providers own independent configuration/credential store ports.
- Providers own HTTP transport, LLM runtime, and event-sink ports.
- History owns repository and event-source ports.
- Capture, Result Window, and Pinned Image own their runtime-host ports.

An Application module must not import Infrastructure or crate-root startup adapters in production code.

### Platform Adapter

A concrete Tauri, operating-system, storage, network, or native-library implementation of an Application or frontend port. Platform adapters live under `src/platform/tauri/` on the frontend and `src-tauri/src/infrastructure/` on the backend.

### Composition

The only place that chooses concrete adapters and injects them into a runtime.

- Frontend view entry points create an Application runtime with `src/platform/tauri/*` adapters.
- Backend `src-tauri/src/composition.rs` and its builders create Application runtimes with Infrastructure adapters.
- Infrastructure may conditionally compile platform implementations, but only Composition selects which concrete adapter satisfies an Application seam.

### Capture Session

A frozen snapshot of the desktop used to produce screenshot output, OCR input, and capture-window effects. Capture policy is owned by `application/capture`; OS screenshot and window mechanics are Infrastructure adapters.

### Result Window and Pinned Image

Application runtimes that own their workflows and request runtime-host effects. Result Window owns one read-only View-facing state projection, editable text and language intents, translation/OCR favorite sequencing, OCR Provider fallback, and clipboard intents. Their Tauri window, clipboard, and notification mechanics remain in Infrastructure.

### Hotkey

`domain/hotkey_config.rs` owns the supported category/action vocabulary. `application/hotkeys` owns configuration validation, registration state, display-key parsing, and pressed/released policy. The concrete global-shortcut registrar is an Infrastructure adapter. `startup_shortcuts.rs` only maps a valid category/action to an `AppAction` for startup composition.

### Settings

`application/settings` owns durable setting defaults, normalization, and section updates. Its frontend Settings Configuration module owns hydration, mutation serialization, Provider reload policy, Hotkey snapshots, and cross-window invalidation through narrow ports. Frontend stores only project Application state for Views. UI navigation state is view-local and not durable settings.

### System Speech

The System Speech Application module owns text normalization, persisted voice and rate policy, and locale-based voice selection. A voice exposes an opaque platform ID separately from its display name. Platform adapters own native voice discovery, speech execution, rate conversion, and interruption mechanics; Composition selects the adapter for the current operating system.

### History

The History module turns Application events into stored records and serves history queries. It depends only on its event-source and repository ports.

### Library

The Library is the Settings Window's unified browsing surface for History and Favorites across screenshot, translation, and OCR content. Its frontend Application module owns cross-source filtering, ordering, pagination, and mutation sequencing; Views own rendering and local interaction state. A backend Library Index port performs lightweight global ordering before the frontend loads only the final page's source records and thumbnails.

History and Favorites remain separate backend Application modules with independent persistence and mutation rules. The read-only Library Index combines ordering metadata without merging their storage models.

### Favorite Capacity

Favorite Capacity is the backend Application module that owns the global maximum across translation, OCR, and screenshot Favorites. All favorite insertion workflows share its atomic in-process gate; its repository port reports the combined persisted count without leaking one favorite storage model into another.

### Release Verification

`script/release-verification.mjs` is the shared release module used by local npm commands and native CI. It requires the package, Cargo, and Tauri versions to match, discovers the Cargo target directory through metadata, invokes the Tauri bundle build, and verifies the expected macOS, Linux, or Windows artifacts.

## Dependency Direction

```text
Frontend: Views -> Application -> Platform port -> Tauri
Backend:  Commands -> Application <- Infrastructure
                         ^
                    Composition injects adapters
```

`domain/` contains shared data and vocabulary. It does not depend on Application, Platform, Tauri, or Infrastructure.

## Verification Rules

- Frontend production views and Application modules do not import Platform modules or Tauri packages.
- Backend Application production modules do not import `crate::infrastructure` or crate-root startup adapters.
- Architecture tests enforce these rules; no migration allowlist is retained.
- Native CI runs the frontend test/build and backend format/test/check commands on macOS, Ubuntu, and Windows. It is native verification, not a cross-compilation claim.
