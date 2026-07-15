# ADR 0009: Single-scroll Settings Navigation

## Status

Accepted — 2026-07-15

## Context

The original Settings Window used a second navigation column for pages such as Screenshot Hotkeys, Save Settings, and Editor. Most destinations contained only a few related controls, so the extra level made settings harder to scan and separated controls that belong to one feature workflow.

Some editor values were also exposed as application settings even though users adjust them while annotating. Advanced was a sparse top-level destination, while About needed to remain directly addressable from the menu-bar app shell.

## Decision

General, Screenshot, Translation, and OCR use one scrollable page per feature. Each page owns a sticky section index and grouped setting cards; it does not introduce a persistent secondary navigation column.

- Screenshot settings own shortcuts, output behavior, capture border and mask appearance, selection aids, and pin defaults.
- Annotation color is edited only in the capture editor. Font size and stroke width are edited there and persisted as the defaults for later capture sessions.
- Translation uses selection/screenshot translation and one manual translation window. The retired Input Translation shell action and duplicate shortcut are removed.
- Ordinary screenshot OCR and file OCR show results without implicit copying. Silent OCR remains the copy-oriented workflow.
- Advanced is removed. Its still-relevant maintenance and diagnostic controls live in General; history and favorite capacities remain in the Library capacity panel.
- About is a section of General. The backend emits a Settings navigation intent, while the frontend Settings Application workflow owns tab selection and section scrolling.

## Consequences

- Settings navigation is shallower and feature controls can be scanned in one pass.
- React views receive navigation and persistence actions through frontend Application seams rather than invoking shell behavior directly.
- The Settings window can be lazy-created with an initial route or receive a route event when already open.
- Editor-only controls no longer have duplicate Settings UI.

This ADR supersedes ADR 0002's secondary settings-page structure and Advanced destination. ADR 0008 continues to define the unified Library workflow and top-level History/Favorites destinations.
