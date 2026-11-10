# Desktop integrations

These implementations belong to the desktop product. `bootstrap/` constructs
adapters and injects their Core-owned contracts into feature sessions. Native
feature code consumes its supplied dependencies instead of resolving AppState.

- `http/` and `llm/` implement product network transports and provider protocols.
- `ocr/` owns packaged local OCR inference engines, models and session lifetime.
  System OCR adapters are in Platform.
- `events/`, `desktop_events.rs`, `runtime.rs` and `logging.rs` own application
  notifications, worker execution and log delivery.
- `system/` contains native-window helpers and product-specific effects:
  dialogs, alpha presentation, theme conversion, shortcut-recording focus and
  update installation. The shared color chooser is a Platform capability.
- `ui_thread*` delivers effects to the application's native event loop.

Reusable screenshot acquisition, selected-text acquisition, system OCR/speech,
clipboard, fonts, hotkeys, permissions, paths, login registration and single-instance
ownership live in `crates/platform`. Import them from `snaplingo_platform` directly.
SQLite repositories and filesystem assets live in `crates/storage` and are imported
from `snaplingo_storage`; integrations does not re-export another crate's module
interface.

Add an implementation here when it needs the desktop product's runtime/resources
or native window presentation. An OS capability that can implement an existing
Core contract independently belongs in Platform. Business validation, ordering,
provider choice and persistence policy remain in their owning Core feature.

Run adapter tests in their owning package:

```bash
cargo test --locked -p snaplingo-platform --all-targets
cargo test --locked -p snaplingo-storage --all-targets
cargo test --locked -p snaplingo --lib integrations
```

Real composition and native-window acceptance tests remain under
`apps/desktop/tests`; their environment requirements are documented separately
from portable library checks.
