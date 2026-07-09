# Design: Provider Config Concurrent Read-Modify-Write Race (P1-A)

**Status:** Design complete (codex, 2026-07-10). Not yet implemented.
**Supersedes the fix direction in:** `2026-07-09-provider-config-concurrent-rmw-followup.md`.

## Problem

All custom provider config operations use `load Vec → mutate → save Vec` without a
transaction lock spanning the full RMW:

- `add_custom_translation_provider` — `configuration.rs`
- `update` — `configuration.rs`
- `remove` — `configuration.rs`

`ConfigFile::save()` already holds its lock across clone/write/rename/commit, which
protects a *single* save from lost updates. It does **not** protect the caller's
entire RMW: two concurrent add/update/remove calls can each load a stale Vec, apply
their own mutation, and save — the second save overwrites the first, diverging
in-memory keychain/coordinator state from on-disk config.

## Scope Analysis (all RMW paths surveyed)

| Key / path | Current state | Verdict |
|---|---|---|
| `custom_translation_providers` | add/update/remove all do load Vec → mutate → save Vec | **Must fix (P1)** |
| `active_translation_providers` | activate/deactivate/reorder/unregister hold the `active` mutex across compute/save/commit | Single Vec RMW already covered by `active` lock; but it interleaves with custom remove/register — route active commands through the same provider-config lock |
| `active_ocr_provider` | `OcrCoordinator::activate` saves first, then sets memory under `active_provider_id` lock | Not a Vec RMW, but two concurrent activates can leave disk != memory. **Small fix in same patch** |
| OCR credentials | `ocr_commands.rs` snapshots keychain → saves → reconfigures | No ConfigFile RMW. Concurrent configure = last-writer on keychain/runtime, not the custom-config race. **Separate P2** |
| `settings` | `update_general/screenshot/translation` snapshot → mutate section → save | Generic config RMW risk; **out of scope** for provider P1 |
| `hotkeys` | `update_hotkey` snapshot → mutate → save + runtime register/unregister | Generic RMW + external state risk; **out of scope** |
| `translation_prompt_strategies` | full-object overwrite save | Not a RMW |

## Recommended Approach: Approach A (converged)

Add a coarse-grained `provider_state_lock: Mutex<()>` to `ProviderConfiguration`. All
translation provider state mutations must go through it and be serialized.

Two small convergences alongside it:

1. Route `activate_translation_provider` / `deactivate_translation_provider` /
   `reorder_active_translation_providers` commands through `ProviderConfiguration`
   wrappers instead of calling `TranslationCoordinator` directly (so they can't bypass
   the lock).
2. Fix `OcrCoordinator::activate` to hold the `active_provider_id` mutex across both
   `config.save` and the memory commit, eliminating the active_ocr_provider disk/memory
   split.

**Do NOT change `ConfigFile`'s API in this P1.** `ConfigFile::save()` already protects a
single save; the provider lifecycle problem is a multi-resource transaction (config +
keychain + coordinator), and the lock belongs in the application layer — not buried in a
config-store closure that would also hold keychain/coordinator side effects (deadlock
and layering risk).

## Implementation Design

### `configuration.rs` (`ProviderConfiguration`)

- Add field `provider_state_lock: Mutex<()>`.
- Add private helper `lock_provider_state()` returning a guard. Prefer mapping a poisoned
  lock to `AppError::Other` rather than `.unwrap()`.
- `add`, `update`, `remove` acquire the guard before the first side effect; release only
  after normal completion *or* rollback completes.
- `save_credentials` also acquires the same guard — it mutates keychain and reconfigures
  the runtime provider, racing `update`/`remove` on the same provider.
- Add wrappers `activate_provider`, `deactivate_provider`, `reorder_active_providers`
  that take the lock then delegate to `TranslationCoordinator`.
- `test_custom_provider` may hold the lock only briefly while loading def + api key —
  **never across `.await`**.
- `add_custom_translation_provider` (free function) should become private
  `add_custom_translation_provider_unlocked`, and stop being re-exported from
  `application/providers/mod.rs`, to prevent lock bypass.

### `provider_commands.rs`

- active/deactivate/reorder commands call `state.provider_configuration.activate_provider(...)`
  etc. instead of `TranslationCoordinator` directly.
- add/update/remove already go through `provider_configuration`; only the underlying
  methods need locking.

### `ocr/coordinator.rs`

- `activate`: lock `active_provider_id` → validate provider exists → release provider map
  lock → `config.save("active_ocr_provider", id)` → write memory active.
- `restore_from_config`: adopt lock order `active_provider_id → providers` to match
  `get_active`, avoiding the current reverse `providers → active` ordering.

## Lock Order

Fixed order (never violate):

1. `ProviderConfiguration.provider_state_lock`
2. Brief calls into `ConfigFile::load/save` or keychain, or entering `TranslationCoordinator`
3. `TranslationCoordinator.active` / `providers`
4. `ConfigFile.store` — only held briefly inside `ConfigFile::{load,save}`

**Forbidden orderings:**

- Never call keychain/coordinator while holding `ConfigFile.store`.
- Never call a `ProviderConfiguration` method while holding a coordinator/provider lock.
- Never hold `provider_state_lock` across an async/network await.

The keychain backend may have internal locks, but no current path takes a keychain backend
lock then calls back into `ProviderConfiguration`, so the order above forms no cycle.

## Test Strategy

New concurrency tests asserting the result equals some serial order:

- `concurrent_custom_adds_are_serialized` — multiple threads behind a barrier call `add`;
  assert all defs present in config, all API keys in keychain, all providers registered,
  active list not lost.
- `concurrent_update_and_remove_is_serializable` — two custom providers pre-seeded;
  concurrent update of one and remove of the other; assert no config/keychain/coordinator
  divergence.
- `remove_and_activate_same_provider_cannot_leave_stale_active_id` — concurrent remove
  and activate of the same id; final active config must not reference an unregistered
  provider.
- `save_credentials_and_remove_do_not_orphan_credentials` — concurrent credential save
  and remove; if remove ultimately succeeds, no provider residue in
  config/coordinator/keychain.
- `concurrent_ocr_activations_keep_disk_and_memory_equal` — concurrent activate of two OCR
  providers; after several rounds assert `config.load("active_ocr_provider") ==
  ocr.get_active().id()`.

## Expected File Changes (this P1)

- `src-tauri/src/application/providers/configuration.rs`
- `src-tauri/src/application/providers/mod.rs`
- `src-tauri/src/commands/provider_commands.rs`
- `src-tauri/src/application/providers/ocr/coordinator.rs`
- `src-tauri/src/application/providers/translation/coordinator_test.rs` (or provider tests
  in `configuration.rs`)
- `src-tauri/src/application/providers/ocr/coordinator_test.rs`

## Out-of-scope follow-ups (separate work)

- `src-tauri/src/application/settings/configuration.rs`
- `src-tauri/src/application/hotkeys/configuration.rs`
- `src-tauri/src/application/hotkeys/runtime.rs`
- A possible new `ConfigFile::update_entry` — but **not** in this P1.
