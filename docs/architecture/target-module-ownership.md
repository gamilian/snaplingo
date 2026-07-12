# Target Module Ownership

**Status:** Approved target; migration in progress

This document records the target ownership boundaries approved in the [Architecture Rebuild Design](../superpowers/specs/2026-07-11-architecture-rebuild-design.md). It does not claim that any implementation phase is complete. `ARCHITECTURE.md` remains a current-state overview that is updated as migrations land.

## Vocabulary

- A **module** is a cohesive body of behavior hidden behind an **interface**.
- An **interface** is the narrow surface callers use and tests target.
- **Depth** is the implementation complexity hidden behind an interface.
- A **seam** is a boundary where an implementation can be substituted.
- A **port** is an inward-owned interface for a capability supplied across a seam.
- An **adapter** implements an inward-owned interface using outward platform mechanics.
- **Leverage** means one interface serves multiple callers or tests.
- **Locality** means related state and decisions live in the same module.

The target favors deep modules: narrow interfaces should hide meaningful workflow or platform complexity. A seam is justified when it reverses an outward dependency or provides testing leverage, not when it only forwards a call.

## Ownership Boundaries

| Boundary | Owns | Must not own |
| --- | --- | --- |
| **Frontend View** | Rendering, view-local presentation state, and translation of user interaction into Frontend Application actions. | Workflow decisions, raw Tauri objects, `@tauri-apps` imports, event-name strings, or direct Frontend Platform access. |
| **Frontend Application** | Window workflow state, transitions, effect planning, and narrow interfaces consumed by Views and exercised by workflow tests. It owns the workflow-facing interface needed from the platform seam. | JSX rendering, raw Tauri mechanics, command or event names, or platform object lifetime. |
| **Frontend Platform** | Domain-named Tauri adapters that own command names, event names, payload parsing, window operations, Tauri object lifetime, and subscription cleanup. | Business workflow decisions or View state. |
| **Backend Command** | IPC parsing, AppState access, one Backend Application interface call, and final conversion of an Application error into an IPC error payload. | Workflow state, platform selection, direct Infrastructure orchestration, or duplicate compatibility behavior. |
| **Backend Application** | Domain workflows, state transitions, commit and rollback ordering, portable errors, and the inward ports each module needs. Ports stay beside their consuming module to preserve locality. | Infrastructure imports, platform configuration, Tauri/AppKit/Windows/GTK types, or concrete adapter construction. |
| **Composition** | Runtime assembly, concrete adapter selection, and injection across seams. The frontend composition root injects Frontend Platform adapters into Frontend Application modules; backend Application Composition is the sole selector of concrete OS adapters. | Business workflow decisions or reusable domain behavior. |
| **Infrastructure** | Outward adapters for OS integration, storage, events, HTTP, windows, OCR engines, and other external mechanics. Infrastructure translates platform failures at the seam and implements Backend Application-owned ports. | Backend Application workflow vocabulary, port ownership, or imports that make Infrastructure depend on Application implementations. |

The dependency direction follows ownership: Views consume Frontend Application interfaces; Frontend Platform adapters satisfy the platform seam; Backend Commands call Backend Application interfaces; Backend Application owns ports; Composition selects and injects Infrastructure adapters. This preserves locality inside workflow modules and gives their interfaces leverage across production callers and tests.

## Migration Inventory Is Not Architecture

The frontend and backend dependency allowlists added by the architecture foundation are frozen migration inventory. They record the current direct Tauri imports, raw event-string callers, and Backend Application-to-Infrastructure dependencies so automated checks can reject new leakage while later phases move each seam.

An allowlisted dependency is not accepted architecture, an exception to the target ownership rules, or permission to add a similar dependency. The inventories must shrink as their owning seams migrate and are removed when the strict target rule takes effect. Their presence proves only that the baseline is known and cannot grow.

## Accepted ADR Alignment

- [ADR 0004](../adr/0004-coordinator-consolidation.md) remains accepted. Translation and OCR Provider Coordinators remain deep modules that own activation, persistence, execution, and coordination; they are not split into shallow forwarding modules.
- [ADR 0005](../adr/0005-runtime-provider-reconfiguration.md) remains accepted for runtime Provider reconfiguration through the Coordinator interface. Its older single-`api_key` compatibility-command requirement is superseded and removed by [ADR 0006](../adr/0006-remove-provider-compatibility-command.md).
- [ADR 0006](../adr/0006-menu-bar-app-shell.md) remains accepted. SnapLingo keeps a menu-bar resident shell, with Settings opened explicitly and business workflows independent of the Settings Window lifecycle.
