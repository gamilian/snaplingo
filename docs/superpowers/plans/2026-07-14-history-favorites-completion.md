# History and Favorites Completion Plan

**Status:** Implemented and verified.

**Goal:** Complete translation history, OCR history, translation/OCR favorites, and screenshot favorites without moving persistence or workflow decisions into React views or Tauri commands.

## Audit result

The existing implementation already provides:

- one SQLite history record identity shared by translation and OCR records;
- automatic recording from translation/OCR application events;
- persisted favorite, note, and tag metadata;
- backend query, mutation, and cross-window invalidation ports;
- translation and OCR history/favorite views backed by an in-memory Zustand cache.

The feature is not complete because:

- OCR history and both favorite pages do not hydrate themselves when opened;
- translation/OCR “clear” actions both call the global clear-all command;
- frontend search only examines the first 100 cached records and the backend search seam is unused;
- pagination is absent;
- favorite notes and tags are persisted but cannot be edited in the UI;
- favorite pages are derived from the first history page, so older favorites disappear;
- OCR records persist only an image hash, so thumbnails and re-recognition cannot work;
- screenshot favorites are a placeholder with no backend application module, asset storage, command, or capture action;
- automatic cleanup controls are static and no cleanup policy is implemented.

## Ownership and dependency rules

The implementation keeps the accepted dependency direction:

```text
Frontend View -> Frontend Application -> Platform port -> Tauri command
Backend Command -> Backend Application <- Infrastructure adapters
                                      ^
                              Composition injects ports
```

- Views render data and send actions only.
- Frontend Application owns pagination/search/favorite workflows.
- Commands parse IPC and delegate once.
- Backend Application owns query, cleanup, metadata, and screenshot-favorite workflows.
- SQLite repositories and filesystem asset stores remain Infrastructure.
- Screenshot and OCR image files are stored outside SQLite; SQLite stores metadata and relative paths.

## Confirmed public test seams

Existing architecture documents already define the public seams used by this plan:

1. `application::history::History` for backend history behavior.
2. Backend screenshot-favorites Application interface for asset workflows.
3. Frontend `SettingsRuntime` history/favorites interfaces for view workflows.
4. Tauri platform adapters for IPC payload conversion.
5. Settings Window pages for user-observable loading and actions.

Tests target these interfaces rather than private SQL helpers or React implementation details.

## Execution plan

## Implementation status (2026-07-14)

- Completed: authoritative backend search, favorite-only queries, totals, and pagination.
- Completed: kind-specific history clearing; translation clearing no longer removes OCR history and vice versa.
- Completed: independent Translation/OCR history and favorite hydration.
- Completed: persisted note/tag editing for Translation/OCR favorites.
- Completed: screenshot favorite Application service, SQLite v2 metadata, filesystem originals/thumbnails, capture-toolbar entry point, grid/search/pagination, metadata editing, copy, reveal, and delete.
- Completed: shared-tag cleanup protects tags still referenced by screenshot favorites.
- Completed: OCR source images and thumbnails are persisted outside SQLite, cleaned with records, displayed in Settings, and can be re-recognized.
- Completed: durable automatic cleanup policy, startup/post-insert cleanup, maximum-count enforcement, and favorite exclusion.
- Completed: authoritative tag filtering and complete tag lists across favorite pages.
- Completed: translation favorite JSON export.
- Intentional scope decision: source-type filters were removed because the existing records did not contain trustworthy capture-origin metadata. Reintroducing those filters requires provenance to be added at translation/OCR entry points; displaying guessed values would be incorrect.

### Phase 1 — Correct history semantics

1. Add typed history queries supporting kind, favorite-only, search, limit, and offset.
2. Add kind-specific clear operations while retaining explicit global clear.
3. Route frontend history pages and favorite pages through those queries.
4. Add independent hydration, loading/error state, and pagination.

**Verify:** Backend Application tests, SQLite repository tests, frontend runtime/store/page tests.

### Phase 2 — Complete favorite metadata

1. Expose note and tag editing from translation and OCR favorite cards.
2. Keep metadata updates transactional and broadcast invalidation only after success.
3. Add tag filtering and deterministic normalization.
4. Add translation-favorite export using a frontend Application workflow and platform save-file port.

**Verify:** favorite metadata survives reload; another Settings WebView refreshes after invalidation.

### Phase 3 — Preserve OCR source assets

1. Add a History-owned asset-store port for OCR source images and thumbnails.
2. Persist relative asset references with OCR history metadata.
3. Delete assets when records are deleted or globally cleared.
4. Expose thumbnails and re-recognition through Application interfaces.

**Verify:** OCR favorite survives restart, renders its thumbnail, and can start OCR again.

### Phase 4 — Implement screenshot favorites

1. Add a backend `screenshot_favorites` Application module with repository and file-store ports.
2. Add SQLite v2 metadata tables and filesystem `assets/screenshots` / `assets/thumbnails` storage.
3. Add thin commands and frontend platform adapters.
4. Add a Capture Workspace favorite completion action.
5. Replace the placeholder screenshot favorites page with grid, search, tags, delete, copy, and reveal/open actions.

**Verify:** favoriting from capture writes the file and metadata atomically; rollback removes partial files; favorites survive restart.

### Phase 5 — Automatic cleanup

1. Add durable history policy settings: enabled, retention days, maximum records.
2. Add an Application cleanup workflow that excludes favorites.
3. Run cleanup on startup and after new history insertion, without blocking capture/OCR result delivery.
4. Wire Advanced settings controls to the durable settings runtime.

**Verify:** expired/unfavored records are removed, favorites remain, and associated OCR assets are cleaned.

### Phase 6 — Architecture and release verification

1. Extend architecture tests so Commands cannot own history/favorite policy or filesystem paths.
2. Run all frontend tests and production build.
3. Run all Rust tests, integration tests, format check, and dependency tests.
4. Review the final diff for duplicate state, cross-layer imports, and orphaned compatibility APIs.
