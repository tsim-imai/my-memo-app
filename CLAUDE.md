# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

### Development
- `npm run dev` - Start Vite development server (frontend only)
- `cargo tauri dev` - Start full Tauri development mode (frontend + backend)
- `source $HOME/.cargo/env && cargo tauri dev` - If cargo/tauri not in PATH

### Building
- `npm run build` - Build frontend for production
- `cargo tauri build` - Build complete application bundle for distribution

### Project Structure
- Frontend: Vite + vanilla HTML/CSS/JavaScript
- Backend: Rust with Tauri framework
- Configuration: `src-tauri/tauri.conf.json`
- Dependencies: `package.json` (frontend), `src-tauri/Cargo.toml` (backend)

## Architecture Overview

This is a **completed** macOS-specific clipboard management application built with Tauri, targeting programmer workflows. All 7 development phases have been implemented.

### Core Components

**Frontend (main.js)**
- Modern tabbed interface with 4 main sections: History, Bookmarks, IP History, Settings
- Real-time communication with Rust backend via Tauri's `invoke()` API
- Complete keyboard navigation support (Cmd+1-4 for tabs, arrow keys, Cmd+F for search)
- Dark mode support with localStorage persistence
- Japanese UI text throughout
- Advanced features: preview modals, usage frequency tracking, search/sort functionality

**Backend (src-tauri/src/lib.rs)**
- `ClipboardManager` struct with `Arc<Mutex<AppData>>` for thread-safe state management
- Real-time clipboard monitoring (250ms intervals with adaptive error handling)
- Comprehensive Tauri command API (40+ commands) for all CRUD operations
- Advanced features: memory optimization, file-based logging, system diagnostics
- IP auto-detection with regex pattern `(?:[0-9]{1,3}\.){3}[0-9]{1,3}`

**Data Model**
```rust
AppData {
  version: String,
  history: Vec<ClipboardItem>,      // 50 items max with usage tracking
  bookmarks: Vec<BookmarkItem>,     // Permanent storage with tags
  recent_ips: Vec<IpHistoryItem>,   // 10 items max with access counting
  settings: AppSettings             // User preferences
}
```

Each item tracks `access_count` and `last_accessed` for intelligent sorting and cleanup.

### Implemented Features

**Core Functionality**
- Real-time clipboard monitoring with hash-based change detection
- Global hotkey (Cmd+Shift+V) with accessibility permission handling
- System tray integration with menu and Dock hiding
- JSON data persistence with atomic file operations and backup recovery

**Advanced UX**
- Complete keyboard navigation with shortcuts
- Smart content preview (JSON, URL, code detection)
- Usage frequency tracking and intelligent sorting
- Dark/light mode theming
- Search and filtering across all content types

**Performance & Reliability**
- Hash-based duplicate detection and memory optimization
- Adaptive monitoring intervals based on error states
- Comprehensive error handling with automatic recovery
- File-based logging with rotation and system diagnostics
- Automatic cleanup of large/old items

### macOS System Integration

**Required Permissions**
- Accessibility permission for global hotkeys and clipboard monitoring
- Automatic permission checking with user guidance

**System Features**
- Menu bar tray icon with context menu
- Dock icon visibility control
- Background operation without window focus
- Native macOS look and feel

## Key Dependencies

**Frontend**
- `@tauri-apps/api`: Frontend-backend communication
- Vite: Development server (port 1420)

**Backend**
- `tauri`: Core framework with tray-icon feature
- `clipboard`: macOS clipboard access
- `tauri-plugin-global-shortcut`: Global hotkey support
- `uuid`, `regex`, `chrono`, `tokio`: Core utilities
- `serde_json`: Data serialization

## Development Context

### Project Status
This is a **completed production application** with all planned features implemented across 7 development phases. The codebase represents enterprise-level quality with comprehensive error handling, performance optimization, and user experience features.

### Architecture Patterns
- **Event-driven**: Frontend listens to backend events for real-time updates
- **Thread-safe**: All shared state protected with `Arc<Mutex<>>`
- **Async/await**: Non-blocking operations throughout
- **Error propagation**: Comprehensive `Result<T, String>` error handling
- **Memory-efficient**: Hash-based operations and automatic cleanup

### UI Language
All user-facing text is in Japanese. Maintain this pattern for any new UI elements or error messages.

### Data Management
- JSON persistence in `$APP_DATA/clipboard_data.json`
- Automatic backups for corrupted files
- Log files in `$APP_DATA/clipboard_manager.log` with 5MB rotation
- Memory optimization with configurable cleanup thresholds