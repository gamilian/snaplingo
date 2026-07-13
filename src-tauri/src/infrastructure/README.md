# Infrastructure Layer

The infrastructure layer contains SnapLingo's adapters for persistence, HTTP, and operating-system integration. Application services depend on ports defined in the application layer; composition creates the concrete adapters once at startup.

## Structure

```text
infrastructure/
├── storage/
│   ├── database/       # Shared SQLite connection, migrations, config and history repositories
│   └── keychain/       # Provider credentials
├── http/               # Reqwest transport adapter
├── llm/                # Provider protocol clients
├── events/             # In-process domain event bus
└── system/             # Clipboard, capture, OCR, shortcuts and window adapters
```

## Persistence

All non-secret durable state uses one database in the platform application-data directory:

```text
snaplingo/
└── snaplingo.db
```

`Database` owns the shared `rusqlite::Connection`, guarded by a mutex. Opening it enables foreign keys, WAL mode, normal synchronous writes, and a five-second busy timeout, then applies sequential migrations through `PRAGMA user_version`.

The current repositories are:

- `SqliteConfigStore`: versioned JSON namespaces for settings, hotkeys, Provider definitions/order, and prompt strategies.
- `SqliteHistoryRepository`: relational translation/OCR history with one global history ID plus favorites, notes, and tags.
- `Keychain`: API keys, tokens, and other Provider credentials. Secrets must never be added to SQLite payloads.

The database is created once in `composition.rs` and shared by the repositories. Tests use `Database::in_memory()` or a temporary database instead of production paths.

## Storage Rules

- Rust domain types define defaults; SQL migrations do not insert default configuration rows.
- A missing configuration namespace is handled by the owning application service.
- SQLite schema changes require a new ordered migration and a `user_version` increment.
- A database with a newer schema version is rejected instead of being rebuilt.
- Images and other large assets belong in the filesystem; SQLite stores only metadata when a durable asset feature is introduced.
- Provider secrets stay in the system Keychain and use stable Provider IDs.

## Testing

Run the infrastructure and full backend suites from the repository root:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib infrastructure
cargo test --manifest-path src-tauri/Cargo.toml --test infrastructure_integration_test
cargo test --manifest-path src-tauri/Cargo.toml
```

Repository tests should verify restart persistence, foreign-key behavior, transaction rollback, and that secret values do not appear in SQLite.

## Dependency Direction

Production application code must not import infrastructure implementations. Concrete adapters are assembled in the composition layer and injected behind application ports. The architecture dependency test enforces this boundary.
