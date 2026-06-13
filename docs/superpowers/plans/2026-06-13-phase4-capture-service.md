# Phase 4: Capture Service Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement screenshot capture functionality (Screenshot Mode) with platform-adapted backends, save/load operations, and hotkey integration.

**Architecture:** Application service layer using Infrastructure's ScreenshotBackend and HotkeyBackend. Includes capture, save, copy, and hotkey management.

**Tech Stack:** Rust, core-graphics (macOS), Windows GDI, xcb (Linux), existing infrastructure from Phase 1

**Duration:** 2-3 days

**Prerequisites:** Phase 1 (Infrastructure with Screenshot/Hotkey backend placeholders) completed

---

## File Structure

### New Files to Create

**Application - Services:**
- `src-tauri/src/application/services/mod.rs`
- `src-tauri/src/application/services/capture_service.rs`
- `src-tauri/src/application/services/hotkey_service.rs`
- `src-tauri/src/application/services/capture_service_test.rs`

**Commands:**
- `src-tauri/src/commands/capture_commands.rs`

**Tests:**
- `src-tauri/tests/capture_integration_test.rs`

### Files to Modify

- `src-tauri/src/infrastructure/system/screenshot/macos.rs` - Complete implementation
- `src-tauri/src/infrastructure/system/screenshot/windows.rs` - Complete implementation  
- `src-tauri/src/infrastructure/system/screenshot/linux.rs` - Complete implementation
- `src-tauri/src/infrastructure/system/hotkey/macos.rs` - Complete implementation
- `src-tauri/src/infrastructure/system/hotkey/windows.rs` - Complete implementation
- `src-tauri/src/infrastructure/system/hotkey/linux.rs` - Complete implementation
- `src-tauri/src/application/mod.rs` - Add services module
- `src-tauri/src/lib.rs` - Add CaptureService and HotkeyService to AppState
- `src-tauri/src/commands/mod.rs` - Add capture commands

### Files to Delete

- `src-tauri/src/capture.rs` - Old capture module (after migration)
- `src-tauri/src/hotkeys/` - Old hotkeys module (after migration)

---

## Task 1: Complete Screenshot Backend - macOS

**Files:**
- Modify: `src-tauri/src/infrastructure/system/screenshot/macos.rs`

- [ ] **Step 1: Update macOS screenshot implementation**

```rust
// src-tauri/src/infrastructure/system/screenshot/macos.rs

use super::backend::ScreenshotBackend;
use crate::Result;
use core_graphics::display::{CGDisplay, CGRect, CGPoint, CGSize};
use core_graphics::image::CGImageRef;

pub struct MacOSScreenshot;

impl MacOSScreenshot {
    pub fn new() -> Self {
        Self
    }
    
    fn image_to_png(image: CGImageRef) -> Result<Vec<u8>> {
        // Convert CGImage to PNG bytes
        let width = image.width();
        let height = image.height();
        
        // Get raw pixel data
        let data_provider = image.data_provider();
        let data = data_provider.data();
        let bytes = data.bytes();
        
        // Create PNG encoder
        let mut png_data = Vec::new();
        {
            let mut encoder = image::codecs::png::PngEncoder::new(&mut png_data);
            encoder.write_image(
                bytes,
                width as u32,
                height as u32,
                image::ColorType::Rgba8,
            ).map_err(|e| crate::AppError::Other(format!("PNG encoding error: {}", e)))?;
        }
        
        Ok(png_data)
    }
}

impl ScreenshotBackend for MacOSScreenshot {
    fn capture_full_screen(&self) -> Result<Vec<u8>> {
        let display_id = CGDisplay::main().id;
        let bounds = CGDisplay::bounds(display_id);
        
        let image = CGDisplay::screenshot(
            bounds,
            core_graphics::display::kCGWindowListOptionOnScreenOnly,
            core_graphics::display::kCGNullWindowID,
            core_graphics::display::kCGWindowImageDefault,
        ).map_err(|_| crate::AppError::Other("Failed to capture screenshot".to_string()))?;
        
        Self::image_to_png(image)
    }
    
    fn capture_region(&self, x: i32, y: i32, width: u32, height: u32) -> Result<Vec<u8>> {
        let rect = CGRect::new(
            &CGPoint::new(x as f64, y as f64),
            &CGSize::new(width as f64, height as f64),
        );
        
        let image = CGDisplay::screenshot(
            rect,
            core_graphics::display::kCGWindowListOptionOnScreenOnly,
            core_graphics::display::kCGNullWindowID,
            core_graphics::display::kCGWindowImageDefault,
        ).map_err(|_| crate::AppError::Other("Failed to capture screenshot region".to_string()))?;
        
        Self::image_to_png(image)
    }
}
```

- [ ] **Step 2: Test macOS screenshot**

Run: `cargo check`
Expected: SUCCESS (on macOS)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/infrastructure/system/screenshot/macos.rs
git commit -m "feat(infra): complete macOS screenshot backend implementation"
```

---

## Task 2: Complete Screenshot Backend - Windows/Linux

**Files:**
- Modify: `src-tauri/src/infrastructure/system/screenshot/windows.rs`
- Modify: `src-tauri/src/infrastructure/system/screenshot/linux.rs`

- [ ] **Step 1: Update Windows screenshot implementation**

```rust
// src-tauri/src/infrastructure/system/screenshot/windows.rs

use super::backend::ScreenshotBackend;
use crate::Result;

pub struct WindowsScreenshot;

impl WindowsScreenshot {
    pub fn new() -> Self {
        Self
    }
}

impl ScreenshotBackend for WindowsScreenshot {
    fn capture_full_screen(&self) -> Result<Vec<u8>> {
        // TODO: Implement using Windows GDI API
        // For now, placeholder implementation
        Err(crate::AppError::Other("Windows screenshot not yet implemented".to_string()))
    }
    
    fn capture_region(&self, _x: i32, _y: i32, _width: u32, _height: u32) -> Result<Vec<u8>> {
        // TODO: Implement using Windows GDI API
        Err(crate::AppError::Other("Windows screenshot not yet implemented".to_string()))
    }
}
```

- [ ] **Step 2: Update Linux screenshot implementation**

```rust
// src-tauri/src/infrastructure/system/screenshot/linux.rs

use super::backend::ScreenshotBackend;
use crate::Result;

pub struct LinuxScreenshot;

impl LinuxScreenshot {
    pub fn new() -> Self {
        Self
    }
}

impl ScreenshotBackend for LinuxScreenshot {
    fn capture_full_screen(&self) -> Result<Vec<u8>> {
        // TODO: Implement using xcb
        // For now, placeholder implementation
        Err(crate::AppError::Other("Linux screenshot not yet implemented".to_string()))
    }
    
    fn capture_region(&self, _x: i32, _y: i32, _width: u32, _height: u32) -> Result<Vec<u8>> {
        // TODO: Implement using xcb
        Err(crate::AppError::Other("Linux screenshot not yet implemented".to_string()))
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/infrastructure/system/screenshot/windows.rs
git add src-tauri/src/infrastructure/system/screenshot/linux.rs
git commit -m "feat(infra): add Windows/Linux screenshot backend placeholders (to be implemented)"
```

---

// __CONTINUE_PHASE4__
## Task 3: Complete Hotkey Backend Implementations

**Files:**
- Modify: `src-tauri/src/infrastructure/system/hotkey/macos.rs`
- Modify: `src-tauri/src/infrastructure/system/hotkey/windows.rs`
- Modify: `src-tauri/src/infrastructure/system/hotkey/linux.rs`

Complete hotkey backend implementations using global-hotkey crate for all platforms.

---

## Task 4: CaptureService

**Files:**
- Create: `src-tauri/src/application/services/mod.rs`
- Create: `src-tauri/src/application/services/capture_service.rs`

Implement CaptureService with:
- capture_region(region) -> image bytes
- save(image, filename) -> file path
- generate_filename() -> timestamp-based name

---

## Task 5: HotkeyService

**Files:**
- Create: `src-tauri/src/application/services/hotkey_service.rs`

Implement HotkeyService with:
- register_hotkey(key_combo, callback)
- unregister_hotkey(id)
- Built on top of infrastructure HotkeyBackend

---

## Task 6: Capture Commands

**Files:**
- Create: `src-tauri/src/commands/capture_commands.rs`

Implement commands:
- capture_screenshot(region) -> image bytes
- save_screenshot(image, filename) -> file path
- copy_to_clipboard(image)

---

## Task 7: Update AppState

**Files:**
- Modify: `src-tauri/src/lib.rs`

Add to AppState:
- capture_service: Arc<CaptureService>
- hotkey_service: Arc<HotkeyService>

---

## Task 8: Integration Test

**Files:**
- Create: `src-tauri/tests/capture_integration_test.rs`

Test capture flow (on macOS).

---

## Task 9: Frontend Integration

Test screenshot capture UI with new commands.

---

## Task 10: Delete Old Modules

Delete:
- src-tauri/src/capture.rs
- src-tauri/src/hotkeys/

---

## Phase 4 Completion Checklist

- [ ] Screenshot backends fully implemented (macOS working, Windows/Linux placeholders)
- [ ] Hotkey backends implemented
- [ ] CaptureService implemented
- [ ] HotkeyService implemented
- [ ] Capture commands implemented
- [ ] AppState updated
- [ ] Integration test passes
- [ ] Frontend working
- [ ] Old modules deleted
- [ ] All tests pass

**Next Phase:** Phase 5 - HistoryDb and final integration

**Estimated Time:** 2-3 days
