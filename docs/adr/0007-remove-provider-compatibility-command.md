# ADR 0007: Remove the Provider compatibility command

## Status

Accepted — 2026-07-12

## Context

ADR 0005 retained an older single-`api_key` Provider configuration command as a temporary compatibility adapter. SnapLingo has not shipped, and the architecture rebuild establishes typed credential-map configuration as the only supported contract.

## Decision

Remove the single-key command and its frontend adapter. Provider credentials are configured only through the credential-map command and Application Provider Configuration module.

## Consequences

- IPC clients must use the credential-map contract.
- No compatibility conversion, persisted-data migration, or dual command path is maintained.
- This supersedes the compatibility-command portion of ADR 0005; its runtime Provider reconfiguration decision remains in effect.
