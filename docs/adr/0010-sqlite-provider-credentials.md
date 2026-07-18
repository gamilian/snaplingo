# ADR 0010: SQLite Provider Credentials

## Status

Accepted (2026-07-18)

## Context

The macOS beta initially used platform Keychain adapters for Provider endpoints and credentials. That introduced authorization prompts unrelated to SnapLingo's required screen-recording and Accessibility onboarding, and made one logical Provider configuration span two persistence systems.

The beta distribution explicitly accepts API keys and secret keys being stored unencrypted in the local application database.

## Decision

All Provider credentials use the existing `ProviderCredentialStore` Application port and one Infrastructure implementation: `SqliteCredentialStore`.

- Provider definitions (including Endpoint and Base URL) remain in versioned SQLite config namespaces; API key and secret-key fields are stored in the `provider_credentials` table.
- Multi-field writes, restores, and deletes use one SQLite transaction.
- The application-data directory and database file are restricted to the current user on Unix platforms.
- Production code has no platform Keychain adapter or `keyring` dependency.
- Legacy `provider:{id}:api_key` SQLite rows are normalized to `provider:{id}:credential:api_key` by schema migration v6.

## Consequences

- Provider definitions, settings, and credentials share one database lifecycle and backup boundary.
- macOS runtime credential access no longer produces Keychain password prompts.
- Credentials are not encrypted at rest beyond operating-system filesystem protections.
- A future encrypted store requires a new ADR and a migration behind the same Application port.

## Supersedes

- ADR 0003's system-Keychain storage decision.
- ADR 0005's statement that runtime Provider credentials persist through Keychain.
