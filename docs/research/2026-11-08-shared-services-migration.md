# Shared Rust Services migration

Inspected 2026-11-08 against the current checkout. This is a source audit and an implementation handoff, not a report of completed native Services acceptance. No production UI, model or host is changed by this document.

## Current state and scope

Services is still rendered by React. `SettingsSectionContent` in [index.tsx](../../src/views/SettingsWindow/index.tsx) delegates General, Screenshot, Translation and OCR preferences to `NativeSettingsPage`, but renders Services through its own context navigation, sticky header and scroll area. [navigationModel.tsx](../../src/views/SettingsWindow/navigationModel.tsx) defines exactly three Services routes: `ocr` / OCR 服务, `translation` / 翻译服务 and `tts` / 语音合成. The last selected route is persisted by [settingsStore.ts](../../src/stores/settingsStore.ts); its initial route is OCR. The top-level Translation/OCR preference pages are distinct from these provider-administration pages.

There is already a portable [ServicesController](../../crates/snaplingo-core/src/application/settings/services.rs) for OCR and TTS. It is not mounted in a shared Rust Services view: no Services drawing module exists in `snaplingo-ui` and no Services page is mounted by the macOS shared host. Translation administration, model introspection and prompt configuration already exist behind [ProviderAdministration](../../crates/snaplingo-core/src/application/providers/administration.rs), but their form/workspace lifecycle remains in the frontend.

## Required behavior, with existing workflow ownership

| Surface | Real behavior to preserve | Existing Rust interface |
| --- | --- | --- |
| Service navigation | OCR/translation/TTS selection, retained last route, Native/Studio/Paper context/tab layout, scrolling, configured UI scale and monitor scale | New shared Services presentation; current preference shell/viewport adapter |
| OCR list | All registered providers, configured/active state, local/remote badge, provider subtitle, Paddle device/fallback status | `ProviderAdministration::list_ocr_providers`; `ServicesController::projection` |
| OCR activation | Exactly one active provider; configure a credential-free provider by activating it; reload when OCR GPU preference changes | `ServicesController::activate` / `configure`; `OcrCoordinator` |
| OCR credentials | Async schema, ordered dynamic fields with labels and secret flags, empty initial draft, required values, save then activate, retain retryable form on failure, cancel/back | `ServicesController::configure`, `save`, `cancel`; `OcrProviderConfiguration` |
| Translation list | Active providers first in their execution order, inactive providers in their durable display order; show built-in/custom and local/remote state | `ProviderAdministration::list_translation_providers` |
| Translation activation | Multiple providers may be active; configured rows can activate/deactivate; unconfigured rows configure first | `activate_translation_provider`, `deactivate_translation_provider` |
| Translation ordering | Drag handle, captured pointer, highlighted valid target, reorder only within active or inactive group; cancel/lost capture must not write | `reorder_active_translation_providers`, `reorder_inactive_translation_providers` |
| Built-in translation credentials | Generic schema except DeepLX/DeepL mode switch; save then activate; provider help links | `translation_credential_schema`, `configure_translation_provider_credentials`, `activate_translation_provider` |
| Custom provider lifecycle | Add, edit full definition, keep saved API key when edit field is blank, auto-activate after successful edit, test saved provider, confirm removal | `add_custom_translation_provider`, `update_custom_translation_provider`, `test_custom_translation_provider`, `remove_custom_translation_provider` |
| Custom provider introspection | List models from current draft endpoint/key, select returned model or type one, test current draft connection; progress, empty result and error states | `list_openai_compatible_models`, `list_anthropic_models`, `list_gemini_models`; protocol-specific `test_*` |
| Prompt workspace | Select smart or a saved strategy, edit name/description/multiline system prompt, save existing, create custom from draft, delete only deletable strategy, select general after deletion | `list_translation_prompt_strategies`, `save_translation_prompt_strategies`; `TranslationPromptConfiguration` |
| TTS | Dynamic installed voice list plus system default, persist voice ID and rate, slider 80–400 step 10, test speech text/status | `ServicesController::load_voices`, `edit_voice`, `edit_rate`, `speak`; `TtsRuntime` |

The shared [ProviderCard](../../src/views/SettingsWindow/Services/ProviderCard.tsx) is part of the reference: minimum 72 DIP row height; icon/initial; local/remote and unconfigured badges; active highlight; hover/focus actions; endpoint or provider-specific subtitle. A custom provider whose historical name starts `custom-llm-` is displayed using its model name, as implemented by [configuration.ts](../../src/application/settings/configuration.ts). Preserve that projection rule instead of silently renaming durable provider definitions.

OCR has no deactivate interface. The current shared React card can display a “停用” button for an active OCR provider, but `OcrProvidersPage` supplies no deactivate callback. That is an existing inert control, not an omitted backend capability. Do not invent OCR deactivation as part of migration; keep the one-active-provider workflow and document any correction to that label separately.

## Credential and form invariants

`CredentialField` in [common/provider.rs](../../crates/snaplingo-core/src/application/providers/common/provider.rs) contains `name`, `label`, `secret`; it is dynamic data, not a fixed list of settings enum variants. Provider schema and backend validation remain authoritative.

- Baidu translation requires `app_id` (visible) and `secret_key` (secret).
- Baidu OCR requires `api_key` (schema says visible) and `secret_key` (secret). Use the supplied flag, not a field-name guess.
- DeepLX's schema contains `mode`, `endpoint`, `api_key`, but **all three are not simultaneously required**. [deepl.rs](../../crates/snaplingo-core/src/application/providers/translation/impls/deepl.rs) accepts `{mode: "deeplx", endpoint}` or `{mode: "deepl", api_key}`. The current form defaults to DeepLX and has a Standard DeepL switch. Hidden unused fields must not be sent as blank credentials: `ProviderConfiguration::save_credentials` rejects provided blank values.
- Built-in credential forms start empty; no current UI interface loads stored keys. Custom-provider edits prefill metadata and keep API key empty, with “留空则保持现有 API Key”. Convert blank edit keys to `None`; adding requires a nonblank key.
- Saving credentials reconfigures the registered runtime immediately. Configuration modules own persistence, provider locks, snapshot/rollback and change notification; the UI must not write SQLite or rebuild coordinators.
- Existing OCR `ServicesController::save` already combines configure and activate, keeps the form after failure, excludes duplicate saves, and rejects stale schema completions after cancel/disposal. Reuse that behavior.

The shared editor currently supports single-line text and deliberately strips CR/LF in [text_input.rs](../../crates/snaplingo-ui/src/text_input.rs). Services needs two real additional uses: masked credential display and a multiline prompt. Add those deliberately before switching the entire Services page; treating a prompt as the current single-line field would destroy its line breaks. Secret drafts must render masked text and protected accessibility values; do not place their committed text in `Scene::Text` or geometry diagnostics. Caret/selection indices still address the real committed UTF-8 text, so masked display needs an explicit index mapping.

## Custom provider and prompt details

[CustomTranslationProviderDialog](../../src/views/SettingsWindow/Services/CustomTranslationProviderDialog.tsx) and its [form model](../../src/views/SettingsWindow/Services/customTranslationProviderFormModel.ts) define:

- Name, protocol, endpoint, API key, model, optional reasoning level and prompt strategy.
- Three protocol families: OpenAI, Anthropic, Gemini. OpenAI has Chat Completions and Responses modes; the stored protocol values are `openai`, `openai-responses`, `anthropic`, `gemini`.
- Family/mode changes reset endpoint/model to their current protocol defaults and clear model/test results. Endpoint or key edits invalidate both model results and test status; model edits invalidate test status.
- Reasoning choices are default, minimal, low, medium, high and xhigh.
- Add requires trimmed name/endpoint/model/key. Update requires name/endpoint/model and permits a blank key. Save stays open while pending and on backend failure; successful save closes.
- Models can be loaded only with draft endpoint and key. Connection testing additionally requires model. Editing a provider with an unchanged blank key can save, but the draft introspection buttons remain disabled until a key is supplied; testing the saved row uses stored credentials through `test_custom_translation_provider`.
- Endpoint preview follows [customTranslationProviderForm.ts](../../src/views/SettingsWindow/Services/customTranslationProviderForm.ts), including `/v1/chat/completions`, `/v1/responses`, `/v1/messages` and Gemini generate-content completion. This is display behavior; HTTP endpoint construction remains owned by the existing LLM runtime.

[useTranslationPromptStrategyWorkspace](../../src/views/SettingsWindow/Services/useTranslationPromptStrategyWorkspace.ts) is embedded in the custom form, not a fourth Services route. Smart selection clears the draft; a named selection populates it. Name and prompt are required, description is optional; inputs are trimmed on save. Built-in strategies can be edited but not deleted. There is no fallback selector: every provider request fixes fallback to `general`, and runtime smart selection also falls back to general. Backend defaults include general, technical, academic, casual and mixed-zh-en; prefer `TranslationPromptConfiguration::list` rather than copying the frontend's one-item emergency fallback into a second source of truth.

Prompt persistence saves a complete `TranslationPromptStrategyConfig`; backend validation/sanitization restores built-in identity/deletability and prevents removing general. The shared workspace should send the complete current draft configuration and project the returned saved configuration, not patch the store directly.

## Small shared interface and actual seams

Keep Services as its own deep presentation module, e.g. `snaplingo_ui::services`, with private OCR/TTS/list/form/prompt implementations. Avoid adding every dynamic provider field to the existing fixed `SettingsUi::Field` enum. The current `Target` is `Copy` and contains fixed settings fields; provider IDs and credential names are runtime strings. A private Services hit-target table may use compact handles, but activation must resolve a handle against the current view/form revision into the stable provider ID/credential name before emitting an intent. Stale targets from a removed provider, switched route or dismissed form must be rejected.

The external presentation interface can stay at the same depth as Settings:

```rust
ServicesUi::new(ServicesSnapshot) -> ServicesUi
ServicesUi::update(ServicesInput) -> Vec<ServicesIntent>
ServicesUi::frame(Size, interface_scale, &mut impl TextMeasurer) -> Scene
```

`ServicesInput` includes fresh snapshots plus normalized pointer/cancel, keyboard, focus, clipboard and composition input. `ServicesSnapshot` contains theme/appearance, selected route, provider/voice projections, current operation state and a versioned form/model-test/prompt projection. `ServicesUi` owns drafts, focus, selection, scroll, dropdown and drag state; it never receives an AppKit/Win32/Tauri type. `ServicesIntent` expresses configure/save/activate/reorder/remove/test/model-list/prompt-save/voice-edit/speak and route changes, using stable IDs and current form revision. Network completions are projected by the workflow owner and do not directly mutate a drawing view from a worker.

Keep the existing OCR/TTS controller. Add a focused translation workspace controller beside `settings/services.rs` rather than extending its current `ServicesKind`/OCR-only provider trait into a grab bag of unrelated implementations. It consumes the real `ProviderAdministration` interface (a narrow test adapter may implement the same workflow-facing interface). Its small caller interface is snapshot plus typed request dispatch and disposal; private implementation owns:

1. Configure schema selection; submit credentials then activate; keep retryable failures.
2. Custom add/update/remove/test, form revision and duplicate-submit exclusion.
3. Model-list and draft connection-test requests tagged with form revision and draft fingerprint; late completion cannot restore results for a different protocol/key/endpoint/model or disposed form.
4. Prompt draft selection and complete-config persistence, with generated custom IDs supplied by the existing ID mechanism or a single injected ID source.

This seam hides the currently scattered React workflow state without duplicating coordinator/storage policy. The desktop adapter supplies worker execution, main-thread delivery, clipboard, external URLs, destructive-action confirmation and OS speech. `SettingsChanged` and `ProvidersChanged` refresh fresh projections. OCR refresh must respond to GPU changes; voice requests and model/schema requests require the current cancellation/revision rules. Host redraw should use the existing scene/scale reuse behavior rather than rasterizing unchanged Services scenes on each notification.

TTS already has both target adapters: macOS `/usr/bin/say` and Windows `System.Speech` through `powershell.exe`, inspected in [system/tts](../../crates/snaplingo-app/src/infrastructure/system/tts). They cache installed voices and replace/reap a previous speech process. A successful `speak` means speech was started, not finished; the UI's “已开始朗读” must retain that meaning. The voice ID is opaque; do not validate it against a stale dropdown list or overwrite unrelated general preferences when saving voice/rate.

## Concrete parallel implementation slices

These slices have bounded file ownership and can proceed while General remains stable. A coordinator first agrees the Services DTO/intent names in one new `services/mod.rs`; workers implement only their listed files, and the coordinator handles the one-time export/router/host wiring.

| Slice | Owned work | Acceptance at its interface |
| --- | --- | --- |
| A — OCR and TTS shared drawing | `snaplingo-ui/src/services/{ocr,tts,credentials}.rs`; project existing `ServicesController` into shared DTOs | Actual provider list and acceleration subtitle; local edit activates; remote schema/save/activate retry; secret masking; installed voice/default/rate/preview. Durable reopen preserves OCR/TTS edits and unrelated General fields. |
| B — Translation workflow | New `snaplingo-core/src/application/settings/translation_services.rs` and focused tests | Real administration calls; save then activate; add/edit blank-key behavior; confirm-remove intent; exact active/inactive reorder sets; newest schema/model/test wins; failure keeps form; disposal ignores late work. No UI/host edits. |
| C — Translation list and custom form | `snaplingo-ui/src/services/{translation,custom_provider}.rs` | Active-first rows, same-group pointer drag with cancel/lost-capture tests, built-in/DeepLX form, all four protocols, reasoning, metadata edit and model/test states; generated intents use provider IDs and the current revision. |
| D — Prompt editor and references | `snaplingo-ui/src/services/prompt.rs`, the needed multiline-editor extension, isolated React Services reference fixture/harness | Newlines/IME/selection/clipboard survive; smart/named strategies, editable built-ins, protected deletion, save/add/remove errors. Reference scenes cover list + credential + custom/prompt states in three themes, appearances, widths and configured scales. |

A and B can begin immediately after the DTO agreement. C can begin with recorded snapshots while B is implemented. D's multiline editor and reference collector can run independently of both; coordinate only the shared editor/Services export edits. Do not switch the production Services navigation merely because A passes: the translation/custom/prompt routes would otherwise regress to an incomplete page.

After those slices integrate, the coordinator connects macOS shared window ownership and the winit host to the same ServicesUi and workflows, preserves service-route persistence, publishes the same AccessKit semantics, and removes the Services React dependency only after full acceptance. The macOS host must demonstrate inline form navigation, main/secondary route switches, resize/zoom/backing changes, schema/model request cancellation, credentials persistence and real TTS. Windows compilation is necessary but does not prove its voice/IME/drag/window behavior; run the same real workflow suite there before claiming both platforms complete.

## Evidence available and missing

Existing tests are useful specifications, not fresh run results from this audit:

- `ServicesController` tests cover retryable schema/configuration/activation failure, busy-save exclusion, cancel/disposal with late schema, durable TTS edits, latest voice request and late speech after disposal.
- `TranslationProvidersPage.pointer.test.tsx` covers captured-pointer reorder, cancel/lost capture, outside/invalid drop and cross-group rejection.
- Custom-provider tests cover pending/error form retention, full edits, protocol modes and the fixed fallback; prompt-workspace tests cover selection, load fallback, trimmed save, custom add/deletion and validation errors.
- Provider configuration tests cover credential validation, runtime reconfiguration rollback, custom lifecycle races and inactive-order persistence.

No shared Services raster/geometry fixture, native Services mount, Windows Services runtime result or complete text-editor accessibility acceptance exists yet in the inspected sources. Those are required deliverables, not reasons to replace any of the real capabilities above with placeholders.

## Services reference acceptance — 2026-11-09

The isolated reference now mounts the actual retained `SettingsWindow` and its Services components. `settings_services_fixture.ts` supplies provider/store/runtime data without initializing Tauri, reading installed settings, or making network calls. `settings_reference_snapshot services [dark] [compact]` captures 108 real WebKit scenes: Native/Studio/Paper, light/dark, 900×650/700×650, and nine list/credential/DeepLX/DeepL/custom/prompt-scroll states. The paired `snaplingo-render` example `services_snapshot` renders the same provider, voice, credential-schema and multiline-prompt DTOs with the CPU renderer.

`node script/check-services-layout.mjs` passes all 108 scenes and 1,788 content/card/control rectangles, with a maximum difference of 0.5 logical points. The comparison includes actual provider cards, visible horizontal or directory navigation, all representative form fields/buttons, and the custom prompt after a 650-point scroll. References are read from `target/native-settings/reference-services-*.json`; shared render diagnostics and PNGs are in `target/shared-services`. The checker compares recorded DOM rectangles with independently generated shared frames; it does not derive reference values from Rust drawing.

The drawing changes correct the list's negative horizontal margin, theme-specific card heights and dividers, reserved action widths, actual badges/icons, header and directory navigation, TTS row/grid geometry, credential panel padding, OCR's retained list above its inline form, custom protocol/model/reasoning/prompt layout, sticky-header scrolling, and custom input/prompt line height. A long voice popup now stays within a short viewport and can scroll to and select the final installed voice.

The compact real DOM fixture is retained at `snaplingo-ui/tests/fixtures/react-services-layout.json` (108 scenes, about 156 KB). `record-services-layout.mjs` reads only WebKit DOM reports; `snaplingo-render/tests/services_react_layout.rs` loads the retained fixture and independently uses `RendererTextMetrics` with the same DTO fixture as the raster example. This permanent regression checks all 1,788 rectangles and each control's enabled flag without reading `target/`. It also preserves the prompt-save button's actual foreground/background colors: the retained React button is enabled and its real click successfully invokes the save port, while its computed style remains raised/faint; no dirty-state disable rule exists in that component.

Input review fixed whole-row hover/drop hit testing (including the top and bottom outside the drag handle), first-click caret placement in existing and masked UTF-8 fields, focus/ScrollIntoView on an offscreen voice scrolling its popup rather than the main page, and the DeepL mode switch's accessibility role and checked state. The existing workflow's revision/request-token tests continue to reject late schema/model/test completion, duplicate saves and reordered drafts.

Services route selection now uses the existing SQLite settings table's independent `services_navigation` namespace. The app settings runtime retains the existing `SqliteConfigStore`; a new Services session restores a valid saved route, and explicit route changes save synchronously. Missing/unknown values use the caller's default; explicit navigation still wins. A real database reopen test proves route persistence without changing the general settings document.

Verification: 14 shared Services behavior tests, the permanent renderer geometry/style test, nine translation workflow tests, five app workflow-adapter tests, six window accessibility tests and three config-store tests pass. The paired renderer/reference examples compile; owned Rust files pass `rustfmt --check`, and frontend type checking passes. Representative list/custom-prompt/TTS raster pairs were inspected. These results do not establish Windows runtime/IME/voice behavior, complete native text-editor accessibility acceptance, or pixel-identical font rasterization. Production route/native mounts are owned by the host integration slice.
