# Infrastructure Layer

The infrastructure layer provides platform-agnostic abstractions for external system interactions. All production code depends on traits; platform-specific implementations live in the `adapters/` directory.

## Architecture

```
infrastructure/
├── storage/           # Persistent data storage
│   ├── traits.rs      # ConfigStore, SecretStore traits
│   └── adapters/      # Platform implementations
│       ├── config_file.rs
│       └── keychain.rs
├── http/              # HTTP client abstraction
│   ├── traits.rs      # HttpClient trait
│   └── adapters/
│       └── reqwest_client.rs
├── system/            # System integration (hotkeys, clipboard)
│   ├── traits.rs      # HotkeyManager, ClipboardManager traits
│   └── adapters/      # Platform implementations (future)
└── mod.rs
```

## Design Principles

1. **Dependency Inversion**: Business logic depends on traits, not concrete implementations
2. **Platform Adaptation**: Platform-specific code isolated in `adapters/`
3. **Testability**: All traits have mock implementations for testing
4. **Async-First**: All I/O operations are async

## Components

### Storage

**ConfigStore** - Application configuration persistence
- File-based storage (JSON)
- Atomic writes via temp file + rename
- Directory creation on demand

```rust
use snaplingo_lib::infrastructure::storage::{ConfigStore, ConfigFile};

let store = ConfigFile::new("config.json");
store.save(&config).await?;
let loaded = store.load::<AppConfig>().await?;
```

**SecretStore** - Secure credential storage
- macOS: Keychain Services
- Windows: Credential Manager (future)
- Linux: Secret Service API (future)

```rust
use snaplingo_lib::infrastructure::storage::{SecretStore, Keychain};

let store = Keychain::new("com.snaplingo.app");
store.save_secret("api_key", "sk-xxx").await?;
let key = store.load_secret("api_key").await?;
```

### HTTP

**HttpClient** - HTTP request abstraction
- Wraps `reqwest` with custom error handling
- JSON request/response convenience methods
- Timeout and retry logic (future)

```rust
use snaplingo_lib::infrastructure::http::{HttpClient, ReqwestClient};

let client = ReqwestClient::new();
let response: ApiResponse = client.post_json(url, &request).await?;
```

### System (Future)

**HotkeyManager** - Global hotkey registration
- Cross-platform hotkey handling
- Conflict detection
- Event delivery to application

**ClipboardManager** - Clipboard access
- Read/write text content
- Format detection (future)

## Testing Strategy

### Unit Tests
- Mock implementations for all traits (`MockConfigStore`, `MockSecretStore`, `MockHttpClient`)
- Test business logic in isolation
- Fast, deterministic, no I/O

### Integration Tests
- Real adapter implementations against test fixtures
- File system interactions use temp directories
- HTTP tests use mock servers (future)

### Running Tests

```bash
# All tests
cargo test

# Infrastructure layer only
cargo test --lib infrastructure

# Specific component
cargo test --lib infrastructure::storage
cargo test --lib infrastructure::http
```

## Platform Support

| Component | macOS | Windows | Linux |
|-----------|-------|---------|-------|
| ConfigFile | ✅ | ✅ | ✅ |
| Keychain | ✅ | 🚧 | 🚧 |
| HttpClient | ✅ | ✅ | ✅ |
| HotkeyManager | 🚧 | 🚧 | 🚧 |
| ClipboardManager | 🚧 | 🚧 | 🚧 |

✅ Implemented | 🚧 Planned

## Error Handling

All infrastructure operations return `InfrastructureError`:

```rust
pub enum InfrastructureError {
    IoError(String),
    SerializationError(String),
    NetworkError(String),
    NotFound(String),
    PermissionDenied(String),
}
```

Errors are propagated up to the application layer for user-facing messages.

## Future Enhancements

- [ ] Windows Credential Manager for `SecretStore`
- [ ] Linux Secret Service API for `SecretStore`
- [ ] HTTP client retry logic with exponential backoff
- [ ] Request timeout configuration
- [ ] Global hotkey management
- [ ] Clipboard access
- [ ] File system watcher for config changes
- [ ] Logging integration

## Dependencies

- `serde` - Serialization framework
- `serde_json` - JSON format
- `reqwest` - HTTP client
- `tokio` - Async runtime
- `security-framework` (macOS) - Keychain access

## Usage in Application

The infrastructure layer is initialized at startup and injected into the application:

```rust
// src-tauri/src/main.rs
use snaplingo_lib::infrastructure::{
    storage::{ConfigFile, Keychain},
    http::ReqwestClient,
};

#[tokio::main]
async fn main() {
    let config_store = ConfigFile::new("config.json");
    let secret_store = Keychain::new("com.snaplingo.app");
    let http_client = ReqwestClient::new();
    
    // Inject into application layer
    let app = App::new(config_store, secret_store, http_client);
    app.run().await;
}
```

For testing, use mock implementations:

```rust
#[cfg(test)]
mod tests {
    use snaplingo_lib::infrastructure::storage::MockConfigStore;
    
    #[tokio::test]
    async fn test_app_logic() {
        let config_store = MockConfigStore::new();
        let app = App::new(config_store, /* ... */);
        // Test without file I/O
    }
}
```

---

**Status**: Phase 1 Infrastructure Layer - COMPLETE ✅
