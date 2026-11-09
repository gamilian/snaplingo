# Shared Rust Library migration checkpoint

Investigated 2026-11-08 against the retained React Library workflow and the
portable Rust application/storage crates. This note records the migration seam
and the current implementation evidence; it does not claim that a native
Library page is wired into a desktop window.

## Existing behavior owned by the Library workflow

- `src/application/settings/library.ts` exposes one `SettingsLibrary` interface
  for History and Favorites. It translates page number to `offset = page ×
  pageSize`, delegates filtered pages to the source port, and uses the backend
  index for `all` pages.
- For an `all` page, the index is queried first. Only the returned IDs are sent
  to `queryTranslationHistory`, `queryOcrHistory`, `queryFavorites`, or
  `queryScreenshotFavorites`; results are re-ordered by `kind:id` to match the
  index. Missing IDs are skipped after concurrent deletion.
- History favorite membership is hydrated with
  `queryFavoriteKeys` for the visible records. It does not load the complete
  Favorites table or thumbnails.
- Stable visible keys are `translation:<id>`, `ocr:<id>`, and
  `screenshot:<id>` in `src/application/settings/library.ts`; content favorite
  membership uses the JSON identities in `src/application/favorites/identity.ts`.
- `src/views/SettingsWindow/Library/HistoryLibraryPage.tsx` and
  `FavoritesLibraryPage.tsx` own selection, search/filter state, page number,
  loading state, and rendering. `LibraryLayout.tsx` owns the 20-item page
  controls and two-pane list/detail presentation. Metadata blur saves note and
  comma-separated tags through the application interface.

## Rust implementation seam

- `crates/core/src/application/library_index.rs` owns the read-only
  index interface and clamps index limits to 1–100.
- `crates/core/src/application/library_index/page.rs` is now exposed
  as the `LibraryPages` application workflow. It owns the cross-source query,
  index-first hydration, stable `LibraryRecord` keys, favorite membership,
  deletion, history clearing, metadata routing, and OCR rerun orchestration.
  Its public surface is `LibraryPages::query`, `delete`, `clear_history`,
  `update_metadata`, `favorite`, `rerun_ocr_and_copy`, `copy_screenshot`, and
  `reveal_screenshot`. OCR rerun must succeed before `LibraryClipboard` is
  invoked. The service accepts existing source workflows in its constructor;
  it opens no database and constructs no platform adapter.
- `crates/storage/src/database/library_index.rs` orders history and
  favorites in SQLite before hydration and applies `LIMIT/OFFSET`. The source
  repositories accept `ids`, so a page can be hydrated without loading earlier
  rows or image assets.
- `crates/core/src/application/history/mod.rs`,
  `favorites/mod.rs`, and `screenshot_favorites/mod.rs` remain separate source
  modules. Their asset stores hydrate only returned OCR/screenshot records.

## Data projection for shared drawing

`LibraryPageSnapshot` is the application projection: `items`, `total`, resolved
zero-based `page`, and the visible History `favorite_keys`. `LibraryRecord`
preserves the complete typed source record, its namespaced stable key and source
kind. Its timestamps, text, per-provider translation results, note/tags, image
dimensions and optional thumbnail data URL are sufficient for the current list
and detail content. A Settings adapter can convert it to a dependency-free
`snaplingo-ui` scene snapshot; database/source paths do not belong in the
portable drawing model.

The shared presentation state should own the current destination, filter,
search draft, request revision, resolved page, selected stable key, list/detail
scroll offsets and note/tag drafts. Search/filter changes reset the requested
page; a matching response preserves selection only when its stable key remains
visible. A request revision rejects stale asynchronous responses, matching the
React effects' cancellation behavior. This remains presentation work; it should
not duplicate SQL ordering, favorite fingerprints, asset cleanup, capacity or
OCR workflow rules.

`crates/app/src/app_state.rs` already exposes the six required source
workflows through `history.history`, `favorites.favorites`,
`screenshot_favorites.favorites`, `library_index`, `history.ocr_replay`, and
`favorites.ocr_application`. A native Library host can inject those existing
Arcs into `LibraryPages::new` when it mounts. No new production `AppState`
field or parallel database is required for that connection.

## Remaining presentation and behavior work

- Draw both destinations with the existing `LibraryLayout.tsx` list/detail
  geometry and theme-specific overrides in `src/styles/themes.css`. Preserve
  filters, list title/preview rules, provider detail cards, image previews,
  pagination, disabled favorite membership actions and clear confirmation.
- Image display is currently React `img` from hydrated data URLs. The shared
  drawing commands in `crates/ui/src/types.rs` currently cover shapes,
  text and clipping; image assets/decoding/cache and an image command still need
  a renderer contract. Load at most the page's thumbnail payloads; do not fetch
  full-size images just to draw a list or detail preview.
- `FavoriteMetadataEditor.tsx` has a two-row multiline note and a single-line
  comma-separated tag field. The existing shared `TextEdit` deliberately strips
  CR/LF for settings fields, so it cannot silently be reused for multiline note
  editing without adding an explicit multiline contract.
- Relative time and localized detail dates currently live in
  `src/utils/formatTime.ts` and the React detail views. Their clock/time-zone
  inputs must be explicit in a deterministic rendering snapshot.
- Desktop invalidations already exist as `HistoryChanged`, `FavoritesChanged`
  and `ScreenshotFavoritesChanged` in
  `crates/app/src/infrastructure/desktop_events.rs`. Subscribe while
  mounted, refresh membership for History on Favorites changes, and dispose
  subscriptions on page teardown.

Two retained behavior limitations are explicit. The React page does not clamp a
page after deleting its last item: `LibraryLayout.tsx` can display an invalid
range. The Rust workflow returns a resolved last valid page instead. Also,
regular favorite free-text search matches `content_json`/note; only screenshot
favorites include tags in free-text search. This is visible in both SQLite
source query and index SQL; the placeholder's mention of tags is broader than
the regular favorite implementation. This checkpoint preserves existing search
semantics and records that limitation rather than claiming universal tag search.

## Verification and limits

The SQLite index tests in
`crates/storage/src/database/library_index.rs` prove global ordering,
stable IDs, and page-before-hydration for an in-memory database. Source query
tests in `history.rs`, `favorites.rs`, and `screenshot_favorites.rs` prove ID
bounded queries and asset hydration. The Rust page module includes identity and
cross-source materialization tests. Six real SQLite workflow tests in
`crates/storage/src/database/library_pages_tests.rs` additionally verify
mixed History ordering, only-page thumbnail reads, lightweight visible-content
membership, a 120-screenshot deep page, colliding IDs across favorite sources,
metadata search/deletion routing, concurrent insert/delete after index query,
last-page deletion resolution and rerun-before-copy sequencing. The desktop settings host still needs a
native Library scene/host adapter; retaining the React Library view is expected
until that scene has visual and interaction parity on macOS and Windows.

The page workflow intentionally keeps screenshot history out of History and
routes screenshot favorites through the screenshot source. This matches the
existing React filters and avoids inventing a second source-specific history.
