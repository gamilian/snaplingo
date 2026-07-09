# Follow-up: Provider Config Concurrent Read-Modify-Write Race

**Origin:** Codex review #3 of the provider-configuration-deepen refactor (2026-07-09).
**Severity:** P1 (real, but pre-existing and out of scope for the credential-safety refactor).
**Status:** Tracked for a separate follow-up; not a merge blocker for the current refactor.

## Problem

All custom provider config operations use a `load Vec → mutate → save Vec` pattern
without a transaction lock spanning the whole read-modify-write:

- `add_custom_translation_provider` — `src-tauri/src/application/providers/configuration.rs:104`
- `update` — `src-tauri/src/application/providers/configuration.rs:693`
- `remove` — `src-tauri/src/application/providers/configuration.rs:797`

`ConfigFile::save()` (config_file.rs) now holds its lock for the full
clone/write/rename/commit, which protects a *single* save from lost updates.
It does **not** protect the caller's entire RMW transaction: two concurrent
`add`/`update`/`remove` calls can each load a stale `Vec`, apply their own
mutation, and save — the second save overwrites the first, diverging the
in-memory keychain/coordinator state from the on-disk config.

## Risk Assessment

- **Likelihood:** Low. Desktop (Tauri) app; provider config ops are manual,
  low-frequency, user-initiated from the settings UI, which typically serializes
  them. Realistic trigger requires two concurrent windows mutating config.
- **Worst case:** Lost update (a provider add doesn't persist), or config vs.
  keychain divergence if a keychain write succeeded before the overwritten save.

## Suggested Fix Direction

Introduce a single serialization lock (e.g. a `Mutex<()>` or `RwLock` on
`ProviderConfiguration`, or a dedicated transaction guard) held across the
entire RMW for every config-mutating operation (`add`/`update`/`remove`).
Alternatively, move the `Vec<CustomTranslationProviderDef>` into an in-memory
store with a single mutex that serializes all access.

This is an architectural change larger than the surgical credential-safety
scope, so it is deferred to a dedicated follow-up.

## Verification Goal

A test that runs two concurrent `add_custom_translation_provider` (or
add+remove) operations and asserts both providers persist with no divergence
between config file, keychain, and coordinator state.
