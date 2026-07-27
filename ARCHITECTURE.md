# SnapLingo Architecture

## Goal

SnapLingo uses domain-oriented Application modules with explicit inward ports. This keeps business workflows cohesive, makes platform mechanics replaceable, and permits native macOS, Windows, and Linux adapters without changing a workflow.

## Module Map

```text
React Views
  -> frontend Application runtimes
  -> Platform adapters (`src/platform/tauri`)
  -> Tauri commands
  -> backend Application modules
  <- Infrastructure adapters (`src-tauri/src/infrastructure`)
       selected by backend Composition
```

### Frontend

- `src/views/` renders the Settings, Capture Workspace, Result Window, and Pinned Image windows. A view receives an Application runtime through its local runtime context; it does not call Tauri directly. Result Window reads its Application state through one projection instead of selecting multiple Stores.
- `src/application/` owns window workflows and their narrow ports:
  - `capture-workspace`: capture launch/session/effect workflow.
  - `result-window`: translation and file-OCR workflows.
  - `pinned-image`: pinned-image workflow.
  - `settings`: configuration hydration, serialized updates, Provider/Hotkey reload, cross-window invalidation, and Library cross-source filtering, ordering, and pagination. Zustand stores project this Application-owned state for Views.
  - `permissions`: required-permission polling and explicit request workflow.
- Result Window and Settings declare separate narrow speech ports. The shared Tauri adapter implements both without becoming part of either workflow.
- `src/platform/tauri/` owns typed command invocation, event parsing, and Tauri-window effects. It implements the frontend ports.
- `src/domain/` holds portable frontend data types and vocabulary.

### Backend

- `src-tauri/src/commands/` is the IPC adapter seam. A command parses an IPC request, obtains `AppState`, makes one Application call, and converts the result to an IPC response.
- `src-tauri/src/application/` owns business workflows:
  - `providers` owns Provider coordination, administration, runtime configuration, credentials, HTTP/LLM vocabulary, and Provider event publication through local ports.
  - `settings` and `hotkeys` own their independent durable-store ports.
  - `history` owns its repository and event-source ports.
  - `favorite_capacity` owns the global Favorites maximum and serializes capacity check plus insertion across regular and screenshot Favorites.
  - `favorites::OcrFavoriteApplication` owns Provider fallback, History source recovery, Favorite insertion, and OCR Favorite replay through local seams.
  - `library_index` owns lightweight cross-source ordering so only final-page History and Favorites records are hydrated.
  - `capture`, `result_window`, and `pinned_image` own window/runtime-host ports. Result Window also owns its read-only state projection, editable text and language intents, translation/OCR favorite orchestration, OCR Provider fallback, and clipboard intents instead of exposing Store mutations or platform adapters to Views.
  - `selected_text` owns its method and context ports.
  - `required_permissions` owns permission ordering and status policy.
  - `tts` owns speech normalization, locale-based voice selection, and persisted voice/rate policy. Voice IDs are opaque to Application so native adapters may use platform identifiers independently of display names.
- `src-tauri/src/infrastructure/` implements OS, Tauri-window, shortcut, storage, network, native OCR, database, and event capabilities.
- `src-tauri/src/composition.rs` and `composition/*_runtime.rs` construct concrete Infrastructure adapters and inject them into Application runtimes.
- `app_actions.rs`, `app_shell.rs`, `settings_window.rs`, and `startup_shortcuts.rs` are startup-shell adapters. They map menu or hotkey intent to Application-facing actions; they do not own Application state.

## Ownership Rules

| Module | Owns | Must not own |
| --- | --- | --- |
| View | rendering, local interaction state, runtime injection | command names, Tauri listeners, business workflow orchestration |
| Frontend Application | workflow, state transitions, port contracts | Tauri imports and adapter selection |
| Frontend Platform | typed Tauri calls, event payload parsing, window effects | React rendering and workflow policy |
| Command | IPC boundary and error conversion | direct Infrastructure orchestration or duplicate workflow logic |
| Backend Application | workflows, policy, local port contracts | concrete storage, network, Tauri, native-library, or OS mechanics |
| Infrastructure | concrete capability implementations | business policy or provider-selection decisions |
| Composition | concrete adapter selection and construction | reusable workflow policy |

## Representative Seams

```text
Provider Configuration -> ProviderConfigStore / ProviderCredentialStore
Provider Administration -> Provider Coordinators / Provider Configuration / LlmIntrospection
OCR Favorite            -> OcrFavoriteHistory / OcrFavoriteStore / OcrFavoriteRecognizer
Required Permissions   -> RequiredPermissionsHost
System Speech          -> SystemTtsHost
Provider execution     -> HttpClient / LlmRuntime / ProviderEventSink
History                -> HistoryEventSource / HistoryRepository
Library                -> SettingsHistoryPort / SettingsFavoritesPort / SettingsScreenshotFavoritesPort
Library Index          -> LibraryIndexRepository
Favorite Capacity      -> FavoriteCapacityRepository / FavoriteCapacityPolicyProvider
Capture                -> CaptureSessionSource / CaptureRuntimeHost
Hotkeys                -> HotkeyStore / HotkeyRegistrar
```

Each port is declared adjacent to its consumer. There is no generic global port bucket.

## Cross-Platform Strategy

Portable data and policy are kept in Domain or Application. Infrastructure contains OS-specific implementations; Composition selects concrete adapters with target-specific builders:

- Screenshot, selection, system OCR, paths, shortcuts, and desktop windows are Infrastructure concerns.
- Tauri-specific command/event/window APIs are frontend or backend adapter concerns.
- Result-window coordinates are durable settings; the window adapter only measures and applies physical positions.
- Native System OCR is registered on macOS and Windows where the platform language engine is available; Tesseract remains a portable native-engine adapter.
- System Speech uses a target-specific Infrastructure adapter selected by Composition. macOS currently uses `say`; Windows and Linux adapters can be added without changing Application or Commands.
- CI verifies the real desktop targets on macOS, Ubuntu, and Windows rather than claiming cross-compilation coverage.
- `script/release-verification.mjs` owns version consistency, Tauri bundle invocation, Cargo target discovery, and the native artifact contract used by local builds and CI.

## Enforcement

`src/architecture/frontendDependencyRules.test.ts` rejects production imports of Platform/Tauri from Views and frontend Application modules. `src-tauri/tests/architecture_dependency_test.rs` rejects backend Application imports of Infrastructure and crate-root startup adapters. These tests are intentionally strict and contain no migration inventory or allowlist.

## Compatibility Policy

The app is pre-release. IPC payloads, persisted settings, credentials, and history formats may change with the architecture. The removed single-key Provider configuration command is not retained as a compatibility path; see [ADR 0007](docs/adr/0007-remove-provider-compatibility-command.md).
