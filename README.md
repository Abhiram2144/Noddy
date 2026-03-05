# 🤖 Noddy - AI Desktop Assistant

A modern AI desktop assistant built with Tauri (Rust), React (TypeScript), and Python. Noddy provides intelligent command parsing, memory management, reminders, and application control with a beautiful dark-themed Control Center UI.

**Version:** 0.5 | **Status:** Beta | **Last Updated:** March 2026

---

## 📋 Table of Contents

1. [Overview](#overview)
2. [Key Features](#key-features)
3. [Architecture](#architecture)
4. [Technology Stack](#technology-stack)
5. [Installation](#installation)
6. [Quick Start](#quick-start)
7. [Feature Implementation Details](#feature-implementation-details)
8. [API Reference](#api-reference)
9. [Project Structure](#project-structure)
10. [Development](#development)
11. [Future Roadmap](#future-roadmap)

---

## 🎯 Overview

Noddy is a desktop assistant that bridges the gap between users and their applications through intelligent command interpretation, persistent memory storage, and scheduled reminders. The assistant operates on three layers:

- **Frontend**: Modern React 19 Control Center UI with dark theme and Framer Motion animations
- **Backend**: Tauri (Rust) with event-driven architecture, SQLite persistence, and command execution
- **Intelligence**: Python FastAPI server for deterministic intent parsing and natural language interpretation

### Design Philosophy

- **Deterministic**: Rule-based parsing with predictable outcomes
- **Privacy-First**: All data stored locally (SQLite)
- **Modular**: Event-driven architecture with decoupled components
- **Testable**: Pure functions and comprehensive error handling
- **User-Friendly**: Natural language command support with intuitive UI

---

## ✨ Key Features

### 1. **Memory Management** 💾
Store and recall information persistently across sessions.
- Save memories with `"remember my project deadline is Friday"`
- Recall all memories with `"recall"`
- Search memories with `"search python"`
- Automatic SQLite persistence
- Memory timestamps with relative formatting ("3 days ago", "2 hours ago")

**Status:** ✅ Fully Implemented (Phase 3d)

### 2. **Reminder System** ⏰
Set reminders with natural language support and countdown tracking.
- Basic: `"remind me to call mom"` (defaults to 1 hour)
- With duration: `"remind me to code in 2 hours"`
- With specific time: `"remind me to review code tomorrow at 3pm"`
- All time units supported: minutes, hours, days
- Future reminder filtering and countdown display
- Local storage with expiration tracking

**Status:** ✅ Fully Implemented (Phase 3d)

### 3. **Application Control** 🖥️
Open, manage, and control applications installed on your system.
- Open apps: `"open chrome"`, `"open vs code"`
- Open URLs: `"open https://github.com"`
- Web shortcuts: `"open youtube in web"`
- Kill processes: `"kill notepad"`, `"kill chrome"`
- List all available apps: `"list apps"`
- Windows app discovery via registry scanning

**Status:** ✅ Fully Implemented

### 4. **Web Search Integration** 🔍
Quick web searches directly from the assistant.
- Google search: `"google machine learning"`
- Web queries: `"search web python documentation"`
- Natural language: `"what is machine learning"`, `"what's AI"`

**Status:** ✅ Fully Implemented

### 5. **Intent Recognition** 🧠
Converts natural language commands to structured intents.
- 8 intent types: remember, recall_memory, search_memory, set_reminder, search_web, open_app, open_url, kill_process, list_apps
- Deterministic parsing (no LLMs in core engine)
- Extensible intent system with payload support
- Error handling for unknown intents

**Status:** ✅ Fully Implemented

### 6. **Event-Driven Architecture** 📡
Decoupled component communication through event publishing.
- Internal EventBus for pub/sub
- Event types: IntentReceived, IntentExecuted, MemorySaved, ReminderScheduled, ReminderTriggered, ErrorOccurred
- Extensible for telemetry and logging
- Non-blocking event emission

**Status:** ✅ Fully Implemented (Phase 2)

### 7. **Permission Management** 🔐
Capability-based access control for safe command execution.
- Permission types: execute_commands, read_memory, write_memory, manage_reminders, kill_process, list_apps
- Role-based access control (planning)
- Permission checks on all sensitive operations
- Audit logging for security events

**Status:** ✅ Partially Implemented (Phase 2) | 🔄 Audit Logging TODO

### 8. **Telemetry & Analytics** 📊
Track system events and performance metrics.
- Command execution tracking
- Performance metrics (execution duration)
- Event frequency counters
- Memory and reminder statistics
- JSON-formatted telemetry output

**Status:** ✅ Partially Implemented (Phase 2) | 🔄 Full Analytics Dashboard TODO

### 9. **Scheduler** ⏱️
Scheduled task execution and reminder triggering.
- Check for expired reminders
- Trigger reminder notifications
- Scheduled task queue
- Background polling support

**Status:** ✅ Partially Implemented (Phase 2) | 🔄 Background Process TODO

### 10. **Control Center Dashboard** 🎨
Modern, responsive UI for interacting with all features.
- 7 integrated views: Dashboard, Reminders, History, Memory, Test Commands, Integrations, Settings
- Dark theme with CSS variables
- Framer Motion animations (fade, slide, stagger, hover effects)
- Responsive 2-column grid layout
- Real-time data fetching from backend
- Command execution testing interface

**Status:** ✅ Fully Implemented (Phase 3)

---

## 🏗️ Architecture

### System Architecture Diagram

```
┌─────────────────────────────────────────────────────────┐
│              User Interface Layer (Frontend)             │
│  ┌───────────────────────────────────────────────────┐  │
│  │  React 19 + TypeScript Control Center              │  │
│  │  ├── Dashboard View       (Status overview)        │  │
│  │  ├── Reminders View       (Upcoming reminders)     │  │
│  │  ├── History View         (Command logs)           │  │
│  │  ├── Memory View          (Stored memories)        │  │
│  │  ├── Test Commands        (Debug & test)           │  │
│  │  ├── Integrations         (Service connections)    │  │
│  │  └── Settings             (Configuration)          │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────┬───────────────────────────────────────┘
                  │ Tauri IPC (invoke)
┌─────────────────▼───────────────────────────────────────┐
│         Command Handler Layer (Tauri/Rust)              │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Tauri Commands (5 endpoints)                      │  │
│  │  ├── execute_action()         [Core dispatcher]    │  │
│  │  ├── get_memories()           [Data retrieval]     │  │
│  │  ├── get_reminders()          [Data retrieval]     │  │
│  │  ├── get_command_history()    [Data retrieval]     │  │
│  │  └── greet()                  [Utility]            │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────┬───────────────────────────────────────┘
                  │ Intent JSON
┌─────────────────▼───────────────────────────────────────┐
│         Core Logic Layer (Rust - lib.rs)                │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Intent Processing Engine                          │  │
│  │  ├── Intent Parsing & Validation                   │  │
│  │  ├── Permission Checking                           │  │
│  │  ├── Event Publishing (EventBus)                   │  │
│  │  └── Intent Execution Routing                      │  │
│  └───────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Infrastructure Layer                              │  │
│  │  ├── EventBus (pub/sub)                            │  │
│  │  ├── PermissionManager (access control)            │  │
│  │  ├── Telemetry (metrics)                           │  │
│  │  └── Scheduler (task execution)                    │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────┬───────────────────────────────────────┘
                  │ Database Query
┌─────────────────▼───────────────────────────────────────┐
│         Persistence Layer (SQLite)                      │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Database: noddy.db                                │  │
│  │  ├── memories table                                │  │
│  │  │   └── Stores both memories and reminders        │  │
│  │  │       (uses expires_at as discriminator)        │  │
│  │  └── [Future: commands, analytics, settings]       │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                  │ Natural Language
┌─────────────────▼───────────────────────────────────────┐
│       Intelligence Layer (Python FastAPI)               │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Intent Classifier (future NLP models)             │  │
│  │  Rule-Based Parser (current implementation)        │  │
│  │  Context & History Processor (future)              │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│         External Systems (App Control)                  │
│  ├── Windows Process Management (std::process)          │
│  ├── Windows Registry (app discovery)                   │
│  ├── Web Browser (URL handling)                         │
│  └── File System & Configuration                        │
└─────────────────────────────────────────────────────────┘
```

### Data Flow Example: "remember my project deadline"

```
1. User Input
   ↓
2. Natural Language → "remember X" pattern detected
   ↓
3. Parsed Intent: { name: "remember", payload: { content: "my project deadline" } }
   ↓
4. Tauri Command: invoke("execute_action", { intentJson })
   ↓
5. Rust Backend: execute_action() receives JSON
   ↓
6. Intent Deserialization: Intent::Remember { content: "..." }
   ↓
7. Permission Check: user has write_memory permission
   ↓
8. Memory Storage: save to SQLite with timestamp
   ↓
9. Event Publishing: EventBus.emit(MemorySaved(content))
   ↓
10. Telemetry: Record execution time, success
   ↓
11. Response: { success: true, message: "Memory saved" }
   ↓
12. Frontend: Update UI or show confirmation
```

---

## 🛠️ Technology Stack

### Frontend
- **React 19** - Modern web framework with hooks
- **TypeScript 5.8** - Type-safe JavaScript
- **Vite 7.3** - Fast bundler and dev server
- **Framer Motion** - Smooth animations (fade, slide, stagger)
- **CSS Variables** - Dynamic theming and styling
- **Responsive Design** - Mobile-friendly layouts

### Backend (Rust)
- **Tauri v2** - Desktop app framework (IPC, window management)
- **Tokio** - Async runtime
- **serde + serde_json** - JSON serialization
- **rusqlite** - SQLite database access
- **winreg** - Windows registry for app discovery
- **chrono** - Date/time manipulation

### Intelligence (Python)
- **FastAPI** - Modern async web framework
- **Pydantic** - Data validation
- **Python 3.9+** - Core language

### Database
- **SQLite** - Local relational database
  - Zero-configuration
  - No server required
  - ACID compliance
  - Excellent for desktop apps

---

## 📦 Installation

### Prerequisites
- Rust 1.70+ ([Install](https://rustup.rs/))
- Node.js 18+ & npm ([Install](https://nodejs.org/))
- Python 3.9+ ([Install](https://python.org/))
- Windows 10+ (currently optimized for Windows)

### Step 1: Clone Repository
```bash
git clone <repository-url>
cd Noddy
```

### Step 2: Install Dependencies

**Rust Backend:**
```bash
cd src-tauri
cargo fetch  # Download Rust dependencies
```

**Frontend:**
```bash
npm install  # Install Node dependencies
```

**Python Brain (Optional - for NLP):**
```bash
cd brain
pip install -r requirements.txt
```

### Step 3: Verify Installation
```bash
# Check Rust
cargo --version

# Check Node
npm --version

# Check Python
python --version
```

---

## 🚀 Quick Start

### Development Mode

**Option 1: Full Stack (Recommended)**
```bash
# Terminal 1: Python Intelligence Server
cd brain
python main.py  # Runs on http://127.0.0.1:8000

# Terminal 2: Tauri App Development
npm run tauri dev
```

**Option 2: Frontend Only
```bash
npm run dev     # React dev server (with mock data)
```

**Option 3: Backend Only**
```bash
cargo run --manifest-path src-tauri/Cargo.toml
```

### Production Build
```bash
# Build Rust backend and React frontend
npm run build

# Create distributable executable
npm run tauri build
```

### Running Tests

**Python Backend Tests:**
```bash
cd brain
python -m pytest test_parser.py -v
```

**Rust Tests:**
```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

---

## 🔧 Feature Implementation Details

### 1. Memory Management

**Database Schema:**
```sql
CREATE TABLE memories (
  id TEXT PRIMARY KEY,
  content TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  expires_at INTEGER,  -- NULL for memories, timestamp for reminders
  source TEXT          -- Python, Local, Test
);
```

**Rust Implementation (lib.rs):**
- `save_memory()` - Stores memory with UUID and timestamp
- `recall_memories()` - Queries all memories (expires_at IS NULL)
- `search_memories()` - Full-text search on content
- `format_timestamp()` - Converts Unix epochs to relative format

**Frontend Integration (App.tsx):**
- `fetchMemories()` - Calls `invoke("get_memories", {limit: 2})`
- `MemoryView` - Displays stored memories in scrollable list
- `DashboardView` - Shows latest 2 memories in "Memory Vault" card

**Example Usage:**
```
User: "remember my meeting is at 2pm"
→ Intent: { name: "remember", payload: { content: "my meeting is at 2pm" } }
→ Stored: { id: "uuid-123...", content: "...", created_at: 1709640000, expires_at: null }
→ Retrieved: [{ id, content, timestamp: "just now" }]
```

### 2. Reminder System

**Storage Pattern:**
- Reminders stored in same `memories` table
- Discriminated by `expires_at` field (not null = reminder)
- Supports future scheduling with Unix timestamp

**Time Parsing (App.tsx, lines ~770-800):**
```typescript
"remind me to do X in 2 hours"       → trigger_at = now + 7200s
"remind me to do X in 10 minutes"    → trigger_at = now + 600s
"remind me to do X in 3 days"        → trigger_at = now + 259200s
"remind me to do X tomorrow at 3pm"  → trigger_at = tomorrow 15:00
```

**Rust Implementation:**
- `set_reminder()` - Stores with expires_at = trigger_at
- `check_expired_reminders()` - Finds reminders where expires_at > now
- `format_timestamp()` - Shows countdown ("in 2 hours", "in 30 minutes")

**Frontend Display:**
- `RemindersView` - Full reminder list with delete buttons
- `DashboardView` - Shows next 3 reminders with countdowns
- Real-time updates via `useEffect` on mount

### 3. Application Control

**Windows App Discovery:**
- Scans Windows registry: `HKEY_LOCAL_MACHINE\Software\Microsoft\Windows\CurrentVersion\Uninstall`
- Extracts DisplayName and InstallLocation
- Builds searchable app index
- Supports partial name matching (case-insensitive)

**Execution Methods:**
- `open_app()` - `std::process::Command` to launch executable
- `open_url()` - Default browser via shell execute
- `kill_process()` - `taskkill` command for process termination
- Error handling for missing or denied access

**Intent Examples:**
```
open chrome         → find chrome.exe, std::process::Command::new("chrome.exe")
open https://...    → shell open URL → default browser
kill notepad        → taskkill /IM notepad.exe /F
list apps           → read Windows registry → return JSON array
```

### 4. Intent Processing Pipeline

**Intent Enum (lib.rs, lines 29-62):**
```rust
enum Intent {
    Remember { content: String },
    RecallMemory,
    SearchMemory { keyword: String },
    SetReminder { content: String, trigger_at: i64 },
    SearchWeb { url: String },
    OpenApp { target: String },
    OpenUrl { url: String },
    KillProcess { process: String },
    ListApps,
    Unknown { text: String },
}
```

**Processing Flow (execute_action, lines ~1280):**
```
1. Receive Intent JSON from frontend
2. Deserialize to Intent enum
3. Check user permissions
4. Emit IntentReceived event
5. Route to handler based on intent type
6. Execute and capture result
7. Emit IntentExecuted event with timing
8. Return response JSON
9. Log telemetry
```

### 5. Event-Driven Architecture

**EventBus (lib.rs, lines ~65-100):**
```rust
struct EventBus {
    subscribers: Arc<Mutex<Vec<Box<dyn Fn(&Event) + Send + Sync>>>>,
}

enum Event {
    IntentReceived(String),
    IntentExecuted { intent_name: String, duration_ms: u128 },
    MemorySaved(String),
    ReminderScheduled(String),
    ReminderTriggered(String),
    ErrorOccurred(String),
}
```

**Usage Pattern:**
- Components register listeners: `event_bus.register(|event| { ... })`
- Publishers emit events: `event_bus.emit(&Event::MemorySaved(...))`
- Non-blocking, no return values
- Enables loose coupling

**Example Integration:**
- Telemetry listener counts IntentExecuted events
- Scheduler listener checks ReminderScheduled events
- UI layer can listen for state changes

### 6. Permission Management

**Permission Types (PermissionManager, lines ~110-150):**
- `execute_commands` - Run apps/processes
- `read_memory` - Access stored memories
- `write_memory` - Create new memories
- `manage_reminders` - Set/delete reminders
- `kill_process` - Terminate processes
- `list_apps` - View available apps

**Access Control (execute_action, before handler):**
```rust
match intent {
    Intent::Remember { .. } => {
        if !pm.check("write_memory") { return Err(...) }
    },
    Intent::KillProcess { .. } => {
        if !pm.check("kill_process") { return Err(...) }
    },
    // ... etc
}
```

**Current Implementation:**
- All operations granted by default
- Framework ready for role-based access
- Audit logging TODO

### 7. Control Center UI

**View Components (App.tsx):**

| View | Purpose | Data Source | Status |
|------|---------|-------------|--------|
| Dashboard | Overview of all system status | get_memories, get_reminders, mock data | ✅ |
| Reminders | Manage upcoming reminders | get_reminders | ✅ |
| History | Command execution logs | get_command_history | 🔄 Empty (TODO) |
| Memory | Browse all stored memories | get_memories | ✅ |
| Test Commands | Debug and test intents | execute_action | ✅ |
| Integrations | Third-party service connections | Mock data | 🔄 Stub |
| Settings | Configuration panel | Mock data | 🔄 Stub |

**Animations (Framer Motion):**
- Panel transitions: fade + slide (x: ±40px)
- Card animations: staggered fade in + y: 20→0
- Hover effects: scale 1.01-1.03, translate
- List items: stagger delay (index * 0.05s)
- Icon animations: rotate, scale on interaction

**Responsive Design:**
- Sidebar: Fixed 240px width
- Main content: max-width 1200px, centered
- Grid: 2-column responsive (minmax(400px, 1fr))
- Mobile: Needs sidebar collapse logic (TODO)

---

## 📡 API Reference

### Tauri Commands

#### `execute_action(intentJson: string): Promise<any>`
Executes a parsed intent and returns result.

**Parameters:**
```typescript
intentJson: string  // JSON string of Intent enum
```

**Example:**
```typescript
const result = await invoke("execute_action", {
  intentJson: JSON.stringify({
    name: "remember",
    payload: { content: "meeting at 2pm" }
  })
});
// Returns: { success: true, message: "Memory saved" }
```

**Returns:**
```typescript
{
  success: boolean,
  message: string,
  data?: any,
  duration?: number
}
```

---

#### `get_memories(limit?: number): Promise<Memory[]>`
Retrieves stored memories.

**Parameters:**
```typescript
limit?: number  // Max results (default: 10)
```

**Returns:**
```typescript
[
  {
    id: string,
    content: string,
    timestamp: string  // Relative format: "2 hours ago"
  },
  ...
]
```

---

#### `get_reminders(limit?: number): Promise<Reminder[]>`
Retrieves future reminders.

**Parameters:**
```typescript
limit?: number  // Max results (default: 10)
```

**Returns:**
```typescript
[
  {
    id: string,
    content: string,
    time: string,  // Countdown: "in 2 hours"
    source: string  // "Local", "Google", "Outlook"
  },
  ...
]
```

---

#### `get_command_history(limit?: number): Promise<CommandHistory[]>`
Retrieves executed command logs.

**Parameters:**
```typescript
limit?: number  // Max results (default: 20)
```

**Returns:**
```typescript
[
  {
    id: string,
    command: string,
    intent: string,
    duration: number,  // milliseconds
    timestamp: string,
    success: boolean
  },
  ...
]
```

**Note:** Currently returns empty array. TODO: Implement tracking.

---

### Command Syntax Reference

| Command | Intent | Example | Result |
|---------|--------|---------|--------|
| Remember | `remember` | `"remember my deadline"` | Stores memory |
| Recall | `recall_memory` | `"recall"`, `"what do you remember?"` | Lists memories |
| Search | `search_memory` | `"search python"` | Filtered memories |
| Remind | `set_reminder` | `"remind me to call in 2 hours"` | Scheduled reminder |
| Web | `search_web` | `"google machine learning"` | Browser search |
| Open | `open_app` / `open_url` | `"open chrome"`, `"open google.com"` | Launches app/URL |
| Kill | `kill_process` | `"kill notepad"` | Terminates process |
| List | `list_apps` | `"list apps"` | Shows installed apps |

---

## 📂 Project Structure

```
Noddy/
├── src/                          # React Frontend
│   ├── App.tsx                   # Main app component (960 lines)
│   │   ├── DataTypes (Memory, Reminder, etc.)
│   │   ├── Sidebar Navigation (7 views)
│   │   ├── Dashboard View (4 cards)
│   │   ├── View Components (Reminders, History, Memory, etc.)
│   │   ├── useEffect hooks (data fetching)
│   │   └── Event handlers
│   ├── App.css                   # Dark theme styles (385+ lines)
│   │   ├── CSS variables
│   │   ├── Component styles
│   │   ├── Animations (@keyframes)
│   │   └── Responsive queries
│   ├── main.tsx                  # React entry point
│   ├── vite-env.d.ts            # Vite environment types
│   └── assets/                   # Images, fonts
│
├── src-tauri/                    # Rust Backend
│   ├── src/
│   │   ├── lib.rs               # Core logic (1990+ lines)
│   │   │   ├── Intent enum
│   │   │   ├── EventBus (pub/sub)
│   │   │   ├── PermissionManager (access control)
│   │   │   ├── Telemetry (metrics)
│   │   │   ├── Scheduler (task execution)
│   │   │   ├── MemoryStore (SQLite wrapper)
│   │   │   ├── execute_action() (main handler)
│   │   │   ├── Intent handlers (remember, recall, etc.)
│   │   │   ├── Tauri commands (5 endpoints)
│   │   │   ├── Database initialization
│   │   │   └── Helper functions
│   │   └── main.rs              # App entry point
│   ├── Cargo.toml               # Rust dependencies
│   ├── tauri.conf.json          # App configuration
│   ├── build.rs                 # Build script
│   ├── capabilities/            # Tauri security policies
│   ├── icons/                   # App icons
│   └── target/                  # Build artifacts
│
├── brain/                        # Python Intelligence
│   ├── main.py                  # FastAPI server
│   ├── models.py                # Intent model
│   ├── parsers/
│   │   ├── base.py
│   │   ├── app_parser.py
│   │   └── memory_parser.py
│   ├── domain/
│   │   └── intent.py
│   ├── utils.py
│   ├── config.py
│   ├── requirements.txt
│   ├── test_parser.py           # Unit tests
│   └── README.md
│
├── shared/                      # Shared assets
│   └── intent.schema.json       # Intent JSON schema
│
├── public/                      # Static assets
├── dist/                        # Built frontend (prod)
│
├── package.json                 # Node dependencies & scripts
├── tsconfig.json                # TypeScript config
├── vite.config.ts              # Vite bundler config
├── index.html                   # HTML entry point
│
├── README.md                    # This file
├── INTEGRATION_STATUS.md        # Phase 3d integration details
├── PHASE_3_CONTROL_CENTER_SUMMARY.md  # UI implementation
└── features.md                  # Feature list (if exists)
```

---

## 👨‍💻 Development

### Code Style

**TypeScript/React:**
- Functional components with hooks
- PascalCase for components
- camelCase for variables/functions
- Type annotations on props and returns

**Rust:**
- Idiomatic Rust with error handling
- Module organization
- Documentation comments
- Avoid unsafe code

**Python:**
- PEP 8 style guide
- Type hints where possible
- Docstrings for functions

### Build Commands

```bash
# Development
npm run dev          # React dev server
npm run tauri dev    # Tauri dev with hot reload

# Production
npm run build        # Build React + Rust
npm run tauri build  # Create installers

# Testing
cargo test --manifest-path src-tauri/Cargo.toml
cd brain && python -m pytest test_parser.py

# Type Checking
npm run tsc          # TypeScript compiler check
```

### Adding New Features

**Process:**
1. Define Intent variant in `enum Intent` (lib.rs)
2. Implement handler in `execute_action()` match statement
3. Add parsing rule to command parser (frontend or Python brain)
4. Create UI component if needed
5. Add to Tauri command handler if data retrieval needed
6. Test with Test Commands view
7. Document in README

**Example: New Reminder Intent**
```rust
// Step 1: Add to enum
enum Intent {
    #[serde(rename = "snooze_reminder")]
    SnoozeReminder { reminder_id: String, snooze_minutes: i64 },
    ...
}

// Step 2: Add handler
Intent::SnoozeReminder { reminder_id, snooze_minutes } => {
    if !pm.check("manage_reminders") { return Err(...) }
    let new_trigger = utc_now + snooze_minutes * 60;
    memory_store.update_reminder(&reminder_id, new_trigger)?;
    event_bus.emit(&Event::ReminderScheduled(...));
    Ok(json!({ "success": true }))
}

// Step 3: Parse in frontend
if trimmed.starts_with("snooze ") {
    // Extract minutes and reminder ID
    return JSON.stringify({ name: "snooze_reminder", payload: {...} });
}
```

---

## 🗓️ Future Roadmap

### Phase 3e: Command History Tracking (Next)
- **Goal:** Track all executed commands with metrics
- **Implementation:**
  - Store in database or JSON file
  - Emit CommandExecuted events with full metadata
  - Return via get_command_history() API
  - Display in History View with filtering

### Phase 3f: UI Polish
- **Goal:** Complete Control Center functionality
- **Features:**
  - Mobile-responsive sidebar (collapse/hamburger)
  - "New Memory" button in Memory View
  - "New Reminder" button in Reminders View
  - Delete confirmation dialogs
  - Keyboard shortcuts (Ctrl+K for quick commands)
  - Dark/light theme toggle
  - Full Settings panel

### Phase 4: Advanced Reminders
- **Goal:** Rich reminder features
- **Features:**
  - Recurring reminders (daily, weekly, monthly)
  - Reminder categories/tags
  - Sound/visual notifications
  - Reminder snooze functionality
  - Calendar integration

### Phase 5: Natural Language Processing
- **Goal:** Move beyond rule-based parsing
- **Approach:**
  - Transformer-based intent classification
  - Named entity recognition (NER)
  - Context awareness (remember previous queries)
  - Fuzzy command matching
  - Multi-intent parsing

### Phase 6: Advanced Search & Memory
- **Goal:** Intelligent memory retrieval
- **Features:**
  - Semantic search (embedding-based)
  - Memory relationships/linking
  - Auto-categorization
  - Periodic summaries
  - Memory decay/archival

### Phase 7: Deep Integrations
- **Goal:** Third-party service connections
- **Platforms:**
  - Google Calendar ↔ Reminders
  - Outlook ↔ Reminders
  - Slack ↔ Commands
  - GitHub ↔ Notifications
  - Spotify ↔ Music control

### Phase 8: Offline LLM Integration
- **Goal:** Local AI for natural language
- **Models:**
  - LLaMA or Mistral (run locally)
  - Custom fine-tuning on user commands
  - Intent confidence scores
  - Fallback to rule-based parsing

### Phase 9: Learning & Personalization
- **Goal:** Adapt to user behavior
- **Features:**
  - Command frequency analysis
  - Quick shortcuts for favorite commands
  - Predictive command suggestions
  - User preference learning

### Phase 10: Advanced Automation
- **Goal:** Complex automation workflows
- **Features:**
  - Workflow chaining (if-then-else)
  - State machines for command sequences
  - Conditional execution
  - Time-based triggers

---

## 🐛 Troubleshooting

### Issue: Frontend can't reach backend

**Solution:**
```bash
# Verify Tauri dev server is running
npm run tauri dev

# Check for CORS issues in browser console
# Verify IPC communication in DevTools
```

### Issue: Memories not persisting

**Solution:**
```bash
# Check SQLite database exists
# Default location: ~/.config/noddy/noddy.db (Linux)
#                  ~/Library/Application Support/noddy/noddy.db (macOS)
#                  %APPDATA%\noddy\noddy.db (Windows)

# Verify write permissions to directory
```

### Issue: Commands failing to execute

**Solution:**
```bash
# Test command in Test Commands view first
# Check Windows app registry is readable
# Verify app executable paths exist
```

---

## 📄 License

[Add your license here]

---

## 🤝 Contributing

Contributions welcome! Please:
1. Fork the repository
2. Create feature branch (`git checkout -b feature/NewFeature`)
3. Commit changes (`git commit -m 'Add NewFeature'`)
4. Push to branch (`git push origin feature/NewFeature`)
5. Open Pull Request

---

## 💬 Support

For issues, questions, or suggestions:
- Open a GitHub issue
- Check existing documentation
- Review code comments for context

---

## 📞 Contact

[Add contact information here]

---

**Last Updated:** March 2026 | **Version:** 0.5 Beta | **Maintainer:** [Your Name]
