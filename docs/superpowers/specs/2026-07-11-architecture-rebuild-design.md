# SnapLingo Architecture Rebuild Design

**Status:** Proposed and user-approved in conversation, pending written-spec review  
**Date:** 2026-07-11  
**Scope:** Frontend Tauri seam, Capture Workspace, Result Window, System OCR, Selected Text Acquisition

## Context

SnapLingo already has several deep modules that should remain intact:

- Provider Coordinators own activation, persistence, execution, and runtime reconfiguration.
- App Action Dispatch gives menu and Hotkey adapters one shared action vocabulary.
- The menu-bar app shell separates application residency from business windows.
- Capture Session and Pinned Image Runtime already place workflow behavior behind Application interfaces.

The remaining architectural friction is concentrated in five areas:

1. Capture Workspace workflow understanding crosses many shallow frontend modules and wide property bags.
2. Frontend Tauri adapters expose raw event strings and raw Tauri window objects to UI modules.
3. Result Window workflow lives in the backend command root with a global payload mailbox and direct clipboard/window mechanics.
4. System OCR constructs an Infrastructure engine from the Application Provider and repeats platform selection across modules.
5. Selected Text Acquisition imports an Infrastructure-owned Registry and method traits; the Registry is shallow.

SnapLingo has not shipped. This rebuild does not preserve old IPC names, serialized payloads, persisted settings, credentials, history, or compatibility adapters. User-visible product behavior should remain recognizable, but internal contracts may change freely when the new module structure is simpler.

## Decision

Rebuild the shared architecture skeleton first, then migrate the five areas onto it. Do not keep old and new runtime paths in parallel.

The target dependency direction is:

    Frontend Views
        ↓
    Frontend Application Modules
        ↓
    Frontend Platform/Tauri Adapters
        ↓ IPC
    Backend Command Adapters
        ↓
    Backend Application Modules
        ↓ owned ports
    Application Composition
        ↓
    Infrastructure Adapters

## Architectural Vocabulary

This design uses the following terms consistently:

- **module**: a cohesive body of behavior hidden behind an interface;
- **interface**: the surface callers use and tests target;
- **depth**: how much implementation complexity an interface hides;
- **seam**: a place where implementation can be substituted;
- **adapter**: an outward implementation of an inward-owned port;
- **locality**: related decisions and state live together;
- **leverage**: one interface serves multiple callers or tests.

The deletion test remains the primary test for shallow modules: deleting a suspected module should concentrate complexity rather than merely move forwarding code to a neighbor.

## Invariants

1. Frontend Views do not import @tauri-apps packages, raw Tauri Window/WebviewWindow types, or event-name strings.
2. Backend Commands parse IPC, obtain AppState, call one Application interface, and convert the final error.
3. Each Application module owns the ports it needs. Infrastructure implements those ports.
4. Application modules never import Infrastructure modules.
5. Application Composition is the only place that selects concrete OS adapters.
6. Platform cfg belongs in Composition or Infrastructure, never in portable Application modules.
7. No generic global ports, interfaces, or shared-type bucket is introduced. A port stays beside the module that consumes it.
8. No compatibility re-export, duplicate command, duplicate event, or temporary dual runtime survives a merged phase.
9. Tests exercise module interfaces and workflows, not source-code strings, except for explicit dependency-rule tests.

## Target Frontend Structure

    src/
    ├─ application/
    │  ├─ capture-workspace/
    │  │  ├─ runtime.ts
    │  │  ├─ state.ts
    │  │  ├─ effects.ts
    │  │  └─ runtime.test.ts
    │  ├─ result-window/
    │  │  ├─ runtime.ts
    │  │  ├─ state.ts
    │  │  └─ runtime.test.ts
    │  └─ selected-text/
    │     └─ types.ts
    ├─ platform/
    │  └─ tauri/
    │     ├─ capture.ts
    │     ├─ resultWindow.ts
    │     ├─ pinnedImage.ts
    │     ├─ settings.ts
    │     ├─ providers.ts
    │     └─ appEvents.ts
    ├─ views/
    │  ├─ CaptureWorkspace/
    │  ├─ ResultWindow/
    │  ├─ PinnedImageWindow/
    │  └─ SettingsWindow/
    ├─ domain/
    ├─ stores/
    └─ App.tsx

The views directory contains rendering modules and view-local presentation state. A window that owns a workflow is not implemented entirely inside a View; its workflow state and decisions live in application.

The platform/tauri directory contains domain-named adapters. It owns command names, event names, payload parsing, window operations, Tauri object lifetime, and subscription cleanup. It does not contain business decisions.

## Target Backend Structure

    src-tauri/src/
    ├─ application/
    │  ├─ capture/
    │  ├─ result_window/
    │  │  ├─ mod.rs
    │  │  ├─ runtime.rs
    │  │  ├─ port.rs
    │  │  └─ tests.rs
    │  ├─ selected_text/
    │  │  ├─ mod.rs
    │  │  ├─ method.rs
    │  │  └─ tests.rs
    │  ├─ providers/
    │  │  └─ ocr/
    │  │     ├─ system_engine.rs
    │  │     └─ ...
    │  └─ pinned_image/
    ├─ infrastructure/
    │  ├─ system/
    │  ├─ storage/
    │  ├─ events/
    │  └─ http/
    ├─ composition/
    ├─ commands/
    ├─ app_state.rs
    └─ lib.rs

The exact filenames may change during planning when an existing module already provides the intended responsibility. The ownership and dependency rules are fixed.

## Module Designs

### Frontend Tauri Adapter

The current src/tauri/events.ts and src/tauri/window.ts are shallow because callers still own event names, payload meaning, and window mechanics. Their replacement is a set of domain-named adapter modules.

Views call operations such as subscribing to a result-window payload, resizing the current result window, revealing a capture window, or closing a pinned window. They do not receive raw Tauri objects.

The adapter modules:

- own command and event names;
- validate and parse IPC payloads;
- translate Tauri position and size types;
- hide subscription cleanup;
- expose portable outcomes to Application modules and Views.

Deletion test: removing a new domain adapter must force callers to reimplement meaningful payload and lifecycle behavior, not merely rename an invoke call.

### Capture Workspace

Capture Workspace becomes one deep frontend Application module.

It owns:

- workflow state;
- selection completion;
- annotation transactions;
- effect planning and interpretation;
- host workflow coordination;
- conversion from pointer, keyboard, and host events into state transitions.

It does not own:

- pure geometry algorithms;
- image composition;
- reusable annotation calculations;
- final JSX rendering;
- Tauri mechanics.

The View consumes a narrow render-state interface and a narrow set of user actions. The root hook no longer constructs large host/editor/input property bags.

The first characterization tests must cover the real workflow path that is currently spread across pointer handling, input coordination, host actions, effect planning, and host runtime execution.

### Result Window

The backend Result Window Application module owns the complete workflow:

    App Action / Command / Selected Text
                    ↓
    Result Window Runtime
                    ↓
    payload state → window port → delivery notification

It owns:

- entrypoint-to-payload policy;
- pending payload lifecycle;
- show/deliver/take ordering;
- failure semantics when the window cannot open or notification cannot be delivered.

It depends on outward ports for:

- result-window mechanics;
- clipboard access when an entrypoint requires it;
- delivery notification.

The command root no longer owns a static mailbox, direct arboard calls, window creation, or event emission.

The pending payload implementation must define atomic behavior:

- a failed window open must not leave an unintended deliverable payload;
- a failed notification must have an explicit retry or discard outcome;
- taking a payload is a single ownership transfer;
- concurrent opens must have deterministic last-write, queue, or rejection semantics selected in the implementation plan.

The frontend Result Window Application module owns mode state, OCR-file workflow coordination, translation triggering, and window presentation decisions. The View renders the state and sends actions.

### System OCR

The System OCR Provider owns an inward engine port beside the Provider module. The port represents OCR capability, not macOS Vision types.

The Provider:

- contains no platform cfg;
- does not call an Infrastructure factory;
- is constructed with an engine implementation;
- keeps Provider-specific language and result semantics.

Composition:

- registers System OCR only when the current target supplies an adapter;
- constructs the concrete adapter;
- injects it into the Provider.

Infrastructure:

- owns macOS Vision mechanics and errors;
- may later contain Windows or Linux adapters without changing Provider behavior.

One adapter is a hypothetical seam; a second implementation would make the seam real. The port is still justified now because the existing Application-to-Infrastructure dependency is reversed and the Provider already has a fake implementation in tests.

### Selected Text Acquisition

Selected Text Acquisition owns:

- method vocabulary;
- method ordering;
- method lookup;
- availability outcomes;
- attempt outcomes;
- diagnostic aggregation;
- success short-circuiting.

OS modules implement method adapters. They do not own the Registry or workflow vocabulary.

The shallow SelectionMethodRegistry is deleted. The Acquirer directly owns its method collection in the representation that best supports ordered lookup.

The current macOS method sequence remains product behavior unless a dedicated product decision changes it:

    SelfWebview → Accessibility → BrowserScript → MenuCopy → ShortcutCopy

Windows and Linux retain ShortcutCopy behavior with explicit platform diagnostics.

Because CONTEXT.md currently names SelectionMethodRegistry, the domain language must be updated in the same phase that deletes it.

## Error Model

Application errors describe domain and workflow failures. They do not contain Tauri, AppKit, Windows, GTK, arboard, or other adapter-specific types.

Infrastructure adapters translate platform failures at the seam. Commands translate the final Application error into an IPC error payload.

Arbitrary strings are not an internal error interface. Strings remain acceptable for:

- user-facing messages;
- structured diagnostic details;
- logs;
- final IPC serialization.

Every workflow that spans state plus an outward effect must document commit ordering and rollback behavior.

## Migration Strategy

The rebuild is delivered through six sequential branches. Each branch starts from the previously merged branch and leaves the application runnable without a temporary dual path.

### Phase 1: Architecture Foundation

- Establish frontend application, platform/tauri, and views ownership rules.
- Add dependency-rule tests.
- Move only the minimum shared vocabulary required for later migrations.

### Phase 2: Frontend Runtime Migration

- Replace raw Tauri event and window access with domain-named adapters.
- Migrate Capture, Result, Pinned, Settings, and App routing callers.
- Delete the old shallow src/tauri modules after all callers move.

### Phase 3: Capture Workspace Deepening

- Add end-to-end workflow characterization tests.
- Consolidate host/editor/input property plumbing.
- Make Capture Workspace Application module the workflow test surface.

### Phase 4: Result Window Deepening

- Introduce backend Result Window Runtime and outward ports.
- Remove mailbox, clipboard, window, and event mechanics from Commands.
- Move frontend Result Window workflow state into its Application module.

### Phase 5: Platform Port Correction

- Move System OCR engine ownership inward and inject adapters through Composition.
- Move Selected Text method ownership inward and delete the Registry.

### Phase 6: Cleanup and Enforcement

- Delete stale modules, re-exports, commands, events, payload vocabulary, and documentation.
- Update CONTEXT.md, ARCHITECTURE.md, runtime map, and relevant ADRs.
- Add native macOS, Windows, and Linux CI verification.

## Test Strategy

Each phase follows red-green-refactor.

### Application Tests

- Test complete workflows through the Application interface with fake adapters.
- Prefer real state transitions over assertions on mock call counts.
- Cover effect failure, retry, rollback, and concurrency where applicable.

### Adapter Contract Tests

- Verify command and event mapping.
- Verify payload parsing and rejection.
- Verify window operation translation.
- Verify platform error conversion.

### View Tests

- Test render state and user actions.
- Do not mock raw Tauri handles.
- Do not assert source-code text as a substitute for behavior.

### Architecture Tests

Automated checks must reject:

- @tauri-apps imports outside src/platform/tauri;
- imports from backend Application into Infrastructure;
- platform cfg in portable backend Application modules;
- raw event-name strings outside their owning adapter;
- old deleted paths and compatibility exports.

### Verification Per Branch

- focused red/green tests;
- full npm test;
- npm run build;
- full cargo test --manifest-path src-tauri/Cargo.toml;
- cargo fmt --check;
- native target verification available in the current environment.

Cross-platform CI must use native runners. macOS cross-compilation to Linux and Windows is not accepted as proof because GTK, Leptonica, Tesseract, and other native dependencies require target sysroots.

## Completion Criteria

- Capture Workspace no longer passes workflow state through layered property bags.
- Frontend Views contain no raw Tauri objects, direct Tauri imports, or event strings.
- Backend Commands contain no Result Window mailbox, clipboard, window, or delivery workflow.
- Backend Application modules do not import Infrastructure.
- Platform selection is localized to Composition and Infrastructure.
- System OCR Provider is portable and injected.
- Selected Text Acquisition owns method order and diagnostics; Registry is deleted.
- Documentation and code use the same module and domain vocabulary.
- Every phase merges in a runnable, fully tested state.

## Non-Goals

- No new product features.
- No Settings UI redesign.
- No new OCR or Translation Provider.
- No dependency upgrades solely to address the two existing npm audit findings.
- No persisted-data migration.
- No compatibility IPC or configuration layer.
- No speculative common framework shared by unrelated modules.

## Risks and Mitigations

### Broad rename churn

Mitigation: foundation first, mechanical moves separate from behavior changes, full test run per branch.

### Tests that only protect pure helpers

Mitigation: write workflow characterization tests before consolidation and make Application interfaces the test surface.

### Temporary duplicate paths

Mitigation: do not merge a phase until the old path and re-exports are deleted.

### Platform regressions

Mitigation: native runner matrix and adapter contract tests; Composition remains the sole adapter selection point.

### Over-abstraction

Mitigation: apply the deletion test; do not introduce a port until an Application module needs to substitute outward mechanics. One adapter remains explicitly hypothetical; two adapters make a seam real.

## Branch Sequence

1. codex/architecture-foundation
2. codex/frontend-runtime-migration
3. codex/capture-workspace-deepening
4. codex/result-window-deepening
5. codex/platform-port-correction
6. codex/architecture-enforcement

The spec and implementation plans are prepared on codex/architecture-rebuild-spec. Implementation begins only after the written spec and plans are reviewed.
