# Architecture Followups Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete architecture review Candidates 2, 3, 4, and 5, plus the cross-platform cleanup items, while explicitly deferring Candidate 6.

**Architecture:** Execute this as five small refactors with independent verification: first narrow `AppState`, then remove remaining DeepLX dual-state leakage, then deepen backend capture-session rendering, then finish the frontend Capture Workspace shell extraction, and finally delete/align stale cross-platform code and docs. Preserve all IPC command names and frontend-visible data shapes.

**Tech Stack:** Rust, Tauri, React, TypeScript, Vitest, Cargo tests, existing SnapLingo provider/capture modules.

---

## Scope

In scope:
- Candidate 2: split `AppState` into runtime slices and remove raw `config_file` / `keychain` / `http_client` access from app-managed state.
- Candidate 3: move DeepLX/DeepL credential validation knowledge out of commands/configuration branching and into the provider-facing interface.
- Candidate 4: finish thinning `src/components/ScreenshotSession/index.tsx` into a composition shell.
- Candidate 5: remove the pseudo seam in `capture_session_render.rs` by making render/output helpers free functions used by `CaptureSessionRuntime`.
- Cross-platform cleanup: delete the unused legacy hotkey backend/service path, and align OCR provider docs with implemented providers.

Out of scope:
- Candidate 6 (`ProviderStore<P>`) is postponed. Do not introduce shared coordinator storage in this plan.
- No new OCR providers such as PaddleOCR.
- No UI redesign, hotkey behavior change, Tauri command rename, or IPC payload shape change.
- No broad formatting/refactoring outside files listed below.

## Assumptions

- `master` is the working branch unless the executor creates a feature branch first.
- Existing tests are trusted as regression guards; add focused tests only where a module interface changes.
- If a command still needs a low-level dependency after Candidate 2, that is a design smell to fix by moving behavior into an application module, not by re-exposing the dependency.
- Prompt strategy commands must not keep direct `state.config_file` access after Candidate 2. Use a small application helper around the existing `translation_prompt.rs` logic rather than exposing raw storage on `AppState`.

## Execution Order

1. Baseline verification.
2. Candidate 2: `AppState` runtime slices.
3. Candidate 3: DeepLX dual-state cleanup.
4. Candidate 5: backend capture-session render seam.
5. Candidate 4: frontend Capture Workspace shell finish.
6. Cross-platform cleanup.
7. Docs and final verification.

This order keeps foundational boundaries first. Candidate 4 and Candidate 5 can be run by separate agents after Candidate 2 lands, but they both touch capture concepts, so review their diffs carefully before merging.

## Candidate 6 Status

Candidate 6 remains deferred. Add this note to `ARCHITECTURE.md` during the docs task:

```md
ProviderStore<P> remains intentionally deferred: TranslationCoordinator and OcrCoordinator still have different activation semantics, and no current change requires reopening ADR-0004.
```

---

## File Structure

Modify for Candidate 2:
- `src-tauri/src/app_state.rs` - define runtime slice structs and shrink `AppState`.
- `src-tauri/src/composition.rs` - build runtime slices instead of flat fields.
- `src-tauri/src/composition/provider_runtime.rs` - expose hydration inputs without requiring raw fields on `AppState`.
- `src-tauri/src/application/providers/translation_prompt.rs` - add a small persistence helper for prompt strategy commands.
- `src-tauri/src/composition/capture_runtime.rs` - return a slice-shaped capture runtime if needed.
- `src-tauri/src/composition/history_runtime.rs` - update subscription path.
- `src-tauri/src/lib.rs` - update startup hotkey and composition access.
- `src-tauri/src/commands/*.rs` - update field access to runtime slices.

Modify for Candidate 3:
- `src-tauri/src/application/providers/common/provider.rs`
- `src-tauri/src/application/providers/translation/impls/deepl.rs`
- `src-tauri/src/application/providers/configuration.rs`
- `src-tauri/src/commands/provider_commands.rs`
- `src-tauri/src/application/providers/translation/coordinator.rs`
- `src-tauri/src/application/providers/translation/coordinator_test.rs`

Modify for Candidate 5:
- `src-tauri/src/application/services/capture_session_render.rs`
- `src-tauri/src/application/services/capture_session_runtime.rs`
- `src-tauri/src/application/services/mod.rs`
- `src-tauri/src/commands/capture_session_commands.rs`

Modify/create for Candidate 4:
- Create: `src/components/ScreenshotSession/captureWorkspaceDerived.ts`
- Create: `src/components/ScreenshotSession/captureWorkspaceDerived.test.ts`
- Create: `src/components/ScreenshotSession/useCaptureWorkspaceController.ts`
- Create or modify: `src/components/ScreenshotSession/captureWorkspaceEditorActions.ts`
- Modify: `src/components/ScreenshotSession/index.tsx`
- Modify: existing `captureWorkspace*.test.ts` files as behavior guards.

Modify/delete for cross-platform cleanup:
- Delete if unused after verification: `src-tauri/src/application/services/hotkey_service.rs`
- Modify: `src-tauri/src/application/services/mod.rs`
- Modify: `src-tauri/src/application/mod.rs`
- Delete: `src-tauri/src/infrastructure/system/hotkey/mod.rs`
- Delete: `src-tauri/src/infrastructure/system/hotkey/backend.rs`
- Delete: `src-tauri/src/infrastructure/system/hotkey/macos.rs`
- Delete: `src-tauri/src/infrastructure/system/hotkey/windows.rs`
- Delete: `src-tauri/src/infrastructure/system/hotkey/linux.rs`
- Modify: `src-tauri/src/infrastructure/system/mod.rs`
- Modify: `CONTEXT.md`
- Modify: `ARCHITECTURE.md`
- Modify: `docs/FEATURES.md`
- Modify: `docs/prd/settings-window-ui.md`

---

## Task 0: Baseline Verification

**Files:** none.

- [ ] **Step 1: Check worktree**

Run:

```bash
git status --short
```

Expected: clean, or only intentional plan/doc files.

- [ ] **Step 2: Run backend baseline**

Run:

```bash
cd src-tauri && cargo test
```

Expected: PASS. If failures are pre-existing, stop and record them before editing.

- [ ] **Step 3: Run frontend baseline**

Run:

```bash
npm test -- captureWorkspace
npm test -- providerStore
```

Expected: PASS.

---

## Task 1: Candidate 2 - Split `AppState` Into Runtime Slices

**Goal:** Commands should depend on runtime slices, not a flat god-object with raw infrastructure fields.

**Files:**
- Modify: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/composition.rs`
- Modify: `src-tauri/src/composition/provider_runtime.rs`
- Modify: `src-tauri/src/application/providers/translation_prompt.rs`
- Modify: `src-tauri/src/composition/capture_runtime.rs`
- Modify: `src-tauri/src/composition/history_runtime.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/commands/*.rs`

- [ ] **Step 1: Make the target structure explicit and verify the current code is red**

Before implementing the struct changes, update one simple call site, such as `src-tauri/src/lib.rs`, to use the target access path:

```rust
let hotkey_runtime = app_state.settings.hotkeys.clone();
```

The target `AppState` shape is:

```rust
pub struct AppState {
    pub settings: Arc<SettingsRuntime>,
    pub providers: Arc<ProviderRuntime>,
    pub capture: Arc<CaptureRuntimeState>,
    pub history: Arc<HistoryRuntime>,
    pub selection: Arc<SelectionRuntime>,
}
```

Expected runtime slices:

```rust
pub struct SettingsRuntime {
    pub configuration: Arc<SettingsConfiguration>,
    pub hotkeys: Arc<HotkeyRuntime>,
}

pub struct ProviderRuntime {
    pub translation: Arc<TranslationCoordinator>,
    pub ocr: Arc<OcrCoordinator>,
    pub llm_introspection: Arc<LlmIntrospection>,
    pub configuration: Arc<ProviderConfiguration>,
    pub prompt_strategies: Arc<TranslationPromptConfiguration>,
}

pub struct CaptureRuntimeState {
    pub capture: Arc<CaptureService>,
    pub sessions: Arc<CaptureSessionService>,
    pub image_composition: Arc<ImageCompositionService>,
    pub output: Arc<CaptureOutputService>,
    pub session_runtime: Arc<CaptureSessionRuntime>,
    pub pinned_images: Arc<PinnedImageService>,
    pub screenshot_state: Arc<ParkingLotMutex<ScreenshotState>>,
}

pub struct HistoryRuntime {
    pub service: Arc<HistoryService>,
    pub events: Arc<EventBus>,
}

pub struct SelectionRuntime {
    pub selected_text_acquirer: Arc<SelectedTextAcquirer>,
}
```

- [ ] **Step 2: Run compile to verify red**

Run:

```bash
cd src-tauri && cargo test app_state --no-run
```

Expected: FAIL because `app_state.settings` does not exist yet.

- [ ] **Step 3: Implement runtime slice structs in `app_state.rs`**

Move existing field types into the slice structs above. Keep `ScreenshotState` unchanged. Update `AppState::shutdown()` to call:

```rust
self.history.events.drain(std::time::Duration::from_secs(5)).await;
```

Do not leave `config_file`, `keychain`, or `http_client` on `AppState`.

- [ ] **Step 4: Add prompt strategy persistence helper**

In `src-tauri/src/application/providers/translation_prompt.rs`, add:

```rust
pub struct TranslationPromptConfiguration {
    config_file: Arc<ConfigFile>,
}

impl TranslationPromptConfiguration {
    pub fn new(config_file: Arc<ConfigFile>) -> Self;
    pub fn list(&self) -> TranslationPromptStrategyConfig;
    pub fn save(&self, config: TranslationPromptStrategyConfig) -> crate::Result<TranslationPromptStrategyConfig>;
}
```

`list()` should wrap the current `config_file.load + merge_prompt_strategy_config` behavior. `save()` should wrap validation, sanitization, and `config_file.save`.

- [ ] **Step 5: Rewire composition construction**

In `src-tauri/src/composition.rs`, keep local construction variables for `config_file`, `keychain`, and `http_client`, but only use them while wiring modules. Construct the final `AppState` with runtime slices.

For provider credential hydration, add a private composition-owned struct or function input that carries the raw dependencies before `AppState` is managed. Do not re-add raw dependencies to `AppState`.

- [ ] **Step 6: Rewire `lib.rs` startup**

Change startup hotkey access from:

```rust
let hotkey_runtime = app_state.hotkey_runtime.clone();
```

to:

```rust
let hotkey_runtime = app_state.settings.hotkeys.clone();
```

- [ ] **Step 7: Rewire commands mechanically**

Examples:

```rust
state.settings_configuration.as_ref()
```

becomes:

```rust
state.settings.configuration.as_ref()
```

```rust
state.provider_configuration
```

becomes:

```rust
state.providers.configuration
```

```rust
state.capture_session_runtime
```

becomes:

```rust
state.capture.session_runtime
```

Keep behavior unchanged. Do not rename Tauri commands.

Prompt strategy commands should call:

```rust
state.providers.prompt_strategies.list()
state.providers.prompt_strategies.save(config)
```

- [ ] **Step 8: Verify no raw infrastructure fields remain**

Run:

```bash
rg -n "pub (config_file|keychain|http_client)|state\\.(config_file|keychain|http_client)" src-tauri/src
```

Expected: no matches.

- [ ] **Step 9: Run backend tests**

Run:

```bash
cd src-tauri && cargo test
```

Expected: PASS.

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src
git commit -m "refactor(app-state): group runtime dependencies"
```

---

## Task 2: Candidate 3 - Localize DeepLX/DeepL Credential Semantics

**Goal:** DeepLX mode rules should live behind the provider-facing interface, not as repeated command/configuration/coordinator branches.

**Files:**
- Modify: `src-tauri/src/application/providers/common/provider.rs`
- Modify: `src-tauri/src/application/providers/translation/impls/deepl.rs`
- Modify: `src-tauri/src/application/providers/configuration.rs`
- Modify: `src-tauri/src/commands/provider_commands.rs`
- Modify: `src-tauri/src/application/providers/translation/coordinator.rs`
- Modify: `src-tauri/src/application/providers/translation/coordinator_test.rs`

- [ ] **Step 1: Add tests for provider-owned credential validation**

In `deepl.rs` tests, add cases:
- mode `deepl` accepts nonblank `api_key` and does not require `endpoint`.
- mode `deeplx` accepts nonblank `endpoint` and does not require `api_key`.
- blank/missing required field returns the existing user-facing error meaning.
- invalid mode returns an invalid mode error.

Run:

```bash
cd src-tauri && cargo test deepl -- --nocapture
```

Expected: FAIL until validation is exposed without mutating provider state.

- [ ] **Step 2: Add `validate_credentials` to `Provider`**

In `common/provider.rs`, add a default method:

```rust
fn validate_credentials(&self, credentials: &HashMap<String, String>) -> crate::Result<()> {
    for field in self.credential_fields() {
        if credentials
            .get(&field.name)
            .map(|value| value.trim().is_empty())
            .unwrap_or(true)
        {
            return Err(crate::AppError::Other(format!("Missing {}", field.name)));
        }
    }
    Ok(())
}
```

This default should remain equivalent to the current `validate_required_credentials` helper. Providers with optional fields, such as DeepLX mode-dependent `endpoint` / `api_key`, must override it.

- [ ] **Step 3: Override `validate_credentials` in `DeepLProvider`**

Move the mode-specific checks currently duplicated in `ProviderConfiguration` into `DeepLProvider::validate_credentials`. Keep `reconfigure_credentials` responsible for mutation only after validation passes.

- [ ] **Step 4: Remove `deeplx` special validation from `ProviderConfiguration`**

Replace:

```rust
if provider_id == "deeplx" {
    validate_deeplx_credentials_map(&cred_map)?;
} else {
    validate_required_credentials(&expected_fields, &cred_map)
}
```

with:

```rust
provider.read().validate_credentials(&cred_map)?;
```

Delete `validate_deeplx_credentials_map` once tests pass.

- [ ] **Step 5: Move legacy single-api-key command behavior behind `ProviderConfiguration`**

If `configure_translation_provider` still has:

```rust
if provider_id == "deeplx" { ... }
```

replace it with a `ProviderConfiguration` method such as:

```rust
save_legacy_api_key(provider_id, api_key)
```

That method may translate legacy DeepL API-key input to `{ mode: "deepl", api_key }`. Commands should not carry DeepLX mode knowledge.

- [ ] **Step 6: Isolate legacy `"deepl" -> "deeplx"` config mapping**

Keep the migration if needed, but name it clearly in `translation/coordinator.rs`:

```rust
fn normalize_legacy_translation_provider_id(id: &str) -> &str
```

Add or keep a test proving old config containing `"deepl"` restores `"deeplx"`.

- [ ] **Step 7: Search for remaining DeepLX branching**

Run:

```bash
rg -n 'provider_id == "deeplx"|validate_deeplx|match mode|\"deepl\" => \"deeplx\"' src-tauri/src/application src-tauri/src/commands
```

Expected:
- provider implementation may contain mode matching.
- coordinator may contain one named legacy normalization helper.
- commands should contain no `deeplx` special branch.

- [ ] **Step 8: Verify provider tests**

Run:

```bash
cd src-tauri && cargo test provider
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/application/providers src-tauri/src/commands/provider_commands.rs
git commit -m "refactor(providers): localize deeplx credential rules"
```

---

## Task 3: Candidate 5 - Make Capture Session Render a Helper Module

**Goal:** `CaptureSessionRuntime` owns dependencies; render helpers operate on explicit inputs and no longer extend `CaptureSessionService` with dependency-injected methods.

**Files:**
- Modify: `src-tauri/src/application/services/capture_session_render.rs`
- Modify: `src-tauri/src/application/services/capture_session_runtime.rs`
- Modify: `src-tauri/src/application/services/mod.rs`
- Modify: `src-tauri/src/commands/capture_session_commands.rs`

- [ ] **Step 1: Add/adjust tests for runtime-facing render methods**

In `capture_session_runtime.rs` tests, assert:
- `render_png_base64` rejects selection before snapshots are hydrated.
- `recognize_selection_text` uses runtime-owned OCR coordinator.
- `output_selection` returns `CaptureSessionOutput::Pin` without command-layer output dependency.

Run:

```bash
cd src-tauri && cargo test capture_session_runtime -- --nocapture
```

Expected: current tests pass or expose missing coverage.

- [ ] **Step 2: Convert render methods to free helper functions**

In `capture_session_render.rs`, keep `CaptureSessionOutput`, `render_capture_png`, and add free helpers if needed:

```rust
pub fn render_capture_png_base64(...)
pub async fn recognize_capture_selection_text(...)
pub async fn output_capture_selection(...)
```

Remove `impl CaptureSessionService` from this file.

- [ ] **Step 3: Update `CaptureSessionRuntime` to call free helpers**

Runtime methods should call helpers with `&self.sessions`, `&self.image_composition`, `&self.ocr`, and `&self.output`. Dependency injection should happen only inside runtime.

- [ ] **Step 4: Update command imports**

Commands should call `state.capture.session_runtime.output_selection(...)` and should not pass `capture_output_service` or `image_composition_service` into render/output operations for capture sessions.

- [ ] **Step 5: Verify no pseudo seam remains**

Run:

```bash
rg -n "impl CaptureSessionService|render_png_base64\\(|recognize_selection_text\\(|output_selection\\(" src-tauri/src/application/services/capture_session_render.rs src-tauri/src/application/services/capture_session_runtime.rs
```

Expected:
- no `impl CaptureSessionService` in `capture_session_render.rs`.
- runtime still exposes public methods.
- render module contains free helpers only.

- [ ] **Step 6: Run backend capture tests**

Run:

```bash
cd src-tauri && cargo test capture_session
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/application/services src-tauri/src/commands/capture_session_commands.rs
git commit -m "refactor(capture): make session render helpers explicit"
```

---

## Task 4: Candidate 4 - Finish ScreenshotSession Shell Extraction

**Goal:** `ScreenshotSession/index.tsx` becomes a shell that reads settings, initializes the controller hook, connects host hooks, and renders `CaptureWorkspaceView`.

**Files:**
- Create: `src/components/ScreenshotSession/captureWorkspaceDerived.ts`
- Create: `src/components/ScreenshotSession/captureWorkspaceDerived.test.ts`
- Create: `src/components/ScreenshotSession/useCaptureWorkspaceController.ts`
- Create or modify: `src/components/ScreenshotSession/captureWorkspaceEditorActions.ts`
- Modify: `src/components/ScreenshotSession/index.tsx`
- Modify: `src/components/ScreenshotSession/captureWorkspaceHost.ts`
- Modify: `src/components/ScreenshotSession/captureWorkspaceKeyboard.ts`
- Modify: `src/components/ScreenshotSession/captureWorkspacePointer.ts`
- Modify tests under `src/components/ScreenshotSession/*workspace*.test.ts`

- [ ] **Step 1: Capture current behavior with focused tests**

Run:

```bash
npm test -- captureWorkspace
npm test -- captureHostRuntime captureSelectionRuntime captureEditorRuntime capturePointerInteractionRuntime
```

Expected: PASS before extraction.

- [ ] **Step 2: Extract derived geometry into a pure module**

Move calculations for:
- capture candidates
- image readiness
- snap target rects
- selection bounds
- viewport bounds
- selection viewport rect
- cursor viewport point
- selected annotation bounds
- toolbar position

into `captureWorkspaceDerived.ts`.

Add tests in `captureWorkspaceDerived.test.ts` for representative geometry cases.

Run:

```bash
npm test -- captureWorkspaceDerived.test.ts
```

Expected: PASS.

- [ ] **Step 3: Extract editor action assembly**

Move annotation/text action handlers currently created in `index.tsx` into `captureWorkspaceEditorActions.ts` or into the controller hook if they need React refs. Keep this module close to existing `captureEditorRuntime` semantics; do not change annotation behavior.

Run:

```bash
npm test -- captureEditorRuntime captureWorkspacePointer
```

Expected: PASS.

- [ ] **Step 4: Create `useCaptureWorkspaceController`**

The hook should own:
- workspace state hook output
- mutable refs and ref-backed setters
- host action construction via `createCaptureWorkspaceHostActions`
- keyboard action construction
- pointer action construction
- derived state from `captureWorkspaceDerived`
- callbacks passed to `CaptureWorkspaceView`

The hook should return a small object:

```ts
{
  state,
  derived,
  viewHandlers,
  hostSubscriptions,
  keyboardHostEvents,
}
```

Name shape can vary, but `index.tsx` should no longer build dozens of handler callbacks directly.

- [ ] **Step 5: Reduce `index.tsx` to a composition shell**

Target shell responsibilities:
- read `screenshotSavePath`
- create `captureWindow`
- call `useCaptureWorkspaceController`
- connect `useCaptureHostSubscriptions`, `useCaptureHostWindowReveal`, and `useCaptureKeyboardHostEvents`
- render `CaptureWorkspaceView`

Target size: under 500 lines. If under 500 is not practical without unsafe churn, stop and explain the residual hotspots before continuing.

- [ ] **Step 6: Verify index no longer owns large handler clusters**

Run:

```bash
wc -l src/components/ScreenshotSession/index.tsx
rg -n "const .* = useCallback|handleCaptureWorkspace|CaptureWorkspacePointerActions|CaptureWorkspaceKeyboardActions" src/components/ScreenshotSession/index.tsx
```

Expected:
- `index.tsx` is under 500 lines.
- handler construction mostly lives in controller/workspace modules.

- [ ] **Step 7: Run frontend tests**

Run:

```bash
npm test -- captureWorkspace
npm test -- captureHostRuntime captureSelectionRuntime captureEditorRuntime capturePointerInteractionRuntime
npm run build
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src/components/ScreenshotSession
git commit -m "refactor(capture): finish screenshot session shell extraction"
```

---

## Task 5: Cross-Platform Cleanup

**Goal:** Remove unused legacy hotkey backend/service code and align OCR provider documentation with implemented providers.

**Files:**
- Delete if unused: `src-tauri/src/application/services/hotkey_service.rs`
- Modify: `src-tauri/src/application/services/mod.rs`
- Modify: `src-tauri/src/application/mod.rs`
- Delete: `src-tauri/src/infrastructure/system/hotkey/mod.rs`
- Delete: `src-tauri/src/infrastructure/system/hotkey/backend.rs`
- Delete: `src-tauri/src/infrastructure/system/hotkey/macos.rs`
- Delete: `src-tauri/src/infrastructure/system/hotkey/windows.rs`
- Delete: `src-tauri/src/infrastructure/system/hotkey/linux.rs`
- Modify: `src-tauri/src/infrastructure/system/mod.rs`
- Modify: `CONTEXT.md`
- Modify: `ARCHITECTURE.md`
- Modify: `docs/FEATURES.md`
- Modify: `docs/prd/settings-window-ui.md`

- [ ] **Step 1: Prove legacy hotkey code has no production references**

Run:

```bash
rg -n "HotkeyService|HotkeyBackend|HotkeyId|get_hotkey_backend|system::hotkey" src-tauri/src
```

Expected before cleanup: matches only legacy service/module exports and tests. If production command/runtime code depends on it, stop and reassess.

- [ ] **Step 2: Delete legacy hotkey service/module path**

Remove:
- `application/services/hotkey_service.rs`
- `infrastructure/system/hotkey/*`

Update exports in:
- `application/services/mod.rs`
- `application/mod.rs`
- `infrastructure/system/mod.rs`

Keep the current `application/hotkeys/*` and `tauri_plugin_global_shortcut` runtime untouched.

- [ ] **Step 3: Verify hotkey cleanup**

Run:

```bash
rg -n "HotkeyService|HotkeyBackend|HotkeyId|get_hotkey_backend|system::hotkey" src-tauri/src
cd src-tauri && cargo test hotkey
```

Expected:
- no stale references.
- hotkey configuration/runtime tests still pass.

- [ ] **Step 4: Align OCR provider docs with implementation**

Current implemented OCR providers are:
- Tesseract
- macOS System OCR (`SystemOcrProvider`, macOS only)
- Baidu OCR

Update `CONTEXT.md`, `docs/FEATURES.md`, and `docs/prd/settings-window-ui.md` so PaddleOCR/Tencent/Google/Azure are not described as currently implemented provider cards. If they are mentioned, mark them as future candidates, not present behavior.

- [ ] **Step 5: Verify OCR docs match code**

Run:

```bash
rg -n "PaddleOCR|腾讯云 OCR|Google Cloud Vision|Azure Computer Vision" CONTEXT.md docs
rg -n "SystemOcrProvider|TesseractProvider|BaiduOcrProvider" src-tauri/src/application/providers/ocr
```

Expected:
- future-only provider names are clearly marked as future/non-implemented, or removed from current feature docs.
- implemented provider names remain discoverable.

- [ ] **Step 6: Run backend tests**

Run:

```bash
cd src-tauri && cargo test
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src CONTEXT.md ARCHITECTURE.md docs/FEATURES.md docs/prd/settings-window-ui.md
git commit -m "chore(platform): remove legacy hotkey path and align ocr docs"
```

---

## Task 6: Docs and Final Verification

**Files:**
- Modify: `CONTEXT.md`
- Modify: `ARCHITECTURE.md`

- [ ] **Step 1: Update architecture docs**

Document:
- `AppState` now exposes runtime slices instead of raw infrastructure fields.
- DeepLX credential mode rules live in `DeepLProvider`.
- Capture session rendering is a helper module used by `CaptureSessionRuntime`.
- `ScreenshotSession/index.tsx` is a frontend composition shell.
- Candidate 6 remains deferred.

- [ ] **Step 2: Run full verification**

Run:

```bash
cd src-tauri && cargo test
cd .. && npm test
npm run build
```

Expected: PASS.

- [ ] **Step 3: Run structural checks**

Run:

```bash
rg -n "state\\.(config_file|keychain|http_client)" src-tauri/src
rg -n 'provider_id == "deeplx"|validate_deeplx' src-tauri/src/commands src-tauri/src/application/providers/configuration.rs
rg -n "impl CaptureSessionService" src-tauri/src/application/services/capture_session_render.rs
wc -l src/components/ScreenshotSession/index.tsx
rg -n "HotkeyService|HotkeyBackend|HotkeyId|get_hotkey_backend|system::hotkey" src-tauri/src
```

Expected:
- no raw AppState infrastructure access.
- no DeepLX special branching in commands/configuration.
- no render impl on `CaptureSessionService`.
- `ScreenshotSession/index.tsx` under 500 lines or documented residual.
- no legacy hotkey path references.

- [ ] **Step 4: Review diff**

Run:

```bash
git diff --stat
git diff
```

Expected: every changed line traces to Candidate 2, 3, 4, 5, cross-platform cleanup, or docs.

- [ ] **Step 5: Commit docs/final polish**

```bash
git add CONTEXT.md ARCHITECTURE.md
git commit -m "docs: record architecture followup refactors"
```

---

## Residual Risks

- Candidate 2 touches many call sites. Keep it mechanical and avoid opportunistic command refactors.
- Candidate 3 should not break legacy saved config with `"deepl"` IDs; preserve migration tests.
- Candidate 4 can become another large hook if everything is moved blindly. If `useCaptureWorkspaceController.ts` grows too large, split host/editor/pointer assembly before finishing.
- Candidate 5 must preserve snapshot readiness checks in `CaptureSessionRuntime`.
- Cross-platform cleanup should not delete the current `application/hotkeys` runtime or `tauri_plugin_global_shortcut` usage.
