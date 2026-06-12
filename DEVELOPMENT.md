# Development Setup

## Prerequisites

- Node.js 18+
- Rust 1.77 - 1.82 (⚠️ **Important**: Rust 1.83+ has a known issue with `time` crate)
- Platform-specific dependencies:
  - **macOS**: Xcode Command Line Tools
  - **Linux**: `libgtk-3-dev`, `libwebkit2gtk-4.0-dev`, `libayatana-appindicator3-dev`
  - **Windows**: WebView2 Runtime

## Known Issues

### Rust 1.83+ Compilation Error

**Problem**: Rust 1.83 and later versions have breaking changes that cause conflicts with the `time` crate used by Tauri dependencies.

**Error message**:
```
error[E0119]: conflicting implementations of trait `From<...>` for type `<...>`
```

**Solutions**:
1. **Use Rust 1.82 or earlier** (Recommended):
   ```bash
   rustup install 1.82.0
   rustup default 1.82.0
   ```

2. **Wait for Tauri upstream fix**: Track https://github.com/tauri-apps/tauri/issues

## Installation

```bash
# Install Node dependencies
npm install

# Build frontend
npm run build

# Check Rust code
cargo check

# Run in development mode
npm run tauri:dev
```

## Project Structure

```
snaplingo/
├── src/                      # React frontend
│   ├── components/           # UI components
│   ├── hooks/               # React hooks
│   ├── types/               # TypeScript types
│   ├── utils/               # Frontend utilities
│   ├── styles/              # Global styles
│   ├── App.tsx              # Main app component
│   └── main.tsx             # Entry point
├── src-tauri/               # Rust backend
│   ├── src/
│   │   ├── commands.rs      # Tauri commands
│   │   ├── config.rs        # Configuration management
│   │   ├── providers/       # OCR and translation providers
│   │   ├── capture.rs       # Screenshot capture
│   │   ├── history.rs       # History management
│   │   ├── utils.rs         # Utility functions
│   │   ├── lib.rs           # Library entry point
│   │   └── main.rs          # Binary entry point
│   ├── Cargo.toml           # Rust dependencies
│   └── tauri.conf.json      # Tauri configuration
├── docs/                    # Documentation
│   └── adr/                # Architecture Decision Records
├── CONTEXT.md              # Domain language
├── SnapLingo-PRD.md        # English PRD
├── SnapLingo-PRD-CN.md     # Chinese PRD
├── README.md               # Project overview
└── package.json            # Node dependencies
```

## Next Steps

1. Fix Rust version compatibility
2. Implement screenshot capture (platform-specific)
3. Add OCR providers (starting with Tesseract)
4. Build translation system (starting with Google Translate)
5. Create Result Window UI component
6. Implement global hotkeys
7. Add system tray integration
