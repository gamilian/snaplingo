# Frontend Runtime Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox format for tracking.

**Goal:** Replace shallow src/tauri modules and raw Tauri usage with domain-named Platform adapters injected into Frontend Application runtimes.

**Architecture:** Move command mapping into src/platform/tauri and hide raw event/window mechanics. Views consume Application interfaces only; App.tsx is the composition root and may import Platform adapters but not @tauri-apps packages.

**Tech Stack:** React, TypeScript, Tauri 2, Vitest.

---

### Task 1: Establish portable adapter contracts

**Files:**
- Create: src/application/capture-workspace/ports.ts
- Create: src/application/result-window/ports.ts
- Create: src/application/pinned-image/ports.ts
- Create: src/application/settings/ports.ts
- Create: src/platform/tauri/appEvents.ts
- Create: src/platform/tauri/resultWindow.ts
- Create: src/platform/tauri/captureWindow.ts
- Create: src/platform/tauri/pinnedWindow.ts
- Create: src/platform/tauri/settingsWindow.ts
- Test: src/platform/tauri/appEvents.test.ts
- Test each domain window adapter beside its file

- [ ] Write failing tests for typed result-payload, capture-hotkey, and capture-cancel subscriptions.
- [ ] Write separate failing adapter tests for Result resize/close/drag, Capture reveal, Pinned move/resize, and Settings show/hide operations without exposing raw Tauri objects.
- [ ] Run focused tests and verify RED.
- [ ] Implement minimal domain-named Platform adapters with typed callbacks and cleanup functions; do not create a shared window mechanics bucket.
- [ ] Run focused tests and verify GREEN.
- [ ] Commit with message refactor: add typed frontend tauri runtime adapters.

### Task 2: Move command adapters by domain

**Files:**
- Move: src/tauri/captureSession.ts to src/platform/tauri/capture.ts
- Move: src/tauri/pinnedImage.ts to src/platform/tauri/pinnedImage.ts
- Move: src/tauri/translation.ts to src/platform/tauri/translation.ts
- Move: src/tauri/settings.ts to src/platform/tauri/settings.ts
- Move: src/tauri/providers.ts to src/platform/tauri/providers.ts
- Move remaining command adapters under src/platform/tauri
- Move corresponding tests beside the new modules
- Modify: src-tauri/src/commands/provider_commands.rs
- Modify: src-tauri/src/application/providers/configuration.rs
- Modify: src-tauri/src/lib.rs

- [ ] Move one domain at a time and update tests before callers.
- [ ] Preserve behavior but freely rename internal exported functions for domain clarity.
- [ ] Remove the backend configure_translation_provider command, its lib.rs registration, ProviderConfiguration::save_legacy_api_key, the frontend configureTranslationProvider export, and their tests.
- [ ] Update Platform tests to assert the new command vocabulary.
- [ ] Run src/platform/tauri focused tests.
- [ ] Commit with message refactor: move frontend command adapters by domain.

### Task 3: Add Application runtimes for business windows

**Files:**
- Create: src/application/result-window/platformRuntime.ts
- Create: src/application/capture-workspace/platformRuntime.ts
- Create: src/application/pinned-image/platformRuntime.ts
- Create: src/application/settings/runtime.ts
- Create: src/application/settings/runtime.test.ts
- Test each runtime beside its file

- [ ] Write failing tests showing each runtime translates View actions into portable Platform port calls.
- [ ] For Settings, cover durable settings, Providers, Hotkeys, History, clipboard actions, and the Advanced capture entrypoint through one injected Settings runtime interface.
- [ ] Verify RED.
- [ ] Implement minimal runtime factories accepting injected ports.
- [ ] Verify GREEN.
- [ ] Commit with message refactor: inject frontend platform runtimes.

### Task 4: Migrate Views and composition

**Files:**
- Move: src/components/ScreenshotSession to src/views/CaptureWorkspace
- Move: src/components/ResultWindow to src/views/ResultWindow
- Move: src/components/PinnedImageWindow to src/views/PinnedImageWindow
- Move: src/components/SettingsWindow to src/views/SettingsWindow
- Modify: src/App.tsx
- Modify: all imports under src

- [ ] Update App.tsx to construct Platform adapters and inject Application runtimes.
- [ ] Remove direct Platform imports from Views.
- [ ] Move existing Settings stores behind the injected Settings runtime or adapt them to receive ports; no Settings View may import Platform modules.
- [ ] Update view tests and routing tests.
- [ ] Run focused View and App routing tests.
- [ ] Commit with message refactor: route views through application runtimes.

### Task 5: Delete the old seam and tighten rules

**Files:**
- Delete: src/tauri/**
- Modify: src/architecture/frontendDependencyRules.test.ts

- [ ] Delete old passthrough modules and compatibility exports.
- [ ] Remove the Phase 1 allowlist.
- [ ] Require @tauri-apps imports to exist only under src/platform/tauri.
- [ ] Permit App.tsx to import Platform adapters but not @tauri-apps packages.
- [ ] Reject all Platform imports from src/views.
- [ ] Run focused architecture tests.
- [ ] Commit with message test: enforce frontend platform seam.

### Task 6: Full verification

- [ ] Run npm test.
- [ ] Run npm run build.
- [ ] Run rg for @tauri-apps imports and manually verify only src/platform/tauri matches.
- [ ] Run cargo test --manifest-path src-tauri/Cargo.toml.
