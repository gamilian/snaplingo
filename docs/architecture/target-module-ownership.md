# Module ownership

This is the current ownership map for the seven-package native workspace.
[ARCHITECTURE.md](../../ARCHITECTURE.md) contains the package graph and directory
map. [The active refactor plan](../plans/native-workspace-implementation.md) records
implementation and verification status.

A **module** is a cohesive implementation behind an **interface**. Its interface
includes types, ordering, lifetime and error behavior. **Depth** means callers
exercise useful behavior without knowing implementation details. A **seam** is
where an implementation can be replaced; an **adapter** supplies that behavior.
Ports belong beside their consuming workflow so changes remain local.

| Module | Interface and ownership |
| --- | --- |
| `core/capture` | Frozen capture session, selection/editor transitions, output ordering and consumed capture/output ports |
| `core/providers` | OCR/translation execution, provider administration, configuration/credential ordering and event publication |
| `core/library` | History, Favorites, shared capacity and lightweight ordered index; repository contracts remain inward-owned |
| Other Core features | Settings/hotkeys, selected-text policy, result requests, pins, permission and speech policy |
| `ui/<feature>` | Presentation snapshot, input/state transition, Scene or native-control geometry; no Core or OS types |
| `ui/foundation` | Reusable geometry, raster values, normalized input, text editing and semantic theme |
| `ui/render`, `ui/accessibility` | Optional rasterization and semantic projection of UI data |
| `platform/<capability>` | Real system implementations; Mac/Windows/Linux variants sit by capability |
| `providers` | HTTP transport, external translation/LLM/OCR protocols and optional local inference implementing Core contracts |
| `storage` | SQLite migrations/repositories and filesystem assets implementing Core ports |
| `app/src/features/<feature>` | Shared presenter/controller and feature-specific dependencies; native adapters own OS window mechanics |
| `app/src/bootstrap` | Whole-application state, concrete construction and feature injection |
| `app/src/shell` | Resident lifecycle, event routing, tray and application entry actions |
| `app/src/integrations` | Product updates, events/runtime integrations and native window helpers |

Feature models live beside their workflows in Core. Model files cannot import
workflow implementations or concrete adapters. Workflow modules may depend on
other feature interfaces when their policy requires it; a cross-feature call does
not justify another forwarding layer. Callers name the owning module explicitly,
for example `snaplingo_core::capture` or `snaplingo_core::library::history`.

The application has an acyclic assembly direction: bootstrap constructs feature
dependencies, then native hosts consume them. Feature code cannot import AppState,
bootstrap or the resident shell. A platform effect that needs application behavior
receives a callback. Global shortcut recording, for example, receives the host's
UI dispatcher and focus query; Platform does not reach into a Settings window.

Core and UI remain independent of concrete system adapters. Platform, Providers and Storage
cannot import the desktop application. Their implementations are imported directly
from their owning crates, without an application `system` or `storage` alias.
`lib.rs` files expose their own interfaces; they do not re-export Core's old
`domain`/`application` tree or a complete infrastructure module tree.

Shared UI does not require duplicating widgets on each platform. Native adapters
consume the same logical layout/state while retaining text services, accessibility,
menus and native presentation. Product presenters translate Core snapshots into
UI data. This keeps product workflow vocabulary out of reusable drawing code.

Foundation owns the pure drawing Scene and Appearance. Settings owns its page
metadata in SettingsFrame; independent windows supply their own input/IME and
semantic data. Renderer cannot import product pages or their root aliases.
Historical AppKit pages are explicit test_support modules, not production hosts.

The retained browser tree is reference/test code. Its View/Application rules
continue to protect independent behavior baselines, but there is no production
frontend platform, Tauri command registry or browser IPC seam in this workspace.

The workspace and desktop architecture tests enforce these directions, including
all target-specific Cargo edges. Behavioral tests remain at workflow and native
host interfaces; moving files or passing a syntax guard alone does not establish
feature equivalence.
