# Selected Text Acquisition Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current ad hoc selection translation copy path with a cross-platform selected-text acquisition module that explicitly orders supported selection methods and implements the macOS Easydict-style strategy chain first.

**Architecture:** `application` owns the selection scheme and fallback orchestration; `infrastructure/system/selection` owns concrete system selection methods and platform adapters; `domain` owns selection result language. macOS ships first with self-webview, Accessibility, menu-copy, and shortcut-copy methods, with browser AppleScript added as a follow-up task behind the same method seam.

**Tech Stack:** Rust 2021, Tauri 2, async-trait, macOS ApplicationServices/CoreFoundation FFI, `objc2-app-kit::NSPasteboard`, Enigo, Tokio tests.

---

## File Structure

- Create `src-tauri/src/domain/selection.rs`
  - Domain language for selected text: method kinds, sources, snapshots, attempts, diagnostics, and context.
- Modify `src-tauri/src/domain/mod.rs`
  - Export the selection domain types.
- Create `src-tauri/src/application/services/selected_text_acquirer.rs`
  - Deep application module that owns `SelectionScheme`, ordered fallback, diagnostics, and empty/stale-result handling.
- Modify `src-tauri/src/application/services/mod.rs`
  - Export `SelectedTextAcquirer`.
- Create `src-tauri/src/infrastructure/system/selection/mod.rs`
  - Platform selection module entrypoint.
- Create `src-tauri/src/infrastructure/system/selection/backend.rs`
  - `SelectionMethod`, `SelectionContextProvider`, and `SystemSelectionProvider` adapter seams.
- Create `src-tauri/src/infrastructure/system/selection/registry.rs`
  - Method registry keyed by `SelectionMethodKind`.
- Create `src-tauri/src/infrastructure/system/selection/common/clipboard_transaction.rs`
  - Clipboard transaction orchestration: record sequence, perform action, wait for change, read new text, restore original content when supported.
- Create `src-tauri/src/infrastructure/system/selection/common/shortcut_copy.rs`
  - Shared shortcut-copy timing helpers: wait for modifier release and call the platform copy action.
- Create `src-tauri/src/infrastructure/system/selection/macos/mod.rs`
  - macOS provider that registers supported macOS selection methods.
- Create `src-tauri/src/infrastructure/system/selection/macos/context.rs`
  - Frontmost app detection and Accessibility permission helpers.
- Create `src-tauri/src/infrastructure/system/selection/macos/accessibility.rs`
  - `AXFocusedUIElement -> AXSelectedText` method.
- Create `src-tauri/src/infrastructure/system/selection/macos/self_webview.rs`
  - SnapLingo own-window selection method using Tauri `eval_with_callback`.
- Create `src-tauri/src/infrastructure/system/selection/macos/menu_copy.rs`
  - AX menu-bar Copy method.
- Create `src-tauri/src/infrastructure/system/selection/macos/shortcut_copy.rs`
  - Cmd+C fallback method.
- Create `src-tauri/src/infrastructure/system/selection/macos/pasteboard.rs`
  - macOS pasteboard adapter using `NSPasteboard.changeCount`.
- Create `src-tauri/src/infrastructure/system/selection/windows/mod.rs`
  - Placeholder provider returning no methods, keeping the cross-platform seam compile-ready.
- Create `src-tauri/src/infrastructure/system/selection/linux/mod.rs`
  - Placeholder provider returning no methods, keeping the cross-platform seam compile-ready.
- Modify `src-tauri/src/infrastructure/system/mod.rs`
  - Export `selection`.
- Modify `src-tauri/src/app_state.rs`
  - Add `selected_text_acquirer: Arc<SelectedTextAcquirer>`.
- Modify `src-tauri/src/composition.rs`
  - Build the platform selection provider and inject `SelectedTextAcquirer`.
- Modify `src-tauri/src/commands/mod.rs`
  - Remove temporary selection debug implementation and call `state.selected_text_acquirer.acquire()` from selection translation.
- Modify `src-tauri/src/startup_shortcuts.rs`
  - Keep the existing on-release trigger; only update the command call signature if needed.
- Later follow-up, not first slice: create `src-tauri/src/infrastructure/system/selection/macos/browser_applescript.rs`
  - Browser JavaScript selection via AppleScript for Safari, Chrome, and Edge.

## Design Constraints

- Method priority is explicit in `SelectionScheme`; individual methods never decide their global order.
- Methods answer availability for the current `SelectionContext` and perform one acquisition attempt.
- `SelectedTextAcquirer` owns fallback behavior, diagnostics aggregation, and final error wording.
- Clipboard-backed methods must not read stale clipboard data. They may only return text after a platform sequence/change counter changes.
- Clipboard-backed methods must restore original clipboard contents when platform support is available.
- macOS implementation must not introduce the GPL `selection` crate.
- Existing unrelated untracked files must remain untouched:
  - `screenshot-test.html`
  - `screenshot.html`
  - `script/check-screenshot-flow.mjs`
  - `src-tauri/src/lib.rs.bak`

---

### Task 1: Clean Current Selection Debug Code

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`
- Verify: `src-tauri/src/commands/mod.rs`

- [x] **Step 1: Locate temporary diagnostics**

Run:

```bash
rg -n "DEBUG-selection-a7c9|SELECTION_DEBUG_LOG_TAG|attempt_id|SELECTION_COPY_ATTEMPT_ID" src-tauri/src/commands/mod.rs
```

Expected: finds temporary debug constants, attempt IDs, and tagged log lines.

- [x] **Step 2: Remove tagged debug surface without changing behavior**

Remove:

```rust
const SELECTION_DEBUG_LOG_TAG: &str = "[DEBUG-selection-a7c9]";
static SELECTION_COPY_ATTEMPT_ID: AtomicU64 = AtomicU64::new(1);
```

Then simplify current helper signatures back to behavior-only forms:

```rust
async fn copy_selected_text(app: &tauri::AppHandle) -> Result<String, String> {
    read_selected_text_with(
        read_accessibility_selected_text,
        || copy_selected_text_from_clipboard_after_shortcut(app),
    )
    .await
}
```

Keep existing tests temporarily so this cleanup is behavior-preserving.

- [x] **Step 3: Verify debug tag is gone**

Run:

```bash
rg -n "DEBUG-selection-a7c9|SELECTION_DEBUG_LOG_TAG|SELECTION_COPY_ATTEMPT_ID" src-tauri/src
```

Expected: no matches.

- [x] **Step 4: Run focused existing tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib selection_
```

Expected: existing selection tests pass.

- [x] **Step 5: Checkpoint and self-review**

Review:

```bash
git diff -- src-tauri/src/commands/mod.rs
```

Self-review questions:
- Did this only remove temporary debug instrumentation?
- Did any behavior change before the new module exists?
- Are unrelated untracked files untouched?

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/mod.rs
git commit -m "chore: clean selection debug instrumentation"
```

---

### Task 2: Add Selection Domain Language

**Files:**
- Create: `src-tauri/src/domain/selection.rs`
- Modify: `src-tauri/src/domain/mod.rs`
- Test: `cargo test --manifest-path src-tauri/Cargo.toml --lib selection_domain`

- [x] **Step 1: Write domain tests first**

Add a test module in `src-tauri/src/domain/selection.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attempt_converts_non_blank_text_to_snapshot() {
        let attempt = SelectionAttempt::success(
            SelectionMethodKind::Accessibility,
            SelectionSource::Accessibility,
            " selected text ".to_string(),
            SelectionContext::default(),
        );

        let snapshot = attempt.into_valid_snapshot().unwrap();

        assert_eq!(snapshot.text, " selected text ");
        assert_eq!(snapshot.source, SelectionSource::Accessibility);
    }

    #[test]
    fn attempt_rejects_blank_text() {
        let attempt = SelectionAttempt::success(
            SelectionMethodKind::ShortcutCopy,
            SelectionSource::ShortcutCopy,
            "   ".to_string(),
            SelectionContext::default(),
        );

        assert!(attempt.into_valid_snapshot().is_none());
    }
}
```

- [x] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib selection_domain
```

Expected: compile fails because selection domain types do not exist.

- [x] **Step 3: Implement domain types**

Create `src-tauri/src/domain/selection.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SelectionMethodKind {
    SelfWebview,
    Accessibility,
    BrowserScript,
    MenuCopy,
    ShortcutCopy,
    PrimarySelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionSource {
    SelfWebview,
    Accessibility,
    BrowserScript,
    MenuCopy,
    ShortcutCopy,
    PrimarySelection,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FrontmostApp {
    pub bundle_id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SelectionContext {
    pub frontmost_app: Option<FrontmostApp>,
    pub self_bundle_id: Option<String>,
}

impl SelectionContext {
    pub fn is_frontmost_self(&self) -> bool {
        let Some(self_bundle_id) = self.self_bundle_id.as_deref() else {
            return false;
        };
        self.frontmost_app
            .as_ref()
            .and_then(|app| app.bundle_id.as_deref())
            .is_some_and(|bundle_id| bundle_id == self_bundle_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedTextSnapshot {
    pub text: String,
    pub source: SelectionSource,
    pub frontmost_app: Option<FrontmostApp>,
    pub is_editable: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MethodAvailability {
    Available,
    Unsupported(String),
    Unavailable(String),
}

impl MethodAvailability {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionAttemptStatus {
    Success { text: String, source: SelectionSource },
    Empty,
    Unavailable(String),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionAttempt {
    pub method: SelectionMethodKind,
    pub status: SelectionAttemptStatus,
    pub context: SelectionContext,
    pub is_editable: Option<bool>,
}

impl SelectionAttempt {
    pub fn success(
        method: SelectionMethodKind,
        source: SelectionSource,
        text: String,
        context: SelectionContext,
    ) -> Self {
        Self {
            method,
            status: SelectionAttemptStatus::Success { text, source },
            context,
            is_editable: None,
        }
    }

    pub fn empty(method: SelectionMethodKind, context: SelectionContext) -> Self {
        Self {
            method,
            status: SelectionAttemptStatus::Empty,
            context,
            is_editable: None,
        }
    }

    pub fn failed(method: SelectionMethodKind, context: SelectionContext, message: String) -> Self {
        Self {
            method,
            status: SelectionAttemptStatus::Failed(message),
            context,
            is_editable: None,
        }
    }

    pub fn into_valid_snapshot(self) -> Option<SelectedTextSnapshot> {
        let SelectionAttemptStatus::Success { text, source } = self.status else {
            return None;
        };
        if text.trim().is_empty() {
            return None;
        }
        Some(SelectedTextSnapshot {
            text,
            source,
            frontmost_app: self.context.frontmost_app,
            is_editable: self.is_editable,
        })
    }
}
```

Modify `src-tauri/src/domain/mod.rs`:

```rust
pub mod selection;
pub use selection::{
    FrontmostApp, MethodAvailability, SelectedTextSnapshot, SelectionAttempt,
    SelectionAttemptStatus, SelectionContext, SelectionMethodKind, SelectionSource,
};
```

- [x] **Step 4: Run tests to verify pass**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib selection_domain
```

Expected: tests pass.

- [x] **Step 5: Checkpoint and self-review**

Review:

```bash
git diff -- src-tauri/src/domain/selection.rs src-tauri/src/domain/mod.rs
```

Self-review questions:
- Are these pure domain types, with no macOS/Tauri/clipboard imports?
- Are method kind and source separate enough for future diagnostics?
- Did the domain stay small and stable?

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/domain/selection.rs src-tauri/src/domain/mod.rs
git commit -m "feat: add selected text domain model"
```

---

### Task 3: Add Selection Method Interface and Registry

**Files:**
- Create: `src-tauri/src/infrastructure/system/selection/backend.rs`
- Create: `src-tauri/src/infrastructure/system/selection/registry.rs`
- Create: `src-tauri/src/infrastructure/system/selection/mod.rs`
- Create: `src-tauri/src/infrastructure/system/selection/windows/mod.rs`
- Create: `src-tauri/src/infrastructure/system/selection/linux/mod.rs`
- Modify: `src-tauri/src/infrastructure/system/mod.rs`
- Test: `cargo test --manifest-path src-tauri/Cargo.toml --lib selection_registry`

- [x] **Step 1: Write registry tests first**

In `src-tauri/src/infrastructure/system/selection/registry.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        MethodAvailability, SelectionAttempt, SelectionContext, SelectionMethodKind,
    };
    use async_trait::async_trait;

    struct FakeMethod(SelectionMethodKind);

    #[async_trait]
    impl SelectionMethod for FakeMethod {
        fn kind(&self) -> SelectionMethodKind {
            self.0
        }

        fn availability(&self, _context: &SelectionContext) -> MethodAvailability {
            MethodAvailability::Available
        }

        async fn acquire(&self, context: &SelectionContext) -> SelectionAttempt {
            SelectionAttempt::empty(self.0, context.clone())
        }
    }

    #[test]
    fn registry_returns_methods_by_kind() {
        let registry = SelectionMethodRegistry::new(vec![Box::new(FakeMethod(
            SelectionMethodKind::Accessibility,
        ))]);

        assert!(registry.get(SelectionMethodKind::Accessibility).is_some());
        assert!(registry.get(SelectionMethodKind::MenuCopy).is_none());
    }
}
```

- [x] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib selection_registry
```

Expected: compile fails because registry and trait do not exist.

- [x] **Step 3: Implement method interface**

Create `src-tauri/src/infrastructure/system/selection/backend.rs`:

```rust
use async_trait::async_trait;

use crate::domain::{
    MethodAvailability, SelectionAttempt, SelectionContext, SelectionMethodKind,
};

#[async_trait]
pub trait SelectionMethod: Send + Sync {
    fn kind(&self) -> SelectionMethodKind;
    fn availability(&self, context: &SelectionContext) -> MethodAvailability;
    async fn acquire(&self, context: &SelectionContext) -> SelectionAttempt;
}

pub trait SelectionContextProvider: Send + Sync {
    fn context(&self) -> SelectionContext;
}

pub trait SystemSelectionProvider: SelectionContextProvider {
    fn default_scheme(&self) -> Vec<SelectionMethodKind>;
    fn methods(&self) -> Vec<Box<dyn SelectionMethod>>;
}
```

- [x] **Step 4: Implement registry**

Create `src-tauri/src/infrastructure/system/selection/registry.rs`:

```rust
use std::collections::HashMap;

use crate::domain::SelectionMethodKind;

use super::backend::SelectionMethod;

pub struct SelectionMethodRegistry {
    methods: HashMap<SelectionMethodKind, Box<dyn SelectionMethod>>,
}

impl SelectionMethodRegistry {
    pub fn new(methods: Vec<Box<dyn SelectionMethod>>) -> Self {
        let methods = methods
            .into_iter()
            .map(|method| (method.kind(), method))
            .collect();
        Self { methods }
    }

    pub fn get(&self, kind: SelectionMethodKind) -> Option<&dyn SelectionMethod> {
        self.methods.get(&kind).map(Box::as_ref)
    }
}
```

- [x] **Step 5: Add module exports and placeholders**

Create `src-tauri/src/infrastructure/system/selection/mod.rs`:

```rust
pub mod backend;
pub mod registry;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "linux")]
pub mod linux;

pub use backend::{SelectionContextProvider, SelectionMethod, SystemSelectionProvider};
pub use registry::SelectionMethodRegistry;

#[cfg(target_os = "macos")]
pub use macos::platform_selection_provider;
#[cfg(target_os = "windows")]
pub use windows::platform_selection_provider;
#[cfg(target_os = "linux")]
pub use linux::platform_selection_provider;
```

Create Windows/Linux placeholders:

```rust
use crate::domain::{SelectionContext, SelectionMethodKind};

use super::backend::{SelectionContextProvider, SelectionMethod, SystemSelectionProvider};

pub struct PlatformSelectionProvider;

impl SelectionContextProvider for PlatformSelectionProvider {
    fn context(&self) -> SelectionContext {
        SelectionContext::default()
    }
}

impl SystemSelectionProvider for PlatformSelectionProvider {
    fn default_scheme(&self) -> Vec<SelectionMethodKind> {
        Vec::new()
    }

    fn methods(&self) -> Vec<Box<dyn SelectionMethod>> {
        Vec::new()
    }
}

pub fn platform_selection_provider(
    _app: tauri::AppHandle,
    _self_bundle_id: Option<String>,
) -> PlatformSelectionProvider {
    PlatformSelectionProvider
}
```

Modify `src-tauri/src/infrastructure/system/mod.rs`:

```rust
pub mod selection;
```

- [x] **Step 6: Run tests to verify pass**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib selection_registry
```

Expected: tests pass.

- [x] **Step 7: Checkpoint and self-review**

Review:

```bash
git diff -- src-tauri/src/infrastructure/system/selection src-tauri/src/infrastructure/system/mod.rs
```

Self-review questions:
- Does infra expose a seam without application fallback policy?
- Are Windows/Linux compile-ready without pretending support exists?
- Is no macOS implementation mixed into registry or trait files?

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/infrastructure/system/selection src-tauri/src/infrastructure/system/mod.rs
git commit -m "feat: add selection method infrastructure seam"
```

---

### Task 4: Add Application Selection Acquirer and Explicit Scheme Priority

**Files:**
- Create: `src-tauri/src/application/services/selected_text_acquirer.rs`
- Modify: `src-tauri/src/application/services/mod.rs`
- Test: `cargo test --manifest-path src-tauri/Cargo.toml --lib selected_text_acquirer`

- [x] **Step 1: Write acquirer tests first**

Create `src-tauri/src/application/services/selected_text_acquirer.rs` with tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        MethodAvailability, SelectionAttempt, SelectionContext, SelectionMethodKind,
        SelectionSource,
    };
    use crate::infrastructure::system::selection::{SelectionContextProvider, SelectionMethod};
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    struct FakeMethod {
        kind: SelectionMethodKind,
        availability: MethodAvailability,
        result: SelectionAttemptStatusForTest,
        calls: Arc<Mutex<Vec<SelectionMethodKind>>>,
    }

    enum SelectionAttemptStatusForTest {
        Text(&'static str, SelectionSource),
        Empty,
        Failed(&'static str),
    }

    struct FakeContextProvider;

    impl SelectionContextProvider for FakeContextProvider {
        fn context(&self) -> SelectionContext {
            SelectionContext::default()
        }
    }

    #[async_trait]
    impl SelectionMethod for FakeMethod {
        fn kind(&self) -> SelectionMethodKind {
            self.kind
        }

        fn availability(&self, _context: &SelectionContext) -> MethodAvailability {
            self.availability.clone()
        }

        async fn acquire(&self, context: &SelectionContext) -> SelectionAttempt {
            self.calls.lock().unwrap().push(self.kind);
            match self.result {
                SelectionAttemptStatusForTest::Text(text, source) => {
                    SelectionAttempt::success(self.kind, source, text.to_string(), context.clone())
                }
                SelectionAttemptStatusForTest::Empty => {
                    SelectionAttempt::empty(self.kind, context.clone())
                }
                SelectionAttemptStatusForTest::Failed(message) => {
                    SelectionAttempt::failed(self.kind, context.clone(), message.to_string())
                }
            }
        }
    }

    #[tokio::test]
    async fn acquirer_uses_scheme_order_and_stops_on_first_text() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let acquirer = SelectedTextAcquirer::new(
            SelectionScheme::new(vec![
                SelectionMethodKind::Accessibility,
                SelectionMethodKind::MenuCopy,
                SelectionMethodKind::ShortcutCopy,
            ]),
            SelectionMethodRegistry::new(vec![
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::Accessibility,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Empty,
                    calls: calls.clone(),
                }),
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::MenuCopy,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Text(
                        "menu text",
                        SelectionSource::MenuCopy,
                    ),
                    calls: calls.clone(),
                }),
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::ShortcutCopy,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Text(
                        "shortcut text",
                        SelectionSource::ShortcutCopy,
                    ),
                    calls: calls.clone(),
                }),
            ]),
            Arc::new(FakeContextProvider),
        );

        let snapshot = acquirer
            .acquire_with_context(SelectionContext::default())
            .await
            .unwrap();

        assert_eq!(snapshot.text, "menu text");
        assert_eq!(
            *calls.lock().unwrap(),
            vec![
                SelectionMethodKind::Accessibility,
                SelectionMethodKind::MenuCopy,
            ]
        );
    }

    #[tokio::test]
    async fn acquirer_skips_unavailable_methods() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let acquirer = SelectedTextAcquirer::new(
            SelectionScheme::new(vec![
                SelectionMethodKind::BrowserScript,
                SelectionMethodKind::MenuCopy,
            ]),
            SelectionMethodRegistry::new(vec![
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::BrowserScript,
                    availability: MethodAvailability::Unavailable("not browser".to_string()),
                    result: SelectionAttemptStatusForTest::Text(
                        "browser text",
                        SelectionSource::BrowserScript,
                    ),
                    calls: calls.clone(),
                }),
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::MenuCopy,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Text(
                        "menu text",
                        SelectionSource::MenuCopy,
                    ),
                    calls: calls.clone(),
                }),
            ]),
            Arc::new(FakeContextProvider),
        );

        let snapshot = acquirer
            .acquire_with_context(SelectionContext::default())
            .await
            .unwrap();

        assert_eq!(snapshot.text, "menu text");
        assert_eq!(*calls.lock().unwrap(), vec![SelectionMethodKind::MenuCopy]);
    }
}
```

- [x] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib selected_text_acquirer
```

Expected: compile fails because acquirer types do not exist.

- [x] **Step 3: Implement acquirer**

Add implementation above tests:

```rust
use crate::domain::{
    MethodAvailability, SelectedTextSnapshot, SelectionContext, SelectionMethodKind,
};
use crate::infrastructure::system::selection::{
    SelectionContextProvider, SelectionMethodRegistry,
};
use crate::{AppError, Result};
use std::sync::Arc;

pub struct SelectionScheme {
    ordered_methods: Vec<SelectionMethodKind>,
}

impl SelectionScheme {
    pub fn new(ordered_methods: Vec<SelectionMethodKind>) -> Self {
        Self { ordered_methods }
    }
}

pub struct SelectedTextAcquirer {
    scheme: SelectionScheme,
    registry: SelectionMethodRegistry,
    context_provider: Arc<dyn SelectionContextProvider>,
}

impl SelectedTextAcquirer {
    pub fn new(
        scheme: SelectionScheme,
        registry: SelectionMethodRegistry,
        context_provider: Arc<dyn SelectionContextProvider>,
    ) -> Self {
        Self {
            scheme,
            registry,
            context_provider,
        }
    }

    pub async fn acquire(&self) -> Result<SelectedTextSnapshot> {
        self.acquire_with_context(self.context_provider.context()).await
    }

    pub async fn acquire_with_context(
        &self,
        context: SelectionContext,
    ) -> Result<SelectedTextSnapshot> {
        let mut diagnostics = Vec::new();

        for kind in &self.scheme.ordered_methods {
            let Some(method) = self.registry.get(*kind) else {
                diagnostics.push(format!("{kind:?}: not registered"));
                continue;
            };

            match method.availability(&context) {
                MethodAvailability::Available => {}
                MethodAvailability::Unsupported(reason) | MethodAvailability::Unavailable(reason) => {
                    diagnostics.push(format!("{kind:?}: unavailable: {reason}"));
                    continue;
                }
            }

            let attempt = method.acquire(&context).await;
            let method_name = format!("{:?}", attempt.method);
            if let Some(snapshot) = attempt.into_valid_snapshot() {
                log::info!("Selected text acquired through {method_name}");
                return Ok(snapshot);
            }
            diagnostics.push(format!("{method_name}: no valid text"));
        }

        Err(AppError::System(format!(
            "划词翻译没有获取到文本。尝试过的取词方式：{}",
            diagnostics.join("; ")
        )))
    }
}
```

Export in `src-tauri/src/application/services/mod.rs`:

```rust
pub mod selected_text_acquirer;
pub use selected_text_acquirer::{SelectedTextAcquirer, SelectionScheme};
```

- [x] **Step 4: Run tests to verify pass**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib selected_text_acquirer
```

Expected: tests pass.

- [x] **Step 5: Checkpoint and self-review**

Review:

```bash
git diff -- src-tauri/src/application/services/selected_text_acquirer.rs src-tauri/src/application/services/mod.rs
```

Self-review questions:
- Is method priority explicit in `SelectionScheme`?
- Does application contain zero macOS/clipboard/keyboard details?
- Does the first valid non-blank text stop the chain?

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/application/services/selected_text_acquirer.rs src-tauri/src/application/services/mod.rs
git commit -m "feat: add selected text acquisition scheme"
```

---

### Task 5: Implement macOS Context and Accessibility Method

**Files:**
- Create: `src-tauri/src/infrastructure/system/selection/macos/mod.rs`
- Create: `src-tauri/src/infrastructure/system/selection/macos/context.rs`
- Create: `src-tauri/src/infrastructure/system/selection/macos/accessibility.rs`
- Test: `cargo test --manifest-path src-tauri/Cargo.toml --lib macos_selection_context`

- [x] **Step 1: Write pure tests for context helpers**

In `src-tauri/src/infrastructure/system/selection/macos/context.rs`, add tests for pure helper functions:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_bundle_path_detects_bundle_ancestor() {
        let executable_path =
            std::path::Path::new("/Applications/SnapLingo.app/Contents/MacOS/snaplingo");

        assert_eq!(
            macos_app_bundle_path(executable_path).as_deref(),
            Some(std::path::Path::new("/Applications/SnapLingo.app"))
        );
    }

    #[test]
    fn permission_error_includes_runtime_context() {
        let err = selection_accessibility_permission_error();

        assert!(err.contains("当前运行路径："));
        assert!(err.contains("App Bundle："));
    }
}
```

- [x] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib macos_selection_context
```

Expected: compile fails because macOS context module does not exist.

- [x] **Step 3: Move existing macOS Accessibility helpers**

Move behavior from `src-tauri/src/commands/mod.rs` into macOS modules:

From current commands implementation, relocate:

```rust
macos_accessibility_permission_granted(prompt: bool) -> bool
macos_accessibility_runtime_context() -> String
macos_app_bundle_path(executable_path: &Path) -> Option<PathBuf>
selection_copy_permission_error() -> String
macos_read_accessibility_selected_text(...) -> Result<Option<String>, String>
```

Adapt into:

```rust
pub fn accessibility_permission_granted(prompt: bool) -> bool
pub fn selection_accessibility_permission_error() -> String
pub fn frontmost_context(self_bundle_id: Option<String>) -> SelectionContext
pub fn read_accessibility_selected_text() -> Result<Option<String>, String>
```

Do not keep attempt IDs or temporary debug tags.

- [x] **Step 4: Implement `AccessibilitySelectionMethod`**

In `accessibility.rs`:

```rust
use async_trait::async_trait;

use crate::domain::{
    MethodAvailability, SelectionAttempt, SelectionContext, SelectionMethodKind, SelectionSource,
};
use crate::infrastructure::system::selection::SelectionMethod;

pub struct AccessibilitySelectionMethod;

#[async_trait]
impl SelectionMethod for AccessibilitySelectionMethod {
    fn kind(&self) -> SelectionMethodKind {
        SelectionMethodKind::Accessibility
    }

    fn availability(&self, _context: &SelectionContext) -> MethodAvailability {
        if super::context::accessibility_permission_granted(false) {
            MethodAvailability::Available
        } else {
            MethodAvailability::Unavailable(
                super::context::selection_accessibility_permission_error(),
            )
        }
    }

    async fn acquire(&self, context: &SelectionContext) -> SelectionAttempt {
        match super::context::read_accessibility_selected_text() {
            Ok(Some(text)) if !text.trim().is_empty() => SelectionAttempt::success(
                self.kind(),
                SelectionSource::Accessibility,
                text,
                context.clone(),
            ),
            Ok(_) => SelectionAttempt::empty(self.kind(), context.clone()),
            Err(err) => SelectionAttempt::failed(self.kind(), context.clone(), err),
        }
    }
}
```

- [x] **Step 5: Register macOS provider with Accessibility first**

In `macos/mod.rs`:

```rust
mod accessibility;
mod context;

use crate::domain::{SelectionContext, SelectionMethodKind};
use crate::infrastructure::system::selection::{
    SelectionContextProvider, SelectionMethod, SystemSelectionProvider,
};

pub struct MacSelectionProvider {
    app: tauri::AppHandle,
    self_bundle_id: Option<String>,
}

impl MacSelectionProvider {
    pub fn new(app: tauri::AppHandle, self_bundle_id: Option<String>) -> Self {
        Self {
            app,
            self_bundle_id,
        }
    }
}

impl SelectionContextProvider for MacSelectionProvider {
    fn context(&self) -> SelectionContext {
        context::frontmost_context(self.self_bundle_id.clone())
    }
}

impl SystemSelectionProvider for MacSelectionProvider {
    fn default_scheme(&self) -> Vec<SelectionMethodKind> {
        vec![SelectionMethodKind::Accessibility]
    }

    fn methods(&self) -> Vec<Box<dyn SelectionMethod>> {
        vec![Box::new(accessibility::AccessibilitySelectionMethod)]
    }
}

pub fn platform_selection_provider(
    app: tauri::AppHandle,
    self_bundle_id: Option<String>,
) -> MacSelectionProvider {
    MacSelectionProvider::new(app, self_bundle_id)
}
```

The `app` field becomes necessary when ShortcutCopy and SelfWebview are registered in later tasks.

- [x] **Step 6: Run focused tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib macos_selection_context
cargo test --manifest-path src-tauri/Cargo.toml --lib selected_text_acquirer
```

Expected: tests pass.

- [x] **Step 7: Checkpoint and self-review**

Review:

```bash
git diff -- src-tauri/src/infrastructure/system/selection/macos src-tauri/src/commands/mod.rs
```

Self-review questions:
- Was AX code moved out of commands?
- Is unsafe FFI contained in the macOS infra module?
- Does Accessibility permission produce a user-actionable error?

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/infrastructure/system/selection/macos src-tauri/src/commands/mod.rs
git commit -m "feat: add macos accessibility selection method"
```

---

### Task 6: Add Clipboard Transaction and Shortcut Copy Method

**Files:**
- Create: `src-tauri/src/infrastructure/system/selection/common/clipboard_transaction.rs`
- Create: `src-tauri/src/infrastructure/system/selection/common/shortcut_copy.rs`
- Create: `src-tauri/src/infrastructure/system/selection/common/mod.rs`
- Create: `src-tauri/src/infrastructure/system/selection/macos/pasteboard.rs`
- Create: `src-tauri/src/infrastructure/system/selection/macos/shortcut_copy.rs`
- Modify: `src-tauri/src/infrastructure/system/selection/macos/mod.rs`
- Test: `cargo test --manifest-path src-tauri/Cargo.toml --lib clipboard_transaction`

- [x] **Step 1: Write clipboard transaction tests first**

In `common/clipboard_transaction.rs`, add tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[tokio::test]
    async fn reads_text_only_after_change_count_changes() {
        let counts = Arc::new(Mutex::new(vec![0, 1]));
        let read_counts = counts.clone();

        let text = wait_for_clipboard_text_after_action(
            || async { Ok(()) },
            move || Ok(read_counts.lock().unwrap().remove(0)),
            || Ok("selected text".to_string()),
            Duration::ZERO,
            Duration::ZERO,
        )
        .await
        .unwrap();

        assert_eq!(text, "selected text");
    }

    #[tokio::test]
    async fn rejects_unchanged_clipboard_instead_of_history() {
        let err = wait_for_clipboard_text_after_action(
            || async { Ok(()) },
            || Ok(7),
            || panic!("stale clipboard text should not be read"),
            Duration::ZERO,
            Duration::ZERO,
        )
        .await
        .unwrap_err();

        assert_eq!(err, "Timed out waiting for selected text to reach clipboard");
    }
}
```

- [x] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib clipboard_transaction
```

Expected: compile fails because transaction module does not exist.

- [x] **Step 3: Implement transaction helper**

Move the currently tested logic from `commands/mod.rs` into `common/clipboard_transaction.rs`:

```rust
use std::future::Future;
use std::time::Duration;

pub async fn wait_for_clipboard_text_after_action<Action, ActionFuture, ChangeCount, Read>(
    action: Action,
    mut clipboard_change_count: ChangeCount,
    read_clipboard_text: Read,
    timeout: Duration,
    poll_interval: Duration,
) -> Result<String, String>
where
    Action: FnOnce() -> ActionFuture,
    ActionFuture: Future<Output = Result<(), String>>,
    ChangeCount: FnMut() -> Result<i64, String>,
    Read: FnOnce() -> Result<String, String>,
{
    let before_change_count = clipboard_change_count()?;
    action().await?;

    let started_at = tokio::time::Instant::now();
    loop {
        let current_change_count = clipboard_change_count()?;
        if current_change_count != before_change_count {
            let text = read_clipboard_text()?;
            if text.trim().is_empty() {
                return Err("Selected text is empty".to_string());
            }
            return Ok(text);
        }

        if started_at.elapsed() >= timeout {
            return Err("Timed out waiting for selected text to reach clipboard".to_string());
        }

        tokio::time::sleep(poll_interval).await;
    }
}
```

Keep backup/restore as a second function in `macos/pasteboard.rs`, because rich clipboard item backup uses macOS APIs:

```rust
pub async fn with_temporary_pasteboard_text<Action, ActionFuture>(
    action: Action,
) -> Result<String, String>
where
    Action: FnOnce() -> ActionFuture,
    ActionFuture: Future<Output = Result<(), String>>,
{
    // First implementation may restore only string content if full NSPasteboardItem
    // backup is too large for this slice. Do not claim rich item preservation until tested.
    // Later improve to backup/restore NSPasteboardItem.
}
```

If full NSPasteboardItem backup is not implemented in this task, document the limitation in code and keep old text clipboard restoration behavior out of user-facing claims.

- [x] **Step 4: Implement shortcut modifier wait helper**

Move current `wait_for_shortcut_modifiers_to_clear_with` into `common/shortcut_copy.rs`, keeping pure tests from `commands/mod.rs`.

- [x] **Step 5: Implement macOS shortcut copy method**

In `macos/shortcut_copy.rs`, move behavior from current commands:

```rust
pub struct ShortcutCopySelectionMethod {
    app: tauri::AppHandle,
}
```

Availability:
- requires Accessibility trusted, because Enigo system key events need it.

Acquire:
- wait for shortcut modifiers to clear
- dispatch Cmd+C on main thread using `app.run_on_main_thread`
- use `macos::pasteboard` transaction to wait for `NSPasteboard.changeCount`
- return `SelectionSource::ShortcutCopy`

- [x] **Step 6: Register ShortcutCopy after Accessibility**

Update macOS default scheme:

```rust
vec![
    SelectionMethodKind::Accessibility,
    SelectionMethodKind::ShortcutCopy,
]
```

Update methods:

```rust
vec![
    Box::new(accessibility::AccessibilitySelectionMethod),
    Box::new(shortcut_copy::ShortcutCopySelectionMethod::new(self.app.clone())),
]
```

This requires `MacSelectionProvider` to hold `tauri::AppHandle`.

- [x] **Step 7: Run tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib clipboard_transaction
cargo test --manifest-path src-tauri/Cargo.toml --lib selected_text_acquirer
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Expected: all pass.

- [x] **Step 8: Checkpoint and self-review**

Review:

```bash
git diff -- src-tauri/src/infrastructure/system/selection src-tauri/src/commands/mod.rs
```

Self-review questions:
- Does shortcut copy never read unchanged clipboard data?
- Is modifier-wait behavior preserved?
- Are clipboard-backed methods still last in the scheme?
- Is any claim about full rich clipboard preservation backed by implementation?

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/infrastructure/system/selection src-tauri/src/commands/mod.rs
git commit -m "feat: add shortcut copy selection transaction"
```

---

### Task 7: Add macOS Menu Copy Method

**Files:**
- Create: `src-tauri/src/infrastructure/system/selection/macos/menu_copy.rs`
- Modify: `src-tauri/src/infrastructure/system/selection/macos/mod.rs`
- Test: `cargo test --manifest-path src-tauri/Cargo.toml --lib menu_copy_selection`

- [x] **Step 1: Write pure matching tests first**

In `menu_copy.rs`, isolate pure menu matching rules:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_copy_titles() {
        assert!(is_copy_title("Copy"));
        assert!(is_copy_title("复制"));
        assert!(is_copy_title("拷贝"));
    }

    #[test]
    fn recognizes_copy_action_identifier() {
        assert!(is_copy_identifier("copy:"));
        assert!(!is_copy_identifier("paste:"));
    }
}
```

- [x] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib menu_copy_selection
```

Expected: compile fails because menu copy module does not exist.

- [x] **Step 3: Implement pure matching helpers**

Add:

```rust
fn is_copy_identifier(identifier: &str) -> bool {
    identifier == "copy:"
}

fn is_copy_title(title: &str) -> bool {
    matches!(
        title,
        "Copy" | "复制" | "拷贝" | "拷貝" | "複製" | "コピー" | "복사"
    )
}
```

Keep the initial title set small and easy to extend; identifier is preferred.

- [x] **Step 4: Implement menu copy action**

Implement a macOS-only method that:
- gets the frontmost application AX element
- reads its menu bar children
- searches likely Edit menu first, then adjacent menus, then whole tree
- identifies Copy by `AXIdentifier == "copy:"`, falling back to title + command char
- requires enabled menu item
- performs `AXPress`
- reads text through the same pasteboard transaction

Expose:

```rust
pub struct MenuCopySelectionMethod;
```

Availability:
- Accessibility trusted
- frontmost app is not SnapLingo itself unless we explicitly decide self-copy is acceptable for the current window

Acquire:
- `AXPress` the enabled copy item
- return `SelectionSource::MenuCopy`

- [x] **Step 5: Update macOS scheme priority**

Update default scheme:

```rust
vec![
    SelectionMethodKind::Accessibility,
    SelectionMethodKind::MenuCopy,
    SelectionMethodKind::ShortcutCopy,
]
```

Register MenuCopy before ShortcutCopy.

- [x] **Step 6: Run tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib menu_copy_selection
cargo test --manifest-path src-tauri/Cargo.toml --lib selected_text_acquirer
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Expected: all pass.

- [x] **Step 7: Checkpoint and self-review**

Review:

```bash
git diff -- src-tauri/src/infrastructure/system/selection/macos/menu_copy.rs src-tauri/src/infrastructure/system/selection/macos/mod.rs
```

Self-review questions:
- Does MenuCopy precede ShortcutCopy?
- Does MenuCopy require enabled Copy item?
- Does it share the same stale-clipboard protection?
- Does unsafe AX traversal stay inside macOS infra?

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/infrastructure/system/selection/macos/menu_copy.rs src-tauri/src/infrastructure/system/selection/macos/mod.rs
git commit -m "feat: add macos menu copy selection method"
```

---

### Task 8: Add Self Webview Selection Method

**Files:**
- Create: `src-tauri/src/infrastructure/system/selection/macos/self_webview.rs`
- Modify: `src-tauri/src/infrastructure/system/selection/macos/mod.rs`
- Test: `cargo test --manifest-path src-tauri/Cargo.toml --lib self_webview_selection`

- [x] **Step 1: Write JS snippet test first**

In `self_webview.rs`, keep the JS snippet buildable and testable:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_script_reads_window_selection() {
        let script = selection_script();

        assert!(script.contains("window.getSelection"));
        assert!(script.contains("toString"));
    }
}
```

- [x] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib self_webview_selection
```

Expected: compile fails because self webview module does not exist.

- [x] **Step 3: Implement self-webview method**

Use Tauri `WebviewWindow::eval_with_callback`:

```rust
fn selection_script() -> &'static str {
    r#"
    (() => {
      const selection = window.getSelection && window.getSelection();
      return selection ? selection.toString() : "";
    })()
    "#
}
```

Method behavior:
- availability returns `Available` only when `context.is_frontmost_self()` is true and main window exists
- acquire sends JS to the main window and waits on a Tokio oneshot with a short timeout, such as 300ms
- parse callback JSON string defensively
- return `SelectionSource::SelfWebview`

- [x] **Step 4: Put SelfWebview first in macOS scheme**

Update default scheme:

```rust
vec![
    SelectionMethodKind::SelfWebview,
    SelectionMethodKind::Accessibility,
    SelectionMethodKind::MenuCopy,
    SelectionMethodKind::ShortcutCopy,
]
```

Register `SelfWebviewSelectionMethod` with `tauri::AppHandle`.

- [x] **Step 5: Run tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib self_webview_selection
cargo test --manifest-path src-tauri/Cargo.toml --lib selected_text_acquirer
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Expected: all pass.

- [x] **Step 6: Checkpoint and self-review**

Review:

```bash
git diff -- src-tauri/src/infrastructure/system/selection/macos/self_webview.rs src-tauri/src/infrastructure/system/selection/macos/mod.rs
```

Self-review questions:
- Does this method only apply when SnapLingo is frontmost?
- Is JavaScript evaluation timeout bounded?
- Does this avoid treating self webview as the global solution?

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/infrastructure/system/selection/macos/self_webview.rs src-tauri/src/infrastructure/system/selection/macos/mod.rs
git commit -m "feat: add self webview selection method"
```

---

### Task 9: Wire Acquirer into AppState and Selection Translation Command

**Files:**
- Modify: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/composition.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Test: `cargo test --manifest-path src-tauri/Cargo.toml --lib`

- [x] **Step 1: Write command-level seam test if feasible**

If `open_selection_translation_window` can be split without Tauri runtime setup, extract:

```rust
async fn selected_text_for_translation(
    acquirer: &SelectedTextAcquirer,
) -> Result<String, String>
```

Test:

```rust
#[tokio::test]
async fn selected_text_for_translation_returns_acquirer_text() {
    // Use a fake acquirer only if the interface is easy to test.
    // If not, rely on SelectedTextAcquirer tests from Task 4 and keep command thin.
}
```

If this creates a shallow fake-only seam, skip the command test and keep the command as a thin integration call.

- [x] **Step 2: Add acquirer to AppState**

Modify `src-tauri/src/app_state.rs`:

```rust
pub selected_text_acquirer: Arc<SelectedTextAcquirer>,
```

Add import:

```rust
SelectedTextAcquirer,
```

- [x] **Step 3: Build acquirer in composition**

Modify `src-tauri/src/composition.rs`:

```rust
use crate::infrastructure::system::selection::{
    platform_selection_provider, SelectionMethodRegistry, SystemSelectionProvider,
};
use crate::application::{SelectedTextAcquirer, SelectionScheme};
```

During `build_app_state`:

```rust
let self_bundle_id = Some(_app.config().identifier.clone());
let selection_provider = Arc::new(platform_selection_provider(_app.clone(), self_bundle_id));
let selection_scheme = SelectionScheme::new(selection_provider.default_scheme());
let selected_text_acquirer = Arc::new(SelectedTextAcquirer::new(
    selection_scheme,
    SelectionMethodRegistry::new(selection_provider.methods()),
    selection_provider,
));
```

If `AppHandle::config().identifier` type differs, use the current Tauri config accessor that returns the bundle identifier string.

- [x] **Step 4: Simplify selection command**

Change command signature from:

```rust
pub async fn open_selection_translation_window(app: tauri::AppHandle) -> Result<(), String>
```

to:

```rust
pub async fn open_selection_translation_window(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String>
```

Then:

```rust
let snapshot = state
    .selected_text_acquirer
    .acquire()
    .await
    .map_err(|e| e.to_string())?;
open_translation_result_window(snapshot.text, app)
```

- [x] **Step 5: Update shortcut trigger call**

Modify `src-tauri/src/startup_shortcuts.rs` selection branch:

```rust
let state = app.state::<AppState>();
if let Err(err) = commands::open_selection_translation_window(app.clone(), state).await {
    ...
}
```

If Tauri `State` cannot be moved into the spawned future this way, create a command helper:

```rust
pub async fn open_selection_translation_window_for_state(
    app: tauri::AppHandle,
    state: &crate::AppState,
) -> Result<(), String>
```

and call it with `state.inner()`.

- [x] **Step 6: Remove old selection helper code from commands**

Delete from `src-tauri/src/commands/mod.rs` after replacement:
- old AX helper functions
- old shortcut copy helper functions
- old clipboard changeCount functions
- old selection unit tests now covered by domain/acquirer/infra tests

Keep:
- window-opening functions
- hotkey configuration commands
- screenshot command functions

- [ ] **Step 7: Run tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml --tests
```

Expected: all pass.

Checkpoint result: `cargo test --manifest-path src-tauri/Cargo.toml --lib` passed
with 227 passed, 0 failed. `cargo test --manifest-path src-tauri/Cargo.toml --tests`
ran through lib/main/capture tests but failed in existing
`infrastructure_integration_test::test_http_client_integration` with
`system-configuration` panic `Attempted to create a NULL object`.

- [x] **Step 8: Checkpoint and self-review**

Review:

```bash
git diff -- src-tauri/src/app_state.rs src-tauri/src/composition.rs src-tauri/src/commands/mod.rs src-tauri/src/startup_shortcuts.rs
```

Self-review questions:
- Is command layer thin again?
- Does command know nothing about AX, pasteboard, or Cmd+C?
- Is AppState composition the only place wiring platform selection happens?

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/app_state.rs src-tauri/src/composition.rs src-tauri/src/commands/mod.rs src-tauri/src/startup_shortcuts.rs
git commit -m "feat: wire selected text acquirer into translation"
```

---

### Task 10: Add macOS Browser AppleScript Method

**Files:**
- Create: `src-tauri/src/infrastructure/system/selection/macos/browser_applescript.rs`
- Modify: `src-tauri/src/infrastructure/system/selection/macos/mod.rs`
- Test: `cargo test --manifest-path src-tauri/Cargo.toml --lib browser_applescript_selection`

- [x] **Step 1: Write pure browser support tests first**

In `browser_applescript.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_supported_browser_bundle_ids() {
        assert!(is_supported_browser("com.apple.Safari"));
        assert!(is_supported_browser("com.google.Chrome"));
        assert!(is_supported_browser("com.microsoft.edgemac"));
        assert!(!is_supported_browser("com.apple.TextEdit"));
    }

    #[test]
    fn chrome_script_reads_window_selection() {
        let script = browser_selection_script("com.google.Chrome").unwrap();

        assert!(script.contains("execute javascript"));
        assert!(script.contains("window.getSelection().toString()"));
    }
}
```

- [x] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib browser_applescript_selection
```

Expected: compile fails because browser AppleScript module does not exist.

- [x] **Step 3: Implement browser detection and scripts**

Support:

```rust
const SAFARI: &str = "com.apple.Safari";
const CHROME: &str = "com.google.Chrome";
const EDGE: &str = "com.microsoft.edgemac";
```

Scripts:

Chrome/Edge:

```applescript
tell application id "{bundle_id}"
   tell active tab of front window
       set selection_text to execute javascript "window.getSelection().toString();"
   end tell
end tell
```

Safari:

```applescript
tell application id "com.apple.Safari"
    tell front window
        set selection_text to do JavaScript "window.getSelection().toString();" in current tab
    end tell
end tell
```

- [x] **Step 4: Implement AppleScript runner**

Use `osascript` first for simplicity:

```rust
tokio::process::Command::new("osascript")
    .arg("-e")
    .arg(script)
    .output()
    .await
```

Timeout:

```rust
tokio::time::timeout(Duration::from_millis(300), command_future)
```

Map failure to `SelectionAttempt::failed`.

- [x] **Step 5: Register BrowserScript before MenuCopy**

Update macOS default scheme:

```rust
vec![
    SelectionMethodKind::SelfWebview,
    SelectionMethodKind::Accessibility,
    SelectionMethodKind::BrowserScript,
    SelectionMethodKind::MenuCopy,
    SelectionMethodKind::ShortcutCopy,
]
```

Availability:
- frontmost bundle id is Safari/Chrome/Edge

- [x] **Step 6: Run tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib browser_applescript_selection
cargo test --manifest-path src-tauri/Cargo.toml --lib selected_text_acquirer
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Expected: all pass.

- [x] **Step 7: Checkpoint and self-review**

Review:

```bash
git diff -- src-tauri/src/infrastructure/system/selection/macos/browser_applescript.rs src-tauri/src/infrastructure/system/selection/macos/mod.rs
```

Self-review questions:
- Does BrowserScript only run for supported browsers?
- Is timeout bounded?
- Is BrowserScript before clipboard-backed methods?
- Is the user-facing failure still handled by acquirer diagnostics?

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/infrastructure/system/selection/macos/browser_applescript.rs src-tauri/src/infrastructure/system/selection/macos/mod.rs
git commit -m "feat: add browser script selection method"
```

---

### Task 11: Manual Verification and Final Cleanup

**Files:**
- Modify only if verification reveals issues.
- Verify: full Rust tests and Tauri build.

- [x] **Step 1: Format and static checks**

Run:

```bash
rustfmt --edition 2021 --check --config skip_children=true \
  src-tauri/src/domain/selection.rs \
  src-tauri/src/application/services/selected_text_acquirer.rs \
  src-tauri/src/infrastructure/system/selection/backend.rs \
  src-tauri/src/infrastructure/system/selection/registry.rs \
  src-tauri/src/infrastructure/system/selection/common/clipboard_transaction.rs \
  src-tauri/src/infrastructure/system/selection/common/shortcut_copy.rs \
  src-tauri/src/infrastructure/system/selection/macos/context.rs \
  src-tauri/src/infrastructure/system/selection/macos/accessibility.rs \
  src-tauri/src/infrastructure/system/selection/macos/self_webview.rs \
  src-tauri/src/infrastructure/system/selection/macos/menu_copy.rs \
  src-tauri/src/infrastructure/system/selection/macos/shortcut_copy.rs \
  src-tauri/src/infrastructure/system/selection/macos/pasteboard.rs \
  src-tauri/src/infrastructure/system/selection/macos/browser_applescript.rs
git diff --check
```

Expected: no output from `git diff --check`; rustfmt check passes.

- [x] **Step 2: Run full backend tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml --tests
```

Expected: all pass.

Checkpoint result: `cargo test --manifest-path src-tauri/Cargo.toml --lib` passed
with 229 passed, 0 failed. `cargo test --manifest-path src-tauri/Cargo.toml --tests`
passed after making `test_http_client_integration` use a local mock server and an
injected no-proxy reqwest client instead of a real external website. This avoids
the current environment's macOS `system-configuration` proxy panic while keeping
the production default HTTP client unchanged.

- [x] **Step 3: Confirm debug instrumentation is gone**

Run:

```bash
rg -n "DEBUG-selection-a7c9|SELECTION_DEBUG_LOG_TAG|attempt_id" src-tauri/src
```

Expected: no matches.

- [ ] **Step 4: Build app**

If code signing keychain is locked, unlock it first:

```bash
KEYCHAIN="$HOME/.snaplingo/codesign/SnapLingoLocalCodesign.keychain-db"
PASSWORD_FILE="$HOME/.snaplingo/codesign/keychain-password.txt"
/usr/bin/security unlock-keychain -p "$(cat "$PASSWORD_FILE")" "$KEYCHAIN"
/usr/bin/security set-key-partition-list -S apple-tool:,apple: -s -k "$(cat "$PASSWORD_FILE")" "$KEYCHAIN" >/dev/null 2>&1 || true
SNAPLINGO_CODESIGN_IDENTITY="SnapLingo Local Code Signing" SNAPLINGO_CODESIGN_KEYCHAIN="$KEYCHAIN" npm run tauri:build
```

Expected: build succeeds.

Checkpoint result: frontend build, Rust release build, app bundle, and app signing completed.
`npm run tauri:build` exited 1 during DMG bundling: `failed to run .../bundle_dmg.sh`.
The preceding keychain unlock also returned `One or more parameters passed to a function were not valid`.
Minimal rerun with `bundle_dmg.sh --skip-jenkins` failed at the first `hdiutil create`
step with `hdiutil: create failed - 设备未配置`, before Finder AppleScript or signing.

- [ ] **Step 5: Manual app matrix**

Install/run the signed app, then test selection translation:

```text
SnapLingo settings text:
  Expected source: SelfWebview or Accessibility; no stale clipboard.

TextEdit:
  Expected source: Accessibility or MenuCopy.

Chrome normal page:
  Expected source: Accessibility or BrowserScript.

Safari normal page:
  Expected source: BrowserScript if browser JS permission allows it; otherwise MenuCopy/ShortcutCopy.

VSCode:
  Expected source: Accessibility or MenuCopy.

No selected text:
  Expected: clear "划词翻译没有获取到文本" error, not historical clipboard.
```

- [ ] **Step 6: Inspect logs**

Run:

```bash
tail -n 200 "$HOME/Library/Logs/com.snaplingo.app/SnapLingo.log" | rg "Selected text acquired|划词翻译没有获取到文本|Accessibility|MenuCopy|ShortcutCopy|BrowserScript"
```

Expected: method outcome is visible enough for support/debugging without temporary debug tags.

Checkpoint result: no matching recent log lines were available in the current app log tail.

- [x] **Step 7: Final self-review**

Review:

```bash
git status --short
git diff --stat
```

Self-review questions:
- Are old command-layer selection helpers deleted?
- Are platform details contained under `infrastructure/system/selection`?
- Is priority explicit and test-covered?
- Are clipboard-backed methods last?
- Are Windows/Linux placeholders honest and compile-ready?
- Are unrelated untracked files untouched?

- [ ] **Step 8: Final commit**

```bash
git add src-tauri docs/superpowers/plans/2026-06-23-selected-text-acquisition.md
git commit -m "feat: add cross-platform selected text acquisition"
```

---

## Execution Recommendation

Implement tasks 1-9 first as the first working slice:

```text
SelfWebview -> Accessibility -> MenuCopy -> ShortcutCopy
```

Then implement Task 10 as a second slice:

```text
BrowserScript
```

Reason: the current bug is already covered by self-webview, menu-copy, and stale-clipboard-safe shortcut fallback. Browser AppleScript is valuable but has separate permission and browser-setting behavior, so it is cleaner to verify independently.
