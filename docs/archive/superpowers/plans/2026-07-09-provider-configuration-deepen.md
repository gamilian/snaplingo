# Provider Configuration Deepening Implementation Plan

> **For agentic workers:** Execute task-by-task with TDD. Steps use checkbox (`- [ ]`) syntax for tracking. Each task ends with `cargo test`/`cargo build` green before the next starts.

**Goal:** Deepen the custom LLM provider lifecycle out of the Commands seam. `provider_commands.rs` (909 lines) currently hand-orchestrates HTTP headers, three-protocol JSON parsing, LLMClient construction, keychain writes, and config persistence. Move that into two deep modules behind narrow interfaces so commands become one-line delegations — realizing the ADR-0005 boundary ("commands stay thin: validate and save credentials, ask the Coordinator to reconfigure") that the current shape structurally breaks.

**Architecture (decisions from grilling):**
- **Two modules.** `application/providers/configuration.rs` becomes a `ProviderConfiguration` struct owning the full custom-provider lifecycle (add/update/remove + credentials + list + test_custom). New `application/providers/llm_introspection.rs` is an `LlmIntrospection` struct owning raw-credential probing (`list_models`/`test`) for pre-save UI.
- **Sibling trait.** New `LlmModelLister` trait in `infrastructure/llm/client.rs`; `OpenAILLMClient`/`AnthropicLLMClient`/`GeminiLLMClient` implement both `LLMClient` + `LlmModelLister`. `LLMTranslationProvider` depends only on `LLMClient` (interface segregation). `test` reuses existing `LLMClient::generate` — not a new capability.
- **Structs in AppState.** `ProviderConfiguration` + `LlmIntrospection` registered in `AppState`; commands call `state.provider_configuration.*` / `state.llm_introspection.*`.
- **Keychain → trait object.** `Keychain.backend` becomes `Box<dyn KeychainBackend>` + `with_backend()` constructor so `ProviderConfiguration::update/remove` are unit-testable with a stub. Matches the `Arc<dyn Backend>` pattern every other platform adapter already uses.
- **IPC contract frozen.** All `#[tauri::command]` names, arg names, and serde DTOs stay byte-identical. Frontend (`src/tauri/providers.ts`, `providerStore`, `customProviderIntrospection.ts`) is untouched.

**Tech Stack:** Rust, Tauri, async-trait, Vitest (frontend, unchanged), existing `MockHttpClient` test pattern.

---

## Scope

In scope:
- `Keychain` backend → `Box<dyn KeychainBackend>` + injectable constructor.
- New `LlmModelLister` trait + `list_models()` on the three LLM clients (absorbs `parse_*_models_response`, `openai_authorization_headers`, `anthropic_headers`, `ensure_openai_compatible_success_status` from `provider_commands.rs`).
- New `LlmIntrospection` struct (application layer): `list_models(protocol, endpoint, api_key)`, `test(protocol, endpoint, model, api_key)`.
- `ProviderConfiguration` struct: absorb `add` + new `update`/`remove`/`save_credentials`/`list_provider_infos`/`test_custom_provider`/`credential_schema`. Impose `add`'s rollback discipline on `update`/`remove`.
- Rewire 15 `provider_commands.rs` commands to one-line delegations; delete the moved helpers.
- `AppState` + `composition.rs` wire the two new structs.
- Tests: Keychain stub, `LlmIntrospection` dispatch, `ProviderConfiguration` update/remove rollback, `list_models` parse (moved).

Out of scope:
- `list/save_translation_prompt_strategies` (separate prompt-strategy concern; leave in command).
- `activate`/`deactivate`/`reorder_active_translation_providers` (already thin, delegate to coordinator; leave).
- The `deeplx`/`deepl` duality cleanup (Candidate 3) — moved-as-is into `ProviderConfiguration`, not restructured.
- Frontend changes (contract frozen).
- `list_translation_providers` display-ordering helper stays pure (moves to `configuration.rs` or a sibling, behavior unchanged).

## Preconditions

Feature branch off `master`:

```bash
git switch -c refactor/provider-configuration-deepen
git status --short   # expect: only the new plan file (untracked) or clean
```

Baseline (must be green before starting):

```bash
cd src-tauri && cargo test --no-run   # compiles
cd src-tauri && cargo test            # baseline green
cd src-tauri && cargo clippy --all-targets -- -D warnings   # baseline clean (note any pre-existing warnings)
```

If baseline has pre-existing warnings, record them so they are not introduced by this work.

## Success Criteria

- `provider_commands.rs` shrinks from 909 → ~250 lines (DTOs + one-line command delegations + pure display helper).
- No `state.config_file` / `state.keychain` / `state.http_client` direct access in `provider_commands.rs` for the 15 migrated commands — all go through `state.provider_configuration` / `state.llm_introspection`.
- `update`/`remove` have rollback parity with `add` (keychain/config restored on failure).
- `Keychain` constructs via `Box<dyn KeychainBackend>`; a `StubKeychain` exists in tests.
- New unit tests: `LlmIntrospection` protocol dispatch, `list_models` parse (moved from commands), `ProviderConfiguration::update`/`remove` rollback with `StubKeychain`.
- IPC contract unchanged: `npm test` (frontend adapter tests incl. `providers.test.ts`) green without frontend edits.
- Full verify: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `npm test`, `npm run build`.

## File Structure

Create:
- `src-tauri/src/application/providers/llm_introspection.rs` — `LlmIntrospection` struct + tests.
- `src-tauri/src/application/providers/llm_introspection_test.rs` — dispatch + parse tests.

Modify:
- `src-tauri/src/infrastructure/storage/keychain/mod.rs` — backend → `Box<dyn KeychainBackend>`, add `with_backend`.
- `src-tauri/src/infrastructure/storage/keychain/backend.rs` — confirm trait object-safe (no generics); only if needed.
- `src-tauri/src/infrastructure/llm/client.rs` — add `LlmModelLister` trait + `ModelInfo`.
- `src-tauri/src/infrastructure/llm/{openai,anthropic,gemini}.rs` — impl `list_models()` (absorb parse + headers + url usage).
- `src-tauri/src/application/providers/configuration.rs` — `ProviderConfiguration` struct + methods (absorb existing free fns).
- `src-tauri/src/application/providers/configuration_test.rs` — update/remove rollback tests with `StubKeychain` (new file, or extend existing inline `#[cfg(test)]`).
- `src-tauri/src/application/providers/mod.rs` — export new modules.
- `src-tauri/src/commands/provider_commands.rs` — rewrite 15 commands as delegations; delete moved helpers.
- `src-tauri/src/app_state.rs` — add `provider_configuration`, `llm_introspection` fields.
- `src-tauri/src/composition.rs` (+ `composition/provider_runtime.rs`) — construct + inject the two structs.
- `CONTEXT.md` / `ARCHITECTURE.md` — document the two new modules (final task).

Do not modify:
- `src/tauri/providers.ts` and all frontend (contract frozen).
- `src-tauri/src/commands/ocr_commands.rs` (OCR is a separate slice).
- Coordinator files (`ocr/coordinator.rs`, `translation/coordinator.rs`) — behavior unchanged.

---

## Task 1: Keychain → trait object

**Files:** `infrastructure/storage/keychain/mod.rs`, `keychain/backend.rs`, `composition.rs`.

- [ ] **Step 1: Read `keychain/backend.rs`**, confirm `KeychainBackend` is object-safe (no generics, no `Self` returns). If it returns `Result<String>` etc. it is fine.

- [ ] **Step 2: Change `Keychain` to hold `Box<dyn KeychainBackend>`**

  ```rust
  pub struct Keychain {
      backend: Box<dyn KeychainBackend>,
  }
  impl Keychain {
      pub fn new() -> Self { Self { backend: Box::new(PlatformKeychainImpl::new()) } }
      #[cfg(test)]
      pub fn with_backend(backend: impl KeychainBackend + 'static) -> Self {
          Self { backend: Box::new(backend) }
      }
      // all save/load/delete methods unchanged: self.backend.save(...)
  }
  ```

  Keep `Default`. Public API (save/load/delete provider credential*) is unchanged.

- [ ] **Step 3: Verify build + tests**

  ```bash
  cd src-tauri && cargo build && cargo test
  ```

  Expected: green. `composition.rs` `Keychain::new()` still works.

- [ ] **Step 4: Add a trivial `StubKeychain` test** proving `with_backend` works (save→load round-trip in-memory). This is the foundation for Task 4 tests.

---

## Task 2: `LlmModelLister` trait + `list_models()` on three clients

**Files:** `infrastructure/llm/client.rs`, `openai.rs`, `anthropic.rs`, `gemini.rs`. Move `parse_*_models_response` / `models_from_array` / `ensure_openai_compatible_success_status` / `openai_authorization_headers` / `anthropic_headers` out of `provider_commands.rs` INTO the clients.

- [ ] **Step 1: Write failing parse tests in the client test modules** — move the 4 existing parse tests (`parses_openai_compatible_model_list_response`, `rejects_..._without_data_array`, `parses_anthropic_...`, `parses_gemini_...`) from `provider_commands.rs` tests into `infrastructure/llm/{openai,anthropic,gemini}.rs` `#[cfg(test)]`. Verify RED (new location, fns not yet moved).

- [ ] **Step 2: Add `LlmModelLister` trait + `ModelInfo` in `client.rs`**

  ```rust
  #[async_trait]
  pub trait LlmModelLister: Send + Sync {
      async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>>;
  }
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct ModelInfo { pub id: String }
  ```

- [ ] **Step 3: Impl `list_models()` on each client** — each owns its URL (from `endpoint_url.rs`), its headers, the `http_client.get`, status check, and parse. Anthropic/Gemini get their own header/parse fns (moved from commands). OpenAI handles both chat-completions + responses endpoints share the same `/models` list (no mode split needed for listing).

- [ ] **Step 4: Verify**

  ```bash
  cd src-tauri && cargo test --manifest-path src-tauri/Cargo.toml llm
  cd src-tauri && cargo build
  ```

  Expected: moved parse tests green. `provider_commands.rs` still compiles because the 3 `list_*_models` commands still call the old (now-moved) helpers — **temporarily broken**, so simultaneously rewire them in Step 5.

- [ ] **Step 5: Rewire 3 `list_*_models` commands** to construct the client and call `list_models()` (still in `provider_commands.rs`, not yet behind `LlmIntrospection` — that's Task 3). Delete the now-dead parse/header helpers from `provider_commands.rs`. `cargo build` green.

- [ ] **Step 6: Verify full + clippy**

  ```bash
  cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings
  ```

---

## Task 3: `LlmIntrospection` struct + rewire 7 introspection commands

**Files:** new `application/providers/llm_introspection.rs` + test; `app_state.rs`; `composition.rs`; `commands/provider_commands.rs`.

- [ ] **Step 1: Write failing dispatch tests** for `LlmIntrospection`:
  - `list_models` dispatches to the right client by `LLMProtocol` (OpenAI/OpenAIResponses→OpenAI client, Anthropic, Gemini). Use a `MockLlmModelLister` or `MockHttpClient` returning canned model JSON.
  - `test` sends an "OK" generate request and returns Ok on 200 / maps 401/403/429 to error strings (move `ensure_openai_compatible_success_status` semantics if applicable — though test uses `generate`, the error mapping stays in the client's `generate`).
  Verify RED.

- [ ] **Step 2: Implement `LlmIntrospection`**

  ```rust
  pub struct LlmIntrospection { http_client: Arc<dyn HttpClient> }
  impl LlmIntrospection {
      pub fn new(http_client: Arc<dyn HttpClient>) -> Self { ... }
      pub async fn list_models(&self, protocol: LLMProtocol, endpoint: &str, api_key: &str) -> Result<Vec<ModelInfo>>;
      pub async fn test(&self, protocol: LLMProtocol, endpoint: &str, model: &str, api_key: &str) -> Result<()>;
      // constructs the right client by protocol (centralize the match currently duplicated in configuration.rs::create_llm_translation_provider + provider_commands::test_custom_translation_provider_def)
  }
  ```

  Centralize client construction in a `build_llm_client(protocol, http, endpoint, model, key) -> Arc<dyn LLMClient>` (+ `Arc<dyn LlmModelLister>` for listing) shared with `configuration.rs`.

- [ ] **Step 3: Add to `AppState` + `composition.rs`** — `pub llm_introspection: Arc<LlmIntrospection>`. Construct in `build_app_state`.

- [ ] **Step 4: Rewire 7 commands** — `list_openai_compatible_models`/`list_anthropic_models`/`list_gemini_models` → `state.llm_introspection.list_models(protocol, ...)`. `test_openai_compatible_provider`/`test_openai_responses_provider`/`test_anthropic_provider`/`test_gemini_provider` → `state.llm_introspection.test(protocol, ...)`. Each command maps its request DTO → the protocol + fields, returns `Result<_, String>` via `.map_err(|e| e.to_string())`. Delete `test_llm_client` + the per-protocol test bodies from `provider_commands.rs`.

- [ ] **Step 5: Verify**

  ```bash
  cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings
  ```

  Expected: dispatch tests green, `provider_commands.rs` shrunk.

---

## Task 4: `ProviderConfiguration` struct + rewire 8 lifecycle/credential commands

**Files:** `application/providers/configuration.rs` + test; `app_state.rs`; `composition.rs`; `commands/provider_commands.rs`.

- [ ] **Step 1: Write failing rollback tests** with `StubKeychain` + `ConfigFile::new_temp()` + `MockHttpClient`:
  - `update` with a failing `coordinator.replace` restores the prior custom def in config and the prior keychain value.
  - `update` without a new api_key loads the existing key from keychain.
  - `remove` unregisters, drops the def from config, deletes the keychain entry; builtin id rejection (`google-translate`/`deeplx`/`baidu-translate`) returns the "Cannot remove builtin" error.
  - `save_credentials` validates required fields, saves keychain (both `save_provider_credentials` + `save_provider_credential` to preserve current behavior), then `coordinator.reconfigure_provider`.
  Verify RED.

- [ ] **Step 2: Implement `ProviderConfiguration` struct** holding `config_file`, `keychain: Arc<Keychain>` (or `Arc<dyn KeychainBackend>`-backed), `http_client`, `translation_coordinator`, `llm_introspection` (for `test_custom`). Methods:
  - `add(input) -> CustomTranslationProviderView` — move existing free fn body; unchanged rollback.
  - `update(provider_id, input) -> CustomTranslationProviderView` — impose rollback: on `coordinator.replace` failure, restore old def + old key.
  - `remove(provider_id)` — unregister → save config → delete keychain, with the builtin-id guard.
  - `save_credentials(provider_id, credentials)` — validate (incl. deeplx special path, moved as-is) → keychain save (both calls) → `coordinator.reconfigure_provider`.
  - `list_provider_infos() -> Vec<ProviderInfo>` — own the merge (coordinator list + custom defs + builtin detection). `order_provider_infos_for_display` moves here as a pure helper. **`ProviderInfo` moves from `provider_commands.rs` into `configuration.rs`** (decided): it is a domain view, not IPC-only. The command exposes it directly via serde (shape unchanged). The `From<CustomTranslationProviderView>` impl moves with it.  
  - `test_custom_provider(provider_id)` — load def + key, delegate to `llm_introspection.test(def.protocol, def.endpoint, def.model, key)`.
  - `credential_schema(provider_id) -> Vec<CredentialField>` — `coordinator.get(id).credential_fields()`.

  Existing free fns (`add_custom_translation_provider`, `build_*_def`, `create_llm_translation_provider`, `custom_translation_provider_view`, `validate_required_credentials`, parse helpers) become private fns called by methods, or methods themselves. Keep `create_llm_translation_provider` usable by `composition/provider_runtime.rs` if it currently calls it for restore.

- [ ] **Step 3: Add to `AppState` + `composition.rs`** — `pub provider_configuration: Arc<ProviderConfiguration>`. Construct after coordinators + `llm_introspection` exist. Check `hydrate_provider_credentials` / `provider_runtime.rs` still builds (they may call `create_llm_translation_provider` — keep that fn public).

- [ ] **Step 4: Rewire 8 commands** to `state.provider_configuration.*(...)` one-liners. Delete `configure_translation_provider_credentials_inner`, `validate_deeplx_credentials`, `validate_non_blank` (move into configuration if not already), `test_custom_translation_provider_def`. `ProviderInfo` + its `From<CustomTranslationProviderView>` impl already relocated to `configuration.rs` in Task 4 Step 2.

- [ ] **Step 5: Verify**

  ```bash
  cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings
  cd .. && npm test && npm run build
  ```

  Expected: rollback tests green; frontend `providers.test.ts` green (contract unchanged); `provider_commands.rs` now ~250 lines.

---

## Task 5: Docs + final verification

**Files:** `CONTEXT.md`, `ARCHITECTURE.md`.

- [ ] **Step 1: Update `CONTEXT.md`** — deepen the "Provider Configuration Module" entry to reflect the struct + full lifecycle; add a concise "LlmIntrospection" entry and note `LlmModelLister` as the introspection port. Mirror the brevity of existing entries.

- [ ] **Step 2: Update `ARCHITECTURE.md`** — in the runtime seam list, note `application/providers/llm_introspection.rs` as the LLM provider introspection module and `ProviderConfiguration` as the struct owning custom-provider lifecycle; update the "当前演进状态" section.

- [ ] **Step 3: Final verification**

  ```bash
  cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings
  cd .. && npm test && npm run build
  wc -l src-tauri/src/commands/provider_commands.rs   # expect ~250
  grep -nE "state\.(config_file|keychain|http_client)" src-tauri/src/commands/provider_commands.rs || echo "no direct infra access in migrated commands"
  ```

  Expected: all green; no direct infra access in the 15 migrated commands; ~250 lines.

- [ ] **Step 4: Diff review** — `git diff --stat` + `git diff`; confirm every changed line traces to a decision in this plan.

---

## Residual risks
- `list_models` on OpenAIResponses: the responses API shares the OpenAI `/models` list endpoint — confirm in `openai.rs` that `list_models` does not branch on chat-completions vs responses (it shouldn't; listing is endpoint-agnostic). If it must, handle in the client.
- `ProviderInfo` relocation: if `providerStore`/frontend expects the exact serde shape, the move must preserve `#[derive]` + field order/serde attrs. Frontend `providers.test.ts` is the guard.
- Keychain `with_backend` is `#[cfg(test)]`-gated to avoid exposing a production construction path; if `ProviderConfiguration` prod construction needs to inject a real backend it uses `Keychain::new()`.
