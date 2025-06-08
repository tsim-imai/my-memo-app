# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

### Development
- `source $HOME/.cargo/env && cargo tauri dev` - Start full Tauri development mode (required for Rust environment)
- `npm run dev` - Start Vite development server (frontend only, for UI testing)

### Building  
- `npm run build` - Build frontend for production
- `cargo tauri build` - Build complete application bundle for distribution

### Troubleshooting
- If `cargo tauri dev` fails with "command not found", run `source $HOME/.cargo/env` first
- Port 1420 conflicts: Kill existing processes with `lsof -ti:1420 | xargs kill -9`
- Compilation errors: Check for Unicode string syntax in Rust (use raw strings, not escape sequences)

### Project Structure
- **Frontend**: Single-page app with `index.html`, `main.js`, vanilla CSS
- **Backend**: Modular Rust architecture in `src-tauri/src/`
  - `lib.rs` - Core application logic and ClipboardManager
  - `models.rs` - Data structures (AppData, ClipboardItem, BookmarkItem, etc.)
  - `file_manager.rs` - JSON persistence and file operations
  - `clipboard_monitor.rs` - Real-time clipboard monitoring
  - `window_manager.rs` - macOS window positioning and multi-display handling
  - `commands/` - Tauri command modules organized by functionality
- **Configuration**: `src-tauri/tauri.conf.json` (Tauri settings), `src-tauri/Cargo.toml` (Rust deps)

## Architecture Overview

This is a **completed** macOS-specific clipboard management application built with Tauri, targeting programmer workflows. All 7 development phases have been implemented.

### Core Components

**Frontend Architecture (main.js + index.html)**
- **Native macOS Design**: Fullscreen layout without app header (uses system title bar)
- **Tabbed Interface**: 4 main sections with simplified panel headers removed
- **Event-Driven**: Real-time updates via Tauri's `invoke()` API and `listen()` events
- **Keyboard-First**: Complete navigation (Cmd+1-4 tabs, arrows, Cmd+F search, Delete actions)
- **Responsive Flexbox**: Full viewport utilization with proper scroll handling
- **Enhanced Functions**: Function wrapping pattern in `enhanceDisplayFunctions()` for UI updates
- **Japanese UI**: All user-facing text in Japanese, console logging for status updates

**Backend Architecture (Modular Rust)**
- **Core (`lib.rs`)**: `ClipboardManager` with `Arc<Mutex<AppData>>` for thread-safe state management
- **Models (`models.rs`)**: Centralized data structures with serde serialization and Default implementations
- **File Manager (`file_manager.rs`)**: Atomic file operations, JSON persistence, log rotation, and backup recovery
- **Clipboard Monitor (`clipboard_monitor.rs`)**: 250ms polling with hash-based change detection and IP auto-detection
- **Window Manager (`window_manager.rs`)**: macOS-specific window positioning, multi-display support, and coordinate system handling
- **Commands (`commands/`)**: 40+ Tauri commands organized by functionality:
  - `clipboard_commands.rs` - History management operations
  - `bookmark_commands.rs` - Bookmark CRUD and search
  - `ip_commands.rs` - IP detection and history management  
  - `settings_commands.rs` - Configuration management
  - `app_commands.rs` - System-level operations and diagnostics

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
- **Streamlined Operations**: No confirmation dialogs for delete/clear actions (instant feedback)
- **Smart Content Preview**: JSON, URL, code detection with modal display
- **Usage Frequency Tracking**: `access_count` and `last_accessed` for intelligent sorting
- **Adaptive Dark Mode**: System-aware theming with localStorage persistence
- **Real-Time Search**: Debounced search across all content types with live filtering

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

**Multi-Display Support**
- Smart window positioning at mouse cursor location across multiple displays
- Automatic coordinate system detection and scaling (4K vs FullHD)
- Logical/Physical coordinate conversion to prevent double-scaling issues
- Dynamic window focus management for cross-display hotkey activation
- AppleScript-based window focus automation for improved reliability

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
- **Modular Design**: Separated concerns into focused modules (models, file_manager, clipboard_monitor, window_manager, commands)
- **Event-driven**: Frontend listens to backend events for real-time updates
- **Thread-safe**: All shared state protected with `Arc<Mutex<>>`
- **Async/await**: Non-blocking operations throughout
- **Error propagation**: Comprehensive `Result<T, String>` error handling
- **Memory-efficient**: Hash-based operations and automatic cleanup
- **Cross-module communication**: Clean interfaces between modules with minimal coupling

## Development Patterns & Conventions

### Error Handling
- **Rust**: All Tauri commands return `Result<T, String>` for consistent error propagation
- **JavaScript**: Try-catch blocks with `updateStatus()` calls for user feedback (now logs to console)
- **Graceful Degradation**: App continues functioning even if individual features fail (e.g., hotkeys, clipboard monitoring)

### State Management
- **Centralized State**: Single `AppData` struct containing all application data
- **Thread Safety**: All shared state wrapped in `Arc<Mutex<>>` for safe concurrent access
- **Atomic Operations**: File writes use temporary files + rename for data integrity

### UI Patterns
- **Global Window Functions**: UI event handlers exposed as `window.functionName` for onclick attributes
- **Function Enhancement**: `enhanceDisplayFunctions()` pattern for extending existing functions
- **Flex Layout**: All containers use flexbox with `flex: 1` and `overflow-y: auto` for proper scrolling
- **No Confirmation Dialogs**: Immediate actions with status feedback for better UX

### Code Conventions
- **Japanese UI Text**: All user-facing strings must be in Japanese
- **Console Logging**: Use `console.log()` for debugging and status updates (replaces removed status header)
- **Unicode Strings**: Use raw Japanese strings in JavaScript, avoid Unicode escape sequences in Rust
- **Consistent Naming**: `updateStatus()`, `loadData()`, `displayItems()` patterns throughout

### Data Management
- **JSON Persistence**: `$APP_DATA/clipboard_data.json` with atomic writes and backup recovery
- **Log Rotation**: `$APP_DATA/clipboard_manager.log` with 5MB size limit and `.old` rotation
- **Memory Limits**: Configurable cleanup thresholds for history (50 items) and IPs (10 items)
- **Hash-Based Deduplication**: Content hashing for efficient duplicate detection and memory optimization

### Module Development Guidelines
- **Models**: Add new data structures to `models.rs` with serde traits and Default implementations
- **File Operations**: Extend `FileManager` for new persistence requirements with atomic operations
- **Monitoring**: Enhance `ClipboardMonitor` for new content detection patterns
- **Window Management**: Modify `WindowManager` for display-specific behaviors and coordinate handling
- **Commands**: Add new Tauri commands to appropriate modules in `commands/` directory
- **Cross-module Communication**: Use shared `AppData` state and clear interface contracts between modules