# Native General preferences and application actions

The macOS Settings window now mounts its General page through Rust/AppKit. The retained React navigation supplies the page rectangle, interface scale and section route; the native page owns its controls, scrolling, settings edits, permissions, logs, history maintenance, software updates and third-party notices. Other Settings tabs still use their existing renderer. This is a production migration boundary, not a standalone mockup or a completed removal of Tauri.

## Ownership and behavior

`GeneralController` receives Settings, History, RequiredPermissions and AppUpdates services directly. It handles update-check/download state, duplicate operation suppression, failure reporting and invalidation of late network results. Permission requests, repair and restart share a busy guard. Settings field edits hold the existing configuration lock while reading and saving the latest snapshot; startup registration rolls back if storage fails. OCR and Translation have equivalent typed field-edit APIs ready for their remaining native pages.

`GeneralPage` owns the controller and a disposable desktop notification subscription. Worker completions queue main-thread refreshes that read current state when delivered. Structural updates wait for the active AppKit field editor to finish, preserving its draft and selection. Disposing a page explicitly removes subscriptions and invalidates pending work even if AppKit temporarily retains an old view. Native views retain only weak controller references.

`native_general_host` mounts the page alongside the Settings WKWebView and keeps the existing navigation clickable. Requests carry an owner token; old cleanup cannot remove a newer page. Frontend mount/update/disposal requests are serialized across React remounts. The host applies the actual DOM rectangle and scale, and refreshes permissions when the window regains focus. Closing Settings commits its field editor and closes its native notice dialog.

Retained permission, capacity and embedded-result dialogs temporarily conceal the native controls and show their captured PNG beneath the HTML backdrop. This keeps both the prior background and the dialog's hit testing while preserving the live native page for restoration. The preview is generated only when entering that concealed state and is released on reveal/disposal. Native General is not recreated when a retained dialog closes.

The application state also owns `AppActionDispatcher`. Menu and global-shortcut events reach Rust actions independently of `TauriDesktopBridge`; multiple notification bridges cannot duplicate actions. The bridge still supplies retained frontend notifications, Settings window routes, exit/restart and remaining IPC.

## UI fidelity

General uses the original shared Chinese text, semantic theme tokens and all six section routes. The reference harness renders the actual React page in WebKit with isolated data; it verifies the requested document/theme before taking a snapshot. Reference and native content sizes account for the original Native sidebar, Studio rail and Paper top navigation. Fixtures cover every section, permission recovery, update notes, log content, and native notice list/detail views.

Functional tests do not establish complete visual parity. Native snapshots and actual React references remain under `target/native-settings`; outstanding layout or state differences must be resolved against those renderings. Native Windows/Linux Settings, the remaining macOS pages, the permission guide and embedded result surface still require migration. The application event loop, IPC/build shell, npm API and packaging continue to depend on Tauri.

The final checkpoint compares three baseline themes and all six sections plus recovery/update/log scenarios. Baseline card/theme-option frames match their React reference within 0.011 points; the 28-point Paper offset was caused by automatic AppKit scroll-view content insets and is fixed explicitly. Remaining observed General differences include a recovery section about 32 points shorter, update download content about 12 points earlier, and some glyph-weight/miniature-preview details. The contact sheets are `target/native-settings/comparison-baseline.png` and `comparison-scenes.png`. These are remaining acceptance work, not evidence of complete visual equality.

## Verification

The standalone `settings_general_native` harness uses NSApplication and actual AppKit controls with an isolated database. It checks settings persistence, startup rollback, invalid inputs, automatic update delivery, worker notifications, active field-editor preservation, manual scrolling, disposal, native host scaling/owner cleanup and notice search/detail/close/resize behavior without initializing Tauri.

The production Settings workflow mounts the real compiled React navigation, invokes the actual native presentation command and inspects its AppKit/WKWebView hierarchy. It checks all navigation labels, hit testing, theme changes, resize, native routes, tab unmount/remount and persistence. The actual capacity dialog decodes the native PNG underlay at the placeholder bounds, receives hit testing, and restores the same native control instances after closing. About navigation is checked by its visible section and clamped scroll position, accounting for the previous renderer's section-threshold behavior near the document bottom.

The full native workflow passes together with Settings: Carbon/menu routing, all six compatibility events, result-window IME/OCR/favorites/speech, capture editor tools/output/recovery, scrolling and pinned-image cleanup. This checkpoint also passes 748 Rust unit tests (two existing ignored), ten architecture checks, three infrastructure integration tests, 1,037 frontend tests (one skipped), production build, formatting and all-target compilation. These results do not finish the remaining page, shell, platform or visual acceptance work described above.
