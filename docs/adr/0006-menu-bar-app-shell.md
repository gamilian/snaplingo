# ADR 0006: Menu Bar Resident App Shell

## Status

Accepted (2026-07-01)

## Context

SnapLingo currently creates a Settings Window at startup and treats macOS Reopen as a possible signal to show Settings. This couples app lifecycle, Settings, and business windows.

Bob is a menu bar app, and Snipaste uses a tray/menu-bar resident model. SnapLingo fits the same product model: global hotkeys and menu actions trigger business workflows, while Settings is explicit.

## Decision

SnapLingo will use a menu bar resident app shell. Startup creates the app runtime, global shortcuts, and menu bar status item, but does not show Settings. Settings opens only through explicit Settings entrypoints.

The Settings Window information architecture from ADR 0002 remains accepted. The app-shell assumption in ADR 0002 that SnapLingo needs a traditional primary main window is superseded.

## Consequences

- Business workflows must not depend on Settings being open.
- `RunEvent::Reopen` does not open Settings in menu bar mode.
- Settings is lazy-created and hidden on close.
- Capture overlay macOS activation logic remains owned by capture infrastructure.

