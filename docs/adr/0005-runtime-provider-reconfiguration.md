# ADR 0005: Runtime Provider Reconfiguration

## Status
Accepted

## Context

Provider configuration commands save credentials while the Settings Window expects the Provider to be usable immediately. A restart-only configuration path makes the command seam shallow: the UI would need to know that saving credentials is not enough, and users would see stale runtime Provider state after a successful save.

## Decision

Provider configuration must update the runtime Provider instance when the Provider is already registered.

Provider Coordinators own this behavior through their public Interface:
- `TranslationCoordinator::reconfigure_provider(...)`
- `OcrCoordinator::reconfigure_provider(...)`

Credential validation and custom Translation Provider definition handling live in the Provider Configuration Module. Credentials still persist through Keychain, but runtime mutation happens through the registered Provider behind the Coordinator seam.

## Consequences

- Provider Coordinators keep Provider instances behind interior mutability so credentials can change without rebuilding AppState.
- Configuration tests target the Coordinator Interface rather than command internals.
- Commands stay thin: save credentials, ask the Coordinator to reconfigure, and return the result.
- Providers that do not support runtime credential changes must make that explicit through the Provider Interface instead of relying on restart-only behavior.
