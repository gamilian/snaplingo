# SnapLingo Architecture

## Goal

SnapLingo uses domain-oriented Application modules with explicit inward ports. This keeps business workflows cohesive and shares policy between the native macOS and Windows hosts. Portable libraries can support other platforms independently of the complete application host.

## Module Map

```text
Production native path
  snaplingo-ui       presentation state, layout, hit testing -> Scene + Action
  snaplingo-render   Scene -> CPU pixels (tiny-skia + cosmic-text)
  snaplingo-accessibility  Scene + UI state -> AccessKit semantics
  snaplingo-desktop  native windows, input, DPI, presentation (winit + softbuffer)
       |
       | injected SettingsBackend
       v
  snaplingo-settings  portable Settings sessions and shared UI projection
       v
  snaplingo-core    Domain/Application workflows and inward ports
       ^
  snaplingo-storage SQLite repositories and filesystem assets

  snaplingo-assets  bundled fonts shared by Core and Render
  assets/           production defaults, text and third-party notices

Application host
  snaplingo-app native entry -> AppKit / winit event loop
       -> AppState + Composition -> Core workflows + Infrastructure adapters
       -> shared Settings UI / native capture, result, pin and scrolling hosts

Reference-only browser tree
  reference/web/App -> isolated React fixtures -> retained Views/Application tests
       (no desktop IPC; excluded from native packages)
```

### Rust workspace

The portable Domain/Application sources live in `crates/core`; persistence lives in `crates/storage`. App and Storage import domain values and workflows directly from `snaplingo_core`; neither re-exports Core's module tree. The app Cargo package is named `snaplingo`. The former `src-tauri` root, Tauri commands, shell feature, configuration and frontend IPC adapters have been removed.

| Crate | Responsibility | Production workspace dependencies |
| --- | --- | --- |
| `snaplingo-assets` | Bundled font bytes and licenses | None |
| `snaplingo-core` | Domain types, business workflows, adapter contracts | Assets |
| `snaplingo-storage` | SQLite migrations/repositories, configuration/credentials, persisted image assets | Core |
| `snaplingo-ui` | Shared Settings projection, logical layout, themes, hit testing, focus and interaction intents | None |
| `snaplingo-render` | CPU shapes, text shaping/fallback and rasterization into premultiplied RGBA | UI, Assets |
| `snaplingo-accessibility` | Shared scene bounds, roles, values and actions projected into AccessKit | UI |
| `snaplingo-settings` | Portable Settings sessions, ordered mutations and UI projections | Core, UI |
| `snaplingo-desktop` | Native window/event host, device scale, AX/UIA adapters, dialogs, redraw scheduling and opaque pixel presentation | UI, Render, Accessibility |
| `snaplingo-app` | Native application entry, platform integrations, shared Settings adapters, composition and lifecycle | Core, Storage, Assets, UI, Render, Accessibility, Settings, Desktop |

The `settings_preview` example owns its temporary Storage database and registrar composition; it reuses the common preference projection and hotkey mutation workflow. Its recording-only registrar does not install global shortcuts. Production startup uses the complete native application host.

### Native application hosts

The default `native-app` feature supplies the native entry used by `main.rs` and
`native_main.rs`. `lifecycle/macos.rs` owns the macOS AppKit event loop and application
delegate; `lifecycle/windows.rs` owns the Windows winit loop and native event routing.
Both compose `AppState`, tray actions, global shortcuts, single-instance activation,
and native capture/result/pinned hosts plus `window/scrolling` adapters. `RuntimeLifecycle` owns startup
history work, log cleanup and hotkey registration. Quit cancels/drains work before
releasing platform owners; closing Settings hides it while the resident app runs.

`window/settings/macos/host.rs` owns the macOS Settings `NSWindow` and shared canvas;
`window/settings/windows/mod.rs` connects the Windows window host to the portable
`SettingsWindowSession`. All seven page kinds use shared Rust layout and actions.
`composition/settings` injects real workflows and native effects into the
portable `snaplingo-settings` session. Both platform hosts use that session for
preferences, providers, library queries, capacity totals, permissions and updates.
`SharedFeaturePage` retains macOS text composition, native dialogs and accessibility;
`snaplingo-desktop` supplies normalized Windows input, IME and UIA dispatch.
Capacity and permission sheets have explicit native lifecycle owners. Device DPI
remains separate from saved interface zoom and durable window geometry.

`npm run dev` and `npm run build` use the native entry. Packaging stages resources
under `target/native/`, preserves the application/signing/installer identities,
and includes no browser assets. Windows compilation and PE linking have passed;
Windows GUI and installer runtime acceptance are distinct native-runner checks.
Mac native workflow tests, portable adapter tests and independent React/AppKit
layout references cover the corresponding behavior without replacing that
platform acceptance boundary.

Both macOS and Windows are required for this shared path. UI/layout must stay in Rust and be shared; creating a second platform-specific widget tree for each page would duplicate those rules. Native adapters still own IME integration, accessibility, dialogs, menus, capture mechanics and transparent window presentation. Softbuffer accepts opaque `0x00RRGGBB`; alpha overlays/pins need platform presenters consuming the same shared RGBA frame. See [ADR 0012](docs/adr/0012-shared-rust-ui-and-platform-hosts.md) and the [source-based investigation](docs/research/2026-11-08-cross-platform-rust-ui.md).

Pinned Image presentation lives in `ui/pinned`: transforms, input policy, toolbar
and lens geometry, and the Scene share one owner. `app/window/pinned` owns decoded
pixels, placement and adjacent macOS/Windows adapters. The AppKit adapter consumes
the shared state and geometry while retaining native image presentation, buttons,
menus and dragging. The Windows adapter renders the shared Scene into its alpha
presenter. UI has no image decoder or Core dependency.

The shared accessibility crate projects drawn Scene geometry into an AccessKit tree; the desktop host supplies native macOS AX and Windows UIA adapters, and the embedded AppKit canvas supplies its own native AX adapter. Navigation/section selection, toggle states, choice values/popups, range values, hotkey state and disabled actions come from the shared UI projection. Accessibility actions enter the same validated Focus/Activate/SetValue/key/scroll event path as regular input. Adapters cache a complete tree for synchronous activation and publish only changed nodes. These semantics and geometry are tested; actual VoiceOver/Narrator and text-field acceptance remains required. Shared fonts live in the root `assets/fonts` directory, used by Core output and shared rendering through Assets.

### Browser references

The `reference/web/` React tree retains original Views, Domain/Application workflows and
behavior tests as independent references. `reference/web/App.tsx` forwards Vite requests to
isolated Settings, Result or Permission fixtures under
`reference/web/fixtures`, preserving scene query parameters. These
fixtures mount original components with test data. `reference:dev`,
`reference:build` and `reference:preview` serve this harness; desktop startup and
packaging do not execute it.

- `reference/web/views/`, `reference/web/components/` and `reference/web/styles/` retain the original browser UI.
- `reference/web/application/` retains testable capture, result, scrolling, settings, library
  and permission workflows; local runtime contexts inject fixture/test ports.
- `reference/web/stores/` projects those runtime snapshots, and `reference/web/domain/` retains their
  portable types and vocabulary.
- `reference/web/platform/tauri` and the Tauri npm API/CLI dependencies are deleted. There
  is no production browser IPC boundary or browser fallback for native windows.

### Backend

- `crates/core/application/` owns business workflows:
  - `providers` owns Provider coordination, administration, runtime configuration, credentials, HTTP/LLM vocabulary, and Provider event publication through local ports. Its File OCR workflow reads through `ImageFileReader` and uses identical bytes for recognition and preview.
  - `settings` and `hotkeys` own their independent durable-store ports.
  - `history` owns its repository and event-source ports.
  - `favorite_capacity` owns the global Favorites maximum and serializes capacity check plus insertion across regular and screenshot Favorites.
  - `favorites::OcrFavoriteApplication` owns Provider fallback, History source recovery, Favorite insertion, and OCR Favorite replay through local seams.
  - `library_index` owns lightweight cross-source ordering so only final-page History and Favorites records are hydrated.
  - `capture`, `result_window`, and `pinned_image` own window/runtime-host ports. Capture owns startup exclusion, failure cleanup, frozen monitor coordinates, and output delivery. Its pin shortcut selects an active visible Capture Session before delegating idle clipboard pinning. Its `CapturePinOutput` port is implemented by the existing Pinned Image runtime and wired by Composition; Native action adapters do not chain the two workflows. `CaptureOcrDelivery` owns OCR target mapping and stale-request policy; Composition injects the existing Result Window runtime and native clipboard/error effects instead of resolving them through global AppState.
  - `selected_text` owns its method and context ports. `SelectionTranslation` reads current Settings, acquires selected text, and opens Result Window; Hotkey and native menu adapters share this workflow.
  - `required_permissions` owns permission ordering and status policy.
  - `tts` owns speech normalization, locale-based voice selection, and persisted voice/rate policy. Voice IDs are opaque to Application so native adapters may use platform identifiers independently of display names.
- `crates/app/infrastructure/` implements OS window, shortcut, network, native OCR and event capabilities. Its `storage` name re-exports the shared `snaplingo-storage` crate; database and asset implementations live there.
- Scrolling Capture remains in `application/capture`: it reuses the frozen layout while its host supplies fresh pixels and manages the compact control window, transparent input-pass-through preview and viewport outline, and temporary keys. The preview consumes revisioned updates without starting a second sampler. A pure stitcher owns overlap matching, segment undo, image limits, and thumbnails. Completion copies and saves PNG, including at the configurable image limit. System annotation fonts are supplied through the composer's font-provider port. The Capture runtime also retains the last annotation tool for this process, surviving native capture window recreation without persisting it to the database.
- The macOS scrolling host owns its single 250 ms sampler, starts it after session adoption, pauses during output or errors, and stops it on close. Native controls update only changed properties and previews decode only changed PNGs. Native pinned views preserve their presentation and compressed PNG while hidden, release decoded/filter/inspection buffers, and restore those buffers when shown. Pin actions reuse Pinned Image, Settings, and Capture Output workflows.
- Native Capture Editor owns portable annotation gestures, selection, styles, text drafts and bounded undo/redo in `application/capture`. Its AppKit adapter renders the frozen crop and transient annotations through the existing Rust compositor, coalesces preview requests, rejects stale revisions, and uses NSTextView for text input. The same native window continues from selection into editing; it does not hand the session to a WebView. Outputs and the global pin shortcut consume the editor's annotation state through Capture runtime.
- `crates/app/composition.rs` and `composition/*_runtime.rs` construct concrete Infrastructure adapters and inject them into Application runtimes.
- `actions/mod.rs`, `lifecycle/shell.rs`, `lifecycle/macos.rs`, `lifecycle/windows.rs` and `actions/shortcuts.rs` are native startup/action adapters. They map menu or hotkey intent to Application-facing actions; they do not own Application state.
- System integration uses Rust libraries directly: `fern` logs into SQLite, `global-hotkey` owns registered key events, `rfd` supplies native dialogs, `auto-launch` manages login entries, and `tray-icon`/`muda` own tray menus. Single-instance ownership uses an OS file lock plus authenticated loopback activation. `infrastructure/runtime.rs` owns Tokio workers; `ui_thread` dispatches effects onto the platform event loop.
- Capture, editor, scrolling, result and pinned-image hosts call injected Rust services. AppKit supplies macOS native text controls and window presentation; Windows supplies winit input, native menus/dialogs and layered alpha presentation. Hidden editors retain frozen state for failed scrolling handoffs. Windows pin positions cross the OS boundary as physical coordinates, with capture logical origins converted through the primary desktop-scale policy and bounded virtual-desktop centering.
- Application services publish typed notifications through `infrastructure/desktop_events.rs`. Native Settings/menu subscribers refresh through weak subscriptions; there is no frontend event bridge. `build_app_state` accepts paths, identity/version and selected-text inputs through `CompositionOptions`. The macOS selected-text adapter reads native text responders; native menu and single-instance callbacks publish Rust actions.
- Shared Settings canvases reuse the same scene, renderer and accessibility projection on both platforms. Services and Library adapters have real SQLite race/order tests; cached capacity totals refresh asynchronously through `LibraryIndex` without blocking UI drawing or storing usage in General settings. Independent React DOM and AppKit geometry fixtures remain visual acceptance sources.

## Ownership Rules

| Module | Owns | Must not own |
| --- | --- | --- |
| Reference View | original browser rendering, local interaction state, fixture/runtime injection | desktop IPC and business workflow orchestration |
| Reference Application | retained workflow, state transitions, test port contracts | desktop platform imports and adapter selection |
| Backend Application | workflows, policy, local port contracts | concrete storage, network, Tauri, native-library, or OS mechanics |
| Shared UI | presentation state, layout, input behavior, scene and action projection | Application service construction, storage, renderer/window/OS types |
| Shared Renderer | shaping, glyph/shape rasterization and pixel format | business policy, window lifetime or input routing |
| Desktop Host | OS events/windows, DPI/IME, frame presentation and injected backend dispatch | duplicate page layouts, colors or business workflows |
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
File OCR               -> ImageFileReader / OcrCoordinator
Local PaddleOCR        -> PaddleOcrEngine (models, sessions, and platform backends in Infrastructure)
Selection Translation  -> SettingsConfiguration / SelectedTextAcquirer / ResultWindowRuntime
Capture OCR Delivery   -> ResultWindowRuntime / CaptureOcrEffects
History                -> HistoryEventSource / HistoryRepository
Library                -> SettingsHistoryPort / SettingsFavoritesPort / SettingsScreenshotFavoritesPort
Library Index          -> LibraryIndexRepository
Favorite Capacity      -> FavoriteCapacityRepository / FavoriteCapacityPolicyProvider
Capture                -> CaptureSessionSource / CaptureSessionRuntimeHost
Capture Pin output     -> CapturePinOutput (implemented by PinnedImageRuntime)
Hotkeys                -> HotkeyStore / HotkeyRegistrar
```

Native action adapters can call the clipboard capability directly when no business workflow is involved; they do not introduce an empty Application module.

Each port is declared adjacent to its consumer. There is no generic global port bucket.

## Cross-Platform Strategy

Portable data and policy are kept in Domain or Application. Infrastructure contains OS-specific implementations; Composition selects concrete adapters with target-specific builders:

- Shared UI scenes use logical DIP and a separate interface scale; the renderer/host applies the device scale. The same scene supplies snapshots on both required platforms. Device DPI changes must not alter stored user zoom or desynchronize hit testing from pixels.
- Screenshot, selection, system OCR, paths, shortcuts, and desktop windows are Infrastructure concerns.
- Result-window coordinates are durable settings; the window adapter only measures and applies physical positions.
- Capture sessions freeze monitor geometry with their pixels. Windows retains one primary-monitor scale for the capture desktop; macOS uses its native logical-coordinate rules. Window preparation/reveal, cursor mapping, control detection, and output use the requested session's geometry, including after a refresh.
- Native System OCR is registered on macOS and Windows where the platform language engine is available; Tesseract remains a portable native-engine adapter.
- System Speech uses a target-specific Infrastructure adapter selected by Composition: `MacOsSystemTtsHost` on macOS, `WindowsSystemTtsHost` on Windows, and `UnavailableSystemTtsHost` on Linux, where native speech is not yet implemented.
- The full application CI matrix runs macOS Apple Silicon, macOS Intel and Windows. Portable-crate checks, Windows compilation/linking, GUI smoke and manual desktop acceptance remain separate evidence.
- `script/release-verification.mjs` owns npm/Cargo version consistency, native-build invocation, Cargo target discovery, SHA-256 collection and the native artifact contract used by local builds and CI.

## Enforcement

`reference/web/architecture/frontendDependencyRules.test.ts` rejects production imports of Platform/Tauri from Views and frontend Application modules. `crates/app/tests/architecture_dependency_test.rs` rejects backend Application imports of Infrastructure and crate-root startup adapters. They also reject outward Domain dependencies. These tests are intentionally strict and contain no migration inventory or allowlist.

`crates/ui/tests/workspace_architecture.rs` checks real Cargo metadata, including all target/build dependencies and renamed package dependencies. Core may depend on Assets but cannot depend on another workspace crate or a concrete native UI/OS adapter. Storage depends inward on Core; UI permits only its Unicode editing utility; Render, Accessibility and Settings have explicit inward dependency lists; Desktop depends on UI, Render and Accessibility plus native window/input/dialog adapters; Settings workflow construction is injected by the application. The App crate owns concrete OS dependencies and has no Tauri production dependency. A Rust syntax traversal follows Core's production module graph, rejecting concrete platform imports and Rust modules outside Core. Core has no cross-crate test source mounts; real adapter workflow tests live in App integration tests, while Core uses memory adapters. Dev dependencies used by isolated acceptance harnesses do not become production dependency edges.

The shared-native CI workflow checks Core, Storage, UI, Render and Desktop on macOS and Windows and uploads rendered references. The application CI and release workflows package the same native entry and run installation smoke tests; these checks do not certify manual IME, accessibility or desktop screenshot acceptance.

Capture guards also cover the retained browser runtime's Application location and absence of View/DOM dependencies across all Application modules, native startup/Pin policy and file/native mechanics staying behind their application ports, and native window/control adapters using frozen session coordinates. Runtime interface tests verify scheduling, stale-result rejection, readiness, cancellation, overlapping startup, failure recovery, and frozen-pixel delivery.

## Compatibility Policy

The app is pre-release. Action contracts, persisted settings, credentials, and history formats may change with the architecture. The removed single-key Provider configuration command is not retained as a compatibility path; see [ADR 0007](docs/adr/0007-remove-provider-compatibility-command.md).

Scrolling Capture's shared sampler/action lifetime lives in
`crates/app/window/scrolling/session.rs`; `macos/` and `windows/` retain
native window/sample mechanics and share cancellation-safe restoration.

Business-window adapters live in `app/window/{settings,capture,result,pinned,scrolling}`,
with platform implementations adjacent in `macos/` and `windows/`. Screenshot
acquisition, selection, OCR, speech and HTTP remain infrastructure capabilities.
`actions/` owns entry routing; `lifecycle/` owns platform event loops, startup,
resident tray behavior and shutdown. Settings navigation stays with its window
owner; permission recovery uses an injected callback without reaching into the
application event loop.

Development executables have separate owners. `tools/reference` is an unpublished
Cargo package with the macOS WebKit collectors and no production-crate dependency;
App no longer has WebKit even as a development dependency. Production crates
cannot depend on tool packages. `tools/native/pin_snapshot.rs` remains an explicit
App example target so it exercises the actual private pixel-preparation module.
`single_instance_native` is an automatically run App integration test with real
child processes. Reusable UI/Render/Desktop examples stay beside their libraries.
