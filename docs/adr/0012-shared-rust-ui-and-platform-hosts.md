# ADR 0012: Shared Rust drawing and platform window hosts

SnapLingo's replacement UI shares Rust presentation state, layout, hit testing and CPU rasterization across macOS and Windows. `snaplingo-ui` produces logical-DIP scenes, `snaplingo-render` shapes text with cosmic-text and draws with tiny-skia, and desktop adapters own windows, event delivery, IME, accessibility and pixel presentation. The existing interface is the visual and behavioral acceptance reference. This supersedes the migration strategy of implementing every page separately with AppKit controls.

AirSnip demonstrates the viability of event-driven CPU drawing and explicit native window ownership, but is Windows-only and GPL-3.0. Its implementation is not copied. Sharing drawing gives both supported platforms the same layout and behavior; separate AppKit/Win32 page implementations would duplicate these rules. A WebView-free host does not imply a completed application migration.

Opaque settings windows can use winit and softbuffer. Transparent screenshot, preview and pinned windows require platform alpha presentation: softbuffer's `0x00RRGGBB` buffer cannot carry alpha. System integrations remain platform adapters behind existing Application interfaces. Ordinary text composition can use winit's IME events, while complete marked-text replacement, surrounding-text services and accessibility require explicit native integration and real platform acceptance before replacing existing text editors.

## Rollout constraint at adoption

The original rollout constraint was:

> `snaplingo-core` owns portable Domain/Application workflows, `snaplingo-storage` implements their durable ports, and `snaplingo-app` retains existing system adapters and the temporary Tauri shell. New shared UI and desktop crates do not depend on Tauri. The retained shell remains runnable until each page, capture surface, menu action, input method and installer has passed macOS and Windows acceptance; only then can its dependency and build machinery be deleted.

## Implementation status — 2026-11-09

The default application and package entry now uses the native macOS/Windows
hosts in `snaplingo-app`. The production Tauri shell, commands, configuration,
frontend adapters and dependencies are removed. Core/Storage ownership and the
shared Scene/CPU-rendering decision above are unchanged. `src/` and the isolated
React/AppKit collectors remain independent visual and behavior references;
`reference:*` commands serve them without desktop IPC or native packaging.

macOS uses AppKit window/input integration; Windows uses winit, UIA and layered
alpha presentation. Both route real Settings/provider/library and capture/window
workflows through native adapters. Windows compilation and PE linking have
passed. Windows GUI, installer, IME and accessibility acceptance remain separate
native-runner/manual evidence; removing the shell does not turn a cross compile
into runtime acceptance. Current build and verification contracts are documented
in `ARCHITECTURE.md` and `docs/RELEASING.md`; the quoted adoption constraint above
records the original migration strategy rather than a current production path.
