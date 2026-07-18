# SnapLingo - Product Requirements Document

## Problem Statement

Users frequently encounter text in images or foreign language content that requires translation. Current solutions require juggling multiple tools: one for screenshots, another for OCR, and yet another for translation. This fragmented workflow is slow and disrupts focus.

Existing tools like Bob (macOS-only) demonstrate the value of integrated OCR and translation, but lack comprehensive screenshot editing capabilities and cross-platform support. Users need a unified tool that combines Snipaste-level screenshot editing with powerful OCR and multi-provider translation capabilities, available across all major operating systems.

## Solution

SnapLingo is a cross-platform desktop application that unifies screenshot capture, OCR, and translation into a single seamless workflow. Users can:

1. Capture and edit screenshots with professional-grade annotation tools
2. Instantly extract text from images and translate it using multiple translation engines simultaneously
3. Quickly translate selected text from any application with a keyboard shortcut
4. Compare translation quality across different providers (DeepL, OpenAI, Google, etc.) in a unified interface
5. Maintain full control over their data by using local OCR engines or configuring their own API keys

The application runs as a system tray utility with global hotkeys, requiring no window management. All translation services are bring-your-own-API-key, ensuring privacy and cost control.

## User Stories

1. As a researcher, I want to capture screenshots of academic papers with annotations, so that I can highlight important sections while reading
2. As a language learner, I want to OCR text from images and see translations from multiple providers, so that I can understand nuances and choose the most accurate translation
3. As a developer, I want to translate selected text in documentation with a hotkey, so that I can quickly understand foreign language technical content
4. As a privacy-conscious user, I want to use local OCR engines, so that my sensitive content never leaves my machine
5. As a multilingual professional, I want automatic language detection, so that I don't have to manually specify source languages
6. As a frequent translator, I want to compare DeepL and OpenAI translations side-by-side, so that I can choose the better result
7. As a user, I want to save annotated screenshots to my preferred folder, so that they're organized with my other work
8. As a user, I want to pin screenshots on top of other windows, so that I can reference them while working
9. As a student, I want to OCR mathematical equations from lecture slides, so that I can copy them into my notes
10. As a user, I want my translation history automatically cleaned up, so that old entries don't consume disk space
11. As a non-English speaker, I want translations to automatically target my native language, so that I don't have to configure language pairs each time
12. As a macOS user, I want the same keyboard shortcuts as Snipaste, so that I can switch tools without relearning shortcuts
13. As a Windows user, I want Alt-based hotkeys that follow platform conventions, so that shortcuts feel natural
14. As a Linux user, I want the tool to work on both X11 and Wayland, so that I can use it on modern distributions
15. As a power user, I want to configure custom OpenAI-compatible API endpoints, so that I can use local LLMs or alternative providers
16. As a user, I want Provider endpoints and API keys stored in the local application database, so that Provider configuration has one predictable persistence path
17. As a user editing a screenshot, I want to OCR it without saving first, so that I can decide whether to keep it after seeing the text
18. As a user, I want to edit OCR-recognized text before translation, so that I can fix recognition errors
19. As a user, I want to switch source and target languages and re-translate, so that I can experiment with different language pairs
20. As a user, I want each translation card to have its own copy button, so that I can quickly grab specific translations
21. As a user, I want text-to-speech for both source and translated text, so that I can hear correct pronunciation
22. As a user, I want the result window to stay open until I close it, so that I can reference translations while working
23. As a user, I want to toggle "keep window open" mode, so that I can choose between auto-close and persistent behavior
24. As a user dragging a rectangle tool, I want it to snap to a square when holding Shift, so that I can create perfect shapes
25. As a user drawing with the pen tool, I want it to straighten into a line when holding Shift, so that I can draw precise annotations
26. As a user, I want an undo/redo stack for annotations, so that I can experiment without fear of mistakes
27. As a user, I want to customize tool colors and stroke widths, so that annotations match my preferences
28. As a user, I want window recognition during screenshot capture, so that I can quickly capture specific application windows
29. As a user selecting text that happens to be empty, I want the hotkey to pass through to the system, so that I don't block other shortcuts
30. As a user, I want clear error messages when API keys are invalid, so that I can fix configuration issues
31. As a user, I want the app to start on boot, so that it's always available
32. As a user, I want a system tray menu with quick access to all capture modes, so that I can bypass hotkeys when needed
33. As a user, I want to view my translation history with search, so that I can find past translations
34. As a user, I want to configure how long history is retained, so that I can balance memory and usefulness
35. As a developer integrating SnapLingo, I want a consistent Tauri IPC interface between frontend and backend, so that platform-specific code is isolated
36. As a contributor, I want clear separation between the UI layer and system layer, so that I can work on one without touching the other
37. As a user on a metered connection, I want to choose between local and cloud OCR, so that I can control bandwidth usage
38. As a user, I want the default screenshot save location to be configurable, so that screenshots go where I want them
39. As a user, I want to choose between PNG, JPG, and WebP formats, so that I can balance quality and file size
40. As a user, I want collapsed translation cards by default when multiple providers are active, so that results don't overwhelm the screen
41. As a user, I want keyboard shortcuts to be conflict-detected, so that I know when they won't work
42. As a user, I want a "detect conflicts" button in settings, so that I can diagnose hotkey issues
43. As a user with multiple monitors, I want screenshot capture to work correctly on all displays, so that I can capture from any screen
44. As a user with high-DPI displays, I want crisp rendering and correct scaling, so that the tool looks sharp
45. As a user, I want proxy configuration for API calls, so that I can use the tool behind corporate firewalls
46. As a developer, I want comprehensive logs with configurable levels, so that I can troubleshoot issues
47. As a user uninstalling the app, I want a "clear all data" option in settings, so that I can remove API keys before uninstalling

## Implementation Decisions

### Architecture

**Framework**: Tauri 2.0 with React frontend and Rust backend

**Layer Separation**:
- **Frontend (React + TailwindCSS)**: All UI rendering (screenshot editor with Canvas, Result Window, settings, history). User interaction handling.
- **Backend (Rust)**: System integration (screenshot capture, global hotkeys, clipboard), image processing (layer compositing, format conversion), Provider HTTP clients, language detection, configuration management, history persistence, TTS integration.

**Configuration Storage**:
- Provider endpoints, credentials, and application settings: local SQLite database (`snaplingo.db`)
- API keys are stored unencrypted in SQLite for the current beta scope
- Configuration persists across app reinstalls unless user explicitly clears data

### Core Concepts (from CONTEXT.md)

**Capture Mode**: Five independent entry points
1. Screenshot Mode - F1 - Capture, annotate, save/copy/pin
2. OCR Mode - Option/Alt+A - Capture, OCR, manual translate
3. OCR + Translation Mode - Option/Alt+S - Capture, OCR, auto translate
4. Selection Translation Mode - Option/Alt+D - Copy selection, translate
5. Input Translation Mode - Option/Alt+W - Open translate window, manual input

**Provider**: Internal capability modules (not plugins). Users activate/configure via settings.
- OCR Providers: One active at a time (Tesseract, PaddleOCR, Baidu, Tencent, Google, Azure)
- Translation Providers: Multiple active simultaneously (Google, DeepL, Baidu, Youdao, Tencent, OpenAI, Azure)
- Custom Translation Providers: User-configurable endpoints supporting OpenAI/Claude/Gemini API formats

**Result Window**: Unified display for all translation scenarios
- Editable text area (source text)
- Language selection dropdowns (auto-detect source, smart target)
- Translate button
- Vertical stack of translation cards (one per active Provider)
- Each card: Provider name/icon, collapsible content, copy button, TTS button

**Screenshot Editing Tools** (Snipaste parity):
- Shapes: Rectangle, ellipse, arrow, polyline (Shift for constraints)
- Drawing: Freehand pen, highlighter marker
- Effects: Mosaic, gaussian blur, eraser
- Text annotation with font/size/color
- Color picker, stroke width, fill/stroke toggle
- Undo/redo stack
- Two-layer architecture: original screenshot (for OCR) + annotation layer (for display/save)

**Pinned Screenshot**: Floating always-on-top window
- Draggable, resizable (mouse wheel or corner drag)
- Hover toolbar (close, save, copy, pin toggle)
- Multiple pins supported simultaneously
- Not persisted across app restarts

**Language Detection**: Backend Rust implementation using lightweight library (`lingua` or `franc`)
- Auto-detect source language
- Smart target: Chinese → English, non-Chinese → Chinese
- User can override in Result Window

**TTS**: System TTS engines (macOS `say`, Windows SAPI, Linux `espeak`)
- Source text TTS button in text area
- Per-card TTS button for translated text
- Click to play, button shows "stop" while playing

### Module Boundaries

**Capture Module** (Rust):
- Platform-specific screen capture APIs (macOS `CGWindowListCreateImage`, Windows `BitBlt`, Linux X11/Wayland)
- Window detection for smart selection
- Global hotkey registration (per-platform APIs)
- Clipboard operations (via `arboard` crate)

**Image Processing Module** (Rust):
- Layer composition (merge annotations onto original)
- Format conversion (PNG/JPG/WebP via `image` crate)
- File I/O with atomic writes

**Provider Module** (Rust):
- HTTP client abstraction (`reqwest`)
- Provider trait definitions (`OcrProvider`, `TranslateProvider`)
- Credential retrieval from system stores
- Rate limiting and retry logic per provider

**Configuration Module** (Rust):
- TOML/JSON parser for `~/.snaplingo/config.json`
- System credential store interface (platform-specific)
- Config migration for version upgrades

**History Module** (Rust):
- SQLite or JSON-based persistence
- Automatic cleanup (configurable: 30 days / 1000 entries)
- Optional recording per capture mode

**UI Components** (React):
- ScreenshotEditor: Canvas-based annotation tool with tool palette
- ResultWindow: Text editor + language controls + translation card list
- SettingsWindow: Category navigation + form panels
- HistoryWindow: Searchable list with thumbnails
- TrayMenu: Native system tray integration via Tauri

### State Management

**Frontend State** (React Context or Zustand):
- Current capture mode
- Active tool and tool properties (color, width)
- Result window visibility and content
- Settings panel navigation
- History filters

**Backend State** (Rust):
- Active providers (which are enabled)
- Provider credentials (loaded from system store)
- Hotkey registration status
- Pinned screenshot windows (tracked collection)

### API Contracts

**Tauri Commands** (Rust → exposed to frontend):

```rust
// Screenshot
#[tauri::command]
fn capture_screen(mode: CaptureMode) -> Result<ImageData>

#[tauri::command]
fn save_screenshot(image: ImageData, path: Option<PathBuf>) -> Result<PathBuf>

#[tauri::command]
fn create_pinned_window(image: ImageData) -> Result<WindowId>

// OCR
#[tauri::command]
async fn recognize_text(image: ImageData, provider_id: String) -> Result<OcrResult>

// Translation
#[tauri::command]
async fn translate_text(
    text: String,
    from: Language,
    to: Language,
    provider_ids: Vec<String>
) -> Result<Vec<TranslationResult>>

#[tauri::command]
fn detect_language(text: String) -> Result<Language>

// Providers
#[tauri::command]
fn list_providers(provider_type: ProviderType) -> Result<Vec<ProviderInfo>>

#[tauri::command]
fn set_provider_credential(provider_id: String, key: String, value: String) -> Result<()>

#[tauri::command]
fn activate_provider(provider_id: String) -> Result<()>

#[tauri::command]
fn deactivate_provider(provider_id: String) -> Result<()>

// Configuration
#[tauri::command]
fn get_config() -> Result<Config>

#[tauri::command]
fn update_config(updates: ConfigUpdates) -> Result<()>

// History
#[tauri::command]
fn list_history(filter: HistoryFilter) -> Result<Vec<HistoryEntry>>

#[tauri::command]
fn clear_history() -> Result<()>

// Hotkeys
#[tauri::command]
fn register_hotkey(action: CaptureMode, keys: String) -> Result<()>

#[tauri::command]
fn detect_hotkey_conflicts() -> Result<Vec<HotkeyConflict>>

// TTS
#[tauri::command]
async fn speak_text(text: String, language: Language) -> Result<()>

#[tauri::command]
fn stop_speech() -> Result<()>
```

**Events** (Rust → frontend via Tauri event system):
- `hotkey-triggered`: { mode: CaptureMode }
- `ocr-progress`: { progress: f32 }
- `translation-complete`: { provider_id: String, result: TranslationResult }
- `provider-error`: { provider_id: String, error: String }

### Schema Changes

**Config Schema** (`~/.snaplingo/config.json`):

```json
{
  "version": "1.0.0",
  "general": {
    "language": "en",
    "theme": "system",
    "start_on_boot": true
  },
  "screenshot": {
    "default_save_path": "~/Pictures/SnapLingo",
    "format": "png",
    "quality": 95,
    "default_tool_color": "#FF0000",
    "default_stroke_width": 3
  },
  "ocr": {
    "active_provider": "tesseract"
  },
  "translation": {
    "active_providers": ["google-translate", "deepl"],
    "default_target_language": "zh-CN"
  },
  "hotkeys": {
    "screenshot": "F1",
    "ocr": "Option+A",
    "ocr_translate": "Option+S",
    "selection_translate": "Option+D",
    "input_translate": "Option+W"
  },
  "history": {
    "record_screenshot": false,
    "record_ocr": true,
    "record_translation": true,
    "auto_cleanup_enabled": true,
    "max_age_days": 30,
    "max_entries": 1000
  },
  "advanced": {
    "proxy_url": null,
    "log_level": "info"
  },
  "custom_providers": [
    {
      "id": "my-openai",
      "name": "My OpenAI",
      "type": "translation",
      "api_format": "openai",
      "endpoint": "https://api.openai.com/v1/chat/completions",
      "model": "gpt-4"
    }
  ]
}
```

**History Schema** (SQLite):

```sql
CREATE TABLE history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    capture_mode TEXT NOT NULL, -- 'screenshot', 'ocr', 'ocr_translate', 'selection_translate', 'input_translate'
    thumbnail BLOB,              -- Image thumbnail if applicable
    source_text TEXT,
    source_language TEXT,
    target_language TEXT,
    translations JSON            -- Array of {provider_id, text}
);

CREATE INDEX idx_timestamp ON history(timestamp DESC);
CREATE INDEX idx_capture_mode ON history(capture_mode);
```

### Technical Clarifications

**Clipboard Automation for Selection Translation**:
When the user presses Option/Alt+D:
1. Backend checks if text is selected (attempt to read clipboard selection on Linux/X11, or assume selection exists on macOS/Windows)
2. If no selection detected, hotkey passes through to system (no-op)
3. Simulate Ctrl/Cmd+C keypress via `enigo` crate
4. Wait 100ms for clipboard update
5. Read clipboard content via `arboard`
6. Restore original clipboard content after translation (optional, user-configurable)

**Screenshot Layer Architecture**:
- Frontend Canvas renders annotations in real-time
- On save/OCR: Frontend sends annotation commands (vector format) to backend
- Backend: Render annotations onto original screenshot using `image` crate
- For OCR: Always use original screenshot without annotations (stored separately in memory during editing)

**Result Window Behavior**:
- Window positioned at screen center on first open, then remembers last dragged position
- Loses focus: closes by default
- "Keep open" toggle (checkbox in window): disables focus-loss closing
- ESC key: always closes
- Window persists across translations (user can edit text and re-translate without closing)

**Multi-Provider Translation Execution**:
- Frontend calls `translate_text` with array of provider IDs
- Backend spawns parallel async tasks (one per provider)
- Each completion emits `translation-complete` event
- Frontend updates corresponding card as results stream in
- If all fail, display error summary

**Custom Provider Validation**:
- On save, backend tests endpoint with minimal request
- If connection fails, show warning but allow saving (user might be offline)
- Runtime: Display provider status indicator (green/yellow/red) in settings

**Hotkey Conflict Detection**:
- Attempt to register each configured hotkey
- If registration fails, mark as conflicted
- "Detect conflicts" button re-runs this check and displays list of conflicted hotkeys with system assignments (if detectable)

**Cross-Platform Considerations**:
- macOS: Use native `CGWindowListCreateImage` for screenshot, `CGEvent` for hotkeys
- Windows: Use `BitBlt` for screenshot, `RegisterHotKey` for hotkeys
- Linux: Detect X11 vs Wayland at runtime, use appropriate APIs (XCB for X11, wlr-screencopy for Wayland)
- High-DPI: Tauri handles scaling automatically, but ensure screenshot capture uses physical pixels

**Image Format Quality**:
- PNG: Lossless, default for screenshots with transparency
- JPG: Quality slider 1-100 (default 95), no transparency
- WebP: Quality slider 1-100 (default 90), supports transparency

### Architectural Decisions

**Why Tauri over Electron**:
- Smaller bundle size (~10MB vs ~150MB)
- Better performance (native Rust backend)
- Lower memory footprint
- Rust's system-level capabilities ideal for screenshot/hotkey integration
- Stronger security model (explicit IPC permissions)

**Why React over Vue/Svelte**:
- Rich ecosystem for Canvas-based editors
- Mature libraries for complex UI (settings forms, history tables)
- Team familiarity (assumed, adjust if needed)

**Why Not a Plugin System**:
- Simpler implementation (no dynamic loading, no versioning hell)
- All providers ship built-in, users just configure
- Custom providers cover extensibility needs via standardized API formats
- Reduces attack surface (no arbitrary code execution)

**Why Multi-Provider Translation is Core**:
- Differentiation from simple tools
- Translation quality varies significantly across providers
- Power users want to compare before trusting output
- Justifies "bring your own API key" model (users choose providers worth paying for)

**Why System TTS vs Cloud TTS**:
- Zero latency, works offline
- No additional API costs
- Privacy (text doesn't leave device)
- Good-enough quality for pronunciation checking

## Testing Decisions

### What Makes a Good Test

Tests should validate **external behavior** of modules, not implementation details:
- ✅ Test that `recognize_text` returns OCR results given an image
- ❌ Test that `recognize_text` calls a specific internal HTTP client method
- ✅ Test that `translate_text` with multiple provider IDs returns multiple results
- ❌ Test that parallel tasks are spawned (implementation detail)
- ✅ Test that hotkey registration fails gracefully when system denies it
- ❌ Test specific platform APIs (assume those work, test our abstraction)

### Modules to Test

**Provider Module** (Rust unit tests):
- Each built-in provider (OCR and translation) with mocked HTTP responses
- Custom provider configuration parsing and endpoint formatting
- Error handling (network failures, invalid API keys, malformed responses)
- Retry logic and rate limiting

**Configuration Module** (Rust unit tests):
- Config file parsing (valid and invalid JSON)
- Credential storage/retrieval (mocked system stores)
- Config migration between versions

**Image Processing Module** (Rust unit tests):
- Layer composition (original + annotations = final image)
- Format conversion (PNG ↔ JPG ↔ WebP)
- Annotation rendering (rectangles, arrows, text, blur, mosaic)

**History Module** (Rust integration tests):
- Insert, query, and cleanup operations
- Auto-cleanup triggers (age and count limits)
- Search/filter functionality

**Capture Module** (manual testing only):
- Platform-specific, hard to automate
- Manual smoke tests per release on Mac/Windows/Linux

**Frontend Components** (React component tests with Vitest/Jest):
- ScreenshotEditor: Tool selection, color picking, undo/redo
- ResultWindow: Text editing, language switching, card expand/collapse
- SettingsWindow: Form validation, provider activation rules (OCR single-select enforcement)

**End-to-End** (Playwright or Tauri's test framework):
- Full flows: Screenshot → OCR → Translate
- Selection translation workflow
- Settings persistence across app restarts

### Prior Art

Look for existing tests in Tauri ecosystem:
- Tauri's own test suite for IPC commands
- `tauri-plugin-*` repositories for examples of testing Rust commands
- React screenshot editor libraries for Canvas testing patterns

## Out of Scope

### Explicitly Excluded from MVP (P0 + P1)

**P2 Features** (post-MVP):
1. Theme switching (light/dark modes) - ships with system theme only
2. Interface localization (i18n) - English-only UI initially
3. Advanced OCR (table recognition, formula recognition) - plain text only
4. Cloud TTS with high-quality voices - system TTS only
5. Additional built-in providers beyond core set (defer Mathpix, Apple Vision, etc.)
6. Screenshot history replay (animated GIFs, video recording)
7. Collaborative features (sharing pins, cloud sync)
8. Mobile companion app
9. Browser extension integration
10. Batch processing (multiple screenshots at once)

### Never In Scope

1. Built-in payment/subscription system - always BYOK (bring your own key)
2. User accounts or authentication - fully local app
3. Social features (sharing to Twitter/Reddit/etc.)
4. AI model training or fine-tuning
5. Document editing beyond screenshots (no PDF editor, no Word integration)
6. Full OCR document scanner workflow (multi-page, auto-crop, perspective correction)

## Further Notes

### Accessibility Considerations

- All keyboard shortcuts must be remappable (some users can't use modifier keys)
- Result Window text must be selectable and readable by screen readers
- Screenshot editor tools must have keyboard-only operation mode (arrow keys to move selection, Enter to confirm)
- High contrast mode support (respect OS settings)

### Performance Targets

- Cold start (app launch to tray icon visible): < 2 seconds
- Screenshot capture to editor display: < 100ms
- OCR with Tesseract (medium image): < 3 seconds
- Translation API call: < 2 seconds (network dependent)
- Memory footprint (idle): < 150MB
- Memory footprint (with 5 pinned screenshots): < 300MB

### Privacy and Security

- No telemetry or analytics by default
- Optional crash reporting (explicit opt-in during first run)
- All API keys encrypted in system credential stores
- No network requests without user-configured providers
- Local-first: OCR and translation only when user triggers them
- Clipboard access only during Selection Translation mode (not passive monitoring)

### Localization Strategy (Post-MVP)

- Extract all UI strings to i18n files
- Priority languages: English, Simplified Chinese, Spanish, Japanese
- Leverage community contributions for additional languages
- Language packs ship with app (no runtime downloads)

### Distribution

- GitHub Releases for all platforms
- macOS: Signed DMG (requires Apple Developer account), future: Mac App Store
- Windows: Installer (MSI or NSIS), future: Microsoft Store
- Linux: AppImage (universal), .deb (Debian/Ubuntu), .rpm (Fedora/RHEL), AUR package (Arch)
- Auto-updater: Tauri's built-in updater with GitHub Releases backend

### Community and Contribution

- MIT or Apache 2.0 license (decide before first release)
- Contribution guide: Code style (Prettier for TS, rustfmt for Rust), PR template, issue triage labels
- Separate repo for provider implementations to encourage community additions
- Clear ARCHITECTURE.md documenting module boundaries for contributors

### Migration Path from Competitors

- Import settings from Bob (parse Bob's config.json, map providers)
- Import Snipaste hotkeys (detect and offer to migrate on first run)
- No screenshot history import (formats incompatible)

### Known Limitations

- Screenshot capture on Wayland: Limited by compositor support (some compositors block screen capture for security)
- Global hotkeys on Linux: Requires X11 or compositor protocol support (not all DEs support this)
- OCR accuracy: Tesseract/PaddleOCR are good but not perfect; users may need cloud providers for critical use cases
- Translation context: Single-text translations lack context; LLM providers (OpenAI/Claude) may produce better results for paragraphs
- Windows 10 required (minimum): Tauri 2.0 doesn't support Windows 7/8
- macOS 11+ required: Older versions lack required APIs

### Open Questions for Implementation

1. Should history thumbnails be full-resolution or compressed? (Affects storage, trade-off: detail vs space)
2. Should pinned screenshots save to disk for recovery after crash? (P0 says no, but users might expect it)
3. Should we rate-limit Provider API calls to prevent users accidentally burning credits? (Default: no, respect user's configured limits)
4. Should Result Window support rich text formatting in translations (preserve bold/italic from LLM responses)? (P0: plain text only, defer)
5. Should Selection Translation support auto-detection of code blocks and format them accordingly? (P0: treat as plain text, defer)

### Success Metrics (Post-Launch)

- GitHub stars (community interest)
- Issue resolution time (responsiveness)
- Provider adoption (which providers users actually configure)
- User retention (do users keep using it after first week?)
- Crash-free rate (stability target: 99.5%)

### Risks and Mitigations

**Risk**: Provider API rate limits hit too easily
**Mitigation**: Clear UI feedback on rate limit status, suggest local providers as fallbacks

**Risk**: Hotkey conflicts prevent app from working
**Mitigation**: Robust conflict detection, suggest alternative hotkeys, tray menu fallback

**Risk**: Poor OCR accuracy frustrates users
**Mitigation**: Ship with multiple OCR engines, clear guidance on when to use cloud vs local

**Risk**: Tauri cross-platform issues (e.g., Wayland support)
**Mitigation**: Test on all platforms early, document known limitations, graceful degradation

**Risk**: Large app bundle size on Windows due to WebView2 dependency
**Mitigation**: Use WebView2 Runtime (user may need to install), not embedded (saves ~100MB)

**Risk**: Translation provider APIs change, breaking integrations
**Mitigation**: Version provider implementations separately, schema-based API definitions, community can contribute fixes quickly
