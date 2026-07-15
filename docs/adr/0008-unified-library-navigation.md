# ADR 0008: Unified Library Navigation and Workflow Ownership

## Status

Accepted — 2026-07-15

## Context

ADR 0002 placed History and Favorites inside screenshot, translation, and OCR settings sections. That structure kept each feature domain complete, but it duplicated browsing entry points and mixed durable configuration with user content.

The selected three-column design establishes two information groups in the Settings Window:

- Settings: General, Screenshot, Translation, OCR, and Services. Former Advanced settings live in General.
- Library: Favorites and History.

History and Favorites each need one chronological surface that can filter content by screenshot, translation, or OCR. Building that surface requires cross-source querying, global ordering, and pagination. Those decisions are workflow policy, not React rendering behavior.

## Decision

History and Favorites become top-level Library destinations in the Settings Window. Their former feature-domain secondary navigation entries are removed.

Library is a frontend Application workflow concept. Its Application module owns:

- cross-source queries;
- global chronological ordering;
- pagination across source-specific ports;
- conversion into Library list items consumed by Views.
- delete, clear, metadata-update, and rerun-then-copy mutation sequencing.

React Views own rendering, search/filter input state, current page, and the selected item. Views request a page through the Library interface and do not know source-specific query limits or merge algorithms.

A backend Library Index Application module exposes read-only, lightweight global ordering metadata. SQLite applies cross-source ordering and pagination before source-specific ports hydrate only the final page. This prevents deep pages from loading every preceding record or thumbnail.

Backend History, Favorites, and Screenshot Favorites remain separate Application modules. Their storage models and mutation rules are not merged by this decision.

## Consequences

- Users browse saved content through one consistent Library information architecture.
- Settings feature domains contain configuration and shortcuts rather than content collections.
- Frontend Library workflow tests cover cross-source ordering and pagination through the Application interface.
- Source-specific adapter constraints stay hidden from React Views.
- Adding a new Library content source requires extending the Library workflow and its ports, not duplicating merge logic in a View.

This ADR supersedes ADR 0002's History and Favorites navigation placement and its rejection of a unified History surface. ADR 0002's remaining feature-domain settings and centralized Provider decisions remain accepted.
