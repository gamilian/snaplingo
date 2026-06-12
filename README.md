# SnapLingo

A cross-platform screenshot, OCR, and translation tool.

## Features

- 📸 **Screenshot Capture** - Snipaste-level editing tools (annotations, shapes, text, blur)
- 🔍 **OCR** - Extract text from images with multiple engine support
- 🌐 **Translation** - Multi-provider translation with side-by-side comparison
- ⌨️ **Global Hotkeys** - Quick access from anywhere
- 🎯 **Selection Translation** - Translate selected text instantly
- 🔒 **Privacy-First** - Local OCR options, BYOK (bring your own API key)

## Tech Stack

- **Framework**: Tauri 2.0
- **Frontend**: React + TypeScript + TailwindCSS
- **Backend**: Rust
- **Build Tool**: Vite

## Project Structure

```
snaplingo/
├── src/                    # React frontend
│   ├── components/         # UI components
│   ├── hooks/             # React hooks
│   └── utils/             # Frontend utilities
├── src-tauri/             # Rust backend
│   └── src/               # Rust source code
├── docs/                  # Documentation
│   └── adr/              # Architecture Decision Records
├── CONTEXT.md            # Domain language and design decisions
└── SnapLingo-PRD.md      # Product Requirements Document

```

## Development

### Prerequisites

- Node.js 18+
- Rust 1.70+
- Platform-specific dependencies:
  - **macOS**: Xcode Command Line Tools
  - **Linux**: `libgtk-3-dev`, `libwebkit2gtk-4.0-dev`, `libayatana-appindicator3-dev`
  - **Windows**: WebView2 Runtime

### Setup

```bash
# Install dependencies
npm install

# Start development server
npm run tauri dev

# Build for production
npm run tauri build
```

## Documentation

- **[CONTEXT.md](./CONTEXT.md)** - Core concepts and domain language
- **[SnapLingo-PRD.md](./SnapLingo-PRD.md)** - Product requirements and specifications
- **[docs/adr/](./docs/adr/)** - Architecture decisions

## License

TBD (MIT or Apache 2.0)

## Contributing

Contributions welcome! Please read our contributing guide (coming soon).
