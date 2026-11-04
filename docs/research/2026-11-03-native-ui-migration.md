# Native UI migration after SnapLingo 0.1.23

Continued on 2026-11-04: the scrolling control panel and pinned-image windows now also use Rust/AppKit on macOS. See the [continuation, validation and measurements](2026-11-04-airsnip-native-rendering.md) for current status; the notes below record the first overlay slice.

## Scope and baseline

The installed macOS arm64 release is SnapLingo 0.1.23. This work leaves the OCR model and runtime resources unchanged. The official app bundle is about 298 MiB on this machine: about 242 MiB of OCR resources and a 55 MiB executable. The locally rebuilt app is about 294 MiB, including the same 242 MiB resources. These are bundle sizes, not memory measurements.

SnapLingo uses React/Tauri WebViews for Settings, Capture Workspace, Result Window, pinned images, and scrolling-capture controls; the Rust backend already owns capture, OCR, translation, and image composition. See [architecture](../../ARCHITECTURE.md), [scrolling window host](../../src-tauri/src/infrastructure/system/scrolling_window.rs), [pinned windows](../../src-tauri/src/infrastructure/system/pinned_window/tauri.rs), and [settings window](../../src-tauri/src/settings_window.rs).

AirSnip is a Windows-only Rust/Win32 application using tiny-skia for CPU rasterization and DirectWrite for text, without a WebView. Its published idle CPU, memory, latency, and size figures are measurements of AirSnip's smaller feature set and Windows environment, not targets already demonstrated for SnapLingo. Its code is GPL-3.0-only; use the architecture as a reference, not copied source in this MIT project. Sources: [AirSnip README](https://github.com/bd-dxg/airsnip/blob/main/README.md), [Cargo manifest](https://github.com/bd-dxg/airsnip/blob/main/Cargo.toml), [performance notes](https://github.com/bd-dxg/airsnip/blob/main/docs/perf-m5.md).

## Implemented first slice

On macOS, the scrolling viewport outline is an AppKit NSBox in a transparent, mouse-pass-through NSWindow; the scrolling thumbnail is an NSImageView in another native window. Their window lifecycle, positioning, capture-time hiding, and preview updates are owned by the existing Rust scrolling host. The control panel remains a WebView. Windows/Linux retain the prior WebView outline and preview. See [native overlays](../../src-tauri/src/infrastructure/system/scrolling_overlays_macos.rs) and [host wiring](../../src-tauri/src/infrastructure/system/scrolling_window.rs).

The native main-thread lifecycle check exercises creation, PNG preview decoding, visibility, capture overlap, and closure. Rust unit, architecture, and integration checks pass. A local 0.1.23 macOS app/DMG built and passed release verification and fixed-identity code-signature checks. It has not been installed over the official release or visually accepted during an actual long screenshot. No CPU or RAM reduction is claimed yet.

## Remaining migration and acceptance

1. Measure idle process tree, screenshot launch, active editing, scrolling capture, and multi-pin memory/CPU on fixed hardware with the 0.1.23 release as the baseline. Track WebKit helper processes separately and report absolute values, peak values, and repeated-run distributions.
2. Replace the macOS scrolling control panel with native controls while preserving undo, preview toggle, copy, save, pin, complete, cancel, keyboard shortcuts, error recovery, UI scale, and floating-window placement. Verify all actions on an actual desktop session before removing its WebView.
3. Migrate the capture editor and pinned-image interaction as platform-specific native views, using existing Rust image-composition and application workflows. Preserve the current editor, text IME, clipboard/output, pin filters, groups, and cross-window state; verify against the [AirSnip parity inventory](2026-11-02-airsnip-parity-audit.md) and SnapLingo's own [feature list](../FEATURES.md).
4. Migrate Result Window and Settings only after native editing flows are stable. They include rich text input, provider configuration, permissions, Library/history/favorites, and translation states. Keep the current WebView version available until each native surface passes functional and accessibility review.
5. Repeat the platform work independently on Windows and Linux. AirSnip's Win32/DirectWrite drawing cannot serve as macOS/Linux rendering code. Verify packages and actual UI behavior on each target system; compare size excluding OCR resources as requested.

Success requires actual UI parity and measured improvement, not only fewer WebViews in source. A native rewrite can increase executable size and CPU if it redraws continuously, so use event-driven invalidation and measure before committing to broader replacement.
