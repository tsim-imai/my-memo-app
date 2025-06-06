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

This is a macOS-specific clipboard management application built with Tauri, targeting programmer workflows.

### Core Components

**Frontend (main.js)**
- Communicates with Rust backend via Tauri's `invoke()` API
- Displays clipboard history with periodic refresh (2-second intervals)
- Uses Japanese UI text throughout

**Backend (src-tauri/src/lib.rs)**
- `ClipboardManager` struct manages application state with `Mutex<Vec<ClipboardItem>>`
- Tauri commands: `init_clipboard_manager()`, `get_clipboard_history()`
- Planned features: clipboard monitoring, IP detection, bookmark system

**Data Model (from TODO.md)**
```
ClipboardItem { id, content, content_type, timestamp, size }
- History: 50 items max
- Bookmarks: permanent storage with tags
- IP tracking: auto-detection of xxx.xxx.xxx.xxx patterns (10 items max)
```

### Planned Features
1. **Clipboard monitoring**: Real-time NSPasteboard watching
2. **Global hotkey**: Cmd+Shift+V activation
3. **System integration**: Menu bar presence, Dock hiding
4. **Data persistence**: JSON file storage in $APPDATA
5. **IP auto-detection**: Regex pattern `(?:[0-9]{1,3}\.){3}[0-9]{1,3}`

### Development Focus
- Target: Personal use by programmer (not for distribution)
- Platform: macOS only
- UI Language: Japanese
- Data format: JSON persistence
- Access patterns: Frequent clipboard access, token/credential storage

## Key Dependencies
- `@tauri-apps/api`: Frontend-backend communication
- `uuid`, `regex`, `chrono`: Backend utilities
- `tauri-plugin-fs`, `tauri-plugin-log`: File operations and logging
- Vite: Frontend development server (port 1420)

## Development Workflow

### TODO Management
- **TODO tracking**: All tasks are managed in `TODO.md` with checkboxes
- **Progress updates**: Mark completed tasks with `[x]` when finished
- **Commit strategy**: Each functional unit/feature gets its own commit
- **TODO updates**: Update `TODO.md` and commit it along with the implementation

### Git Commit Pattern
1. Implement a feature or complete a TODO item
2. Update `TODO.md` to mark the item as completed `[x]`
3. Stage both the implementation and the updated `TODO.md`
4. Commit with descriptive message explaining what was accomplished
5. Continue to next TODO item

This ensures clear development progress tracking and granular version history.