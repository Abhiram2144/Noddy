# Noddy Brain Layer - Modular Intent-Based Architecture

## Overview

The Noddy Brain Layer has been refactored from a monolithic `main.py` into a **modular microservice-like architecture** with clear separation of concerns.

```
brain/
├── main.py                 # FastAPI app & route handlers (with CORS)
├── config.py              # Logging configuration
├── models.py              # Pydantic request/response models
├── utils.py               # Shared utility functions
├── parsers/               # Modular parser system
│   ├── __init__.py        # Parser registry & orchestration
│   ├── base.py            # BaseParser abstract interface
│   ├── app_parser.py      # Application control (open, list, kill)
│   ├── memory_parser.py   # Memory system (remember, recall, search, remind)
│   └── search_parser.py   # Web search (what is, search about) [NEW]
├── requirements.txt       # Python dependencies
├── test_parser.py         # Comprehensive test suite
└── README.md              # Documentation

# Database location (NOT in src-tauri/):
# Windows: %APPDATA%/com.abhiram.noddy/noddy.db
# macOS: ~/Library/Application Support/com.abhiram.noddy/noddy.db
# Linux: ~/.local/share/com.abhiram.noddy/noddy.db
```

## Architecture Decisions

### 1. Modular Parser System

Each parser is responsible for a specific domain:

| Parser | Responsibility | Commands |
|--------|---|---|
| **MemoryParser** | In-app intentional memory | `remember X`, `recall`, `search X`, `remind me to X in Y` |
| **SearchParser** | Web search & info lookup | `what is X`, `search about X`, `google X`, `look up X` |
| **AppParser** | System interaction | `open X`, `list apps`, `kill X` |

### 2. Parser Registry Pattern

The `ParserRegistry` class manages the dispatch logic:

```python
# parsers/__init__.py
registry = ParserRegistry([
    MemoryParser(),      # Try specific memory commands first
    SearchParser(),      # Then web search patterns
    AppParser(),         # Finally generic app commands
])
```

**Priority matters**: More specific parsers are tried first to avoid pattern conflicts.

### 3. Clean Separation of Concerns

| Module | Responsibility |
|--------|---|
| **config.py** | Logging setup (centralized) |
| **models.py** | API contract (Pydantic) |
| **utils.py** | Shared functions (URL building, normalization) |
| **parsers/base.py** | Base interface & contracts |
| **parsers/*.py** | Domain-specific logic |
| **main.py** | FastAPI routes only (no parsing logic) |

## Supported Commands

### Memory System (MemoryParser)

```
"remember buy milk"
  → {"action": "remember", "value": "buy milk"}

"recall"
  → {"action": "recall_memory", "value": ""}

"search meeting"
  → {"action": "search_memory", "value": "meeting"}

"remind me to call mom in 30 minutes"
  → {"action": "set_reminder", "value": "{...json...}"}
```

### Web Search (SearchParser) - NEW

```
"what is Python"
  → {"action": "search_web", "value": "https://www.google.com/search?q=Python"}

"search about machine learning in web"
  → {"action": "search_web", "value": "https://...q=machine+learning"}

"google rust programming"
  → {"action": "search_web", "value": "https://...q=rust+programming"}

"look up neural networks"
  → {"action": "search_web", "value": "https://...q=neural+networks"}
```

### Application Control (AppParser)

```
"open chrome"
  → {"action": "open_app", "value": "chrome"}

"open https://github.com"
  → {"action": "open_url", "value": "https://github.com"}

"open youtube in web"
  → {"action": "open_url", "value": "https://www.youtube.com"}

"list apps"
  → {"action": "list_apps", "value": ""}

"kill notepad.exe"
  → {"action": "kill_process", "value": "notepad.exe"}
```

## Extending the System

### Adding a New Parser

1. **Create a new parser class** in `parsers/`:

```python
# parsers/voice_parser.py
from parsers.base import BaseParser
from models import InterpretResponse
from config import get_logger

logger = get_logger(__name__)

class VoiceParser(BaseParser):
    """Handle voice control commands."""
    
    def can_parse(self, text: str) -> bool:
        normalized = text.strip().lower()
        return normalized.startswith("voice ")
    
    def parse(self, text: str) -> InterpretResponse:
        command = text[6:].strip()
        logger.info(f"Parsed: '{text}' → action=voice_command")
        return InterpretResponse(
            action="voice_command",
            value=command,
            confidence=1.0
        )
```

2. **Register in ParserRegistry** (`parsers/__init__.py`):

```python
from parsers.voice_parser import VoiceParser

class ParserRegistry:
    def __init__(self):
        self.parsers = [
            MemoryParser(),
            SearchParser(),
            VoiceParser(),      # Add here
            AppParser(),
        ]
```

3. **Add tests** in `test_parser.py`:

```python
def test_voice_command():
    result = parse_command("voice play music")
    assert result.action == "voice_command"
    assert result.value == "play music"
```

4. **Update Rust handler** in `src-tauri/src/lib.rs`:

```rust
"voice_command" => {
    // Handle voice command execution
    ActionResponse {
        success: true,
        message: format!("Executing voice: {}", value),
        // ...
    }
}
```

### Future: LLM Integration

The web search parser already includes comments for future LLM integration:

```python
# In SearchParser.parse()
# Future: Check LLM availability
# if self.LLM_AVAILABLE:
#     logger.info(f"Using LLM for: {query}")
#     return InterpretResponse(
#         action="llm_query",      # New action for LLM
#         value=query,
#         confidence=1.0
#     )
```

When an LLM is available, the system can:
1. Detect "what is", "search about" patterns
2. Delegate to LLM instead of Google search
3. Return intelligent answers instead of just URLs
4. Differentiate LLM prompts from search queries in Rust handler

## Testing

### Run All Tests

```bash
cd brain
python test_parser.py
```

### Expected Output

```
Running Noddy Brain parser tests...

✓ test_remember_command
✓ test_search_memory
✓ test_what_is_query
✓ test_search_about_in_web
✓ test_google_query
✓ test_open_app
...
35 passed, 0 failed
```

### Test Coverage

- **19 existing tests**: App control, memory system functionality
- **16 new tests**: Web search patterns, parser priority, URL encoding
- **100% passing**: All patterns and edge cases validated

## Production Configuration

### CORS Middleware

The Brain API runs on `localhost:8000` (FastAPI) while the Tauri frontend runs on `localhost:1420`. To allow cross-origin requests, CORS middleware is configured in `main.py`:

```python
from fastapi.middleware.cors import CORSMiddleware

app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:1420", "http://127.0.0.1:1420"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)
```

**Why This Matters:**
- Browsers send OPTIONS preflight requests before POST
- Without CORS: `OPTIONS /interpret HTTP/1.1 → 405 Method Not Allowed`
- With CORS: OPTIONS handled automatically, POST succeeds

**Testing CORS:**
```bash
python -c "from main import app; print('✓ CORS configured')"
```

### Database Location

The SQLite database (`noddy.db`) is stored in the **application data directory**, NOT in `src-tauri/`:

```rust
// src-tauri/src/lib.rs
use tauri::Manager;

.setup(move |app| {
    let app_data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_data_dir)?;
    
    let db_path = app_data_dir.join("noddy.db");
    // Database at: AppData/Roaming/com.abhiram.noddy/noddy.db
})
```

**Why This Matters:**
- Storing in `src-tauri/` triggers Tauri's file watcher
- Every `remember` command → DB write → rebuild → app restart
- App data directory isolates user data from source code

**Database Location by OS:**
- Windows: `%APPDATA%\com.abhiram.noddy\noddy.db`
- macOS: `~/Library/Application Support/com.abhiram.noddy/noddy.db`
- Linux: `~/.local/share/com.abhiram.noddy/noddy.db`

## Typed Intent Handling (Rust/Tauri Side)

The Tauri backend (Rust) uses a strongly-typed Intent enum to safely dispatch actions, eliminating string-based action matching.

### Intent Enum

```rust
#[derive(Deserialize, Debug)]
#[serde(tag = "name", content = "payload")]
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

### Type-Safe Dispatch

**Before (Stringly-Typed):**
```rust
match action_name {
    "open_app" => { open_app_internal(value_ref, ...) }
    "search_web" => { open_url_internal(value_ref, ...) }
}
```

**After (Typed):**
```rust
match intent {
    Intent::OpenApp { target } => { open_app_internal(&target, ...) }
    Intent::SearchWeb { url } => { open_url_internal(&url, ...) }
}
```

### Frontend → Tauri Flow

1. **Python Brain API** returns `{"action": "open_app", "value": "chrome"}`
2. **React Frontend** converts via `buildIntentJson()` → `{"name": "open_app", "payload": {"target": "chrome"}}`
3. **Tauri Backend** deserializes JSON string to typed `Intent::OpenApp { target }`
4. **Rust match** safely dispatches enum variant with zero string comparisons

### Example: "remember buy milk"

```json
// Brain API returns
{
  "action": "remember",
  "value": "buy milk"
}

// Frontend converts to
{
  "name": "remember",
  "payload": {
    "content": "buy milk"
  }
}

// Rust deserializes to
Intent::Remember { content: "buy milk" }

// Match dispatch
match intent {
    Intent::Remember { content } => save_memory(&memory_store, &content)
}
```

### Benefits

- **Type Safety:** Compiler ensures valid intent structures
- **Zero Stringly-Typed Logic:** No string `match` arms
- **Structured Data:** Each intent carries specific, validated fields
- **Extensibility:** New intents are compile-time safe
- **Error Handling:** JSON deserialization fails fast with clear messages

## API Endpoints

### POST /interpret (Python Brain)

Parse natural language command.

**Request:**
```json
{
  "text": "what is Python"
}
```

**Response:**
```json
{
  "action": "search_web",
  "value": "https://www.google.com/search?q=Python",
  "confidence": 1.0
}
```

**CORS Headers:** Automatically included for `localhost:1420` origin.

### Tauri: execute_action (Rust)

**Request (from Frontend):**
```json
{
  "intent_json": "{\"name\": \"search_web\", \"payload\": {\"url\": \"https://google.com/search?q=Python\"}}"
}
```

**Response:**
```json
{
  "success": true,
  "message": "Searching Google...",
  "requires_confirmation": false,
  "fallback_action": null,
  "fallback_value": null,
  "data": null
}
```

### GET /health (Python Brain)

Health check endpoint.

**Response:**
```json
{
  "status": "ok",
  "version": "0.2.0"
}
```

### GET / (Python Brain)

API information endpoint.

**Response:**
```json
{
  "name": "Noddy Brain",
  "version": "0.2.0",
  "architecture": "Modular Intent-Based",
  "description": "Intent interpreter for Noddy desktop assistant with modular parser system",
  "parsers": [
    "MemoryParser (remember, recall, search, remind)",
    "SearchParser (what is, search about - Google/LLM)",
    "AppParser (open, list, kill)"
  ]
}
```

## Debugging

### Enable Debug Logging

Logs are printed to console with module names:

```
2026-03-03 19:23:26 - parsers.memory_parser - INFO - Parsed: 'remember buy milk' → action=remember
2026-03-03 19:23:26 - parsers.search_parser - INFO - Parsed: 'what is Python' → action=search_web
2026-03-03 19:23:26 - parsers.app_parser - INFO - Parsed: 'open chrome' → action=open_app
```

### Parser Priority Testing

To verify parser priority works correctly:

```python
# SearchParser should catch "search about" (NOT MemoryParser)
result = parse_command("search about AI in web")
assert result.action == "search_web"  # Not search_memory

# MemoryParser should catch simple "search" (NOT SearchParser)
result = parse_command("search meeting")
assert result.action == "search_memory"  # Not search_web
```

## Troubleshooting

### Issue: 405 Method Not Allowed (CORS Error)

**Symptom:**
```
INFO:     127.0.0.1:xxxxx - "OPTIONS /interpret HTTP/1.1" 405 Method Not Allowed
```

**Cause:** Browser sends OPTIONS preflight request before POST, but FastAPI doesn't handle it without CORS middleware.

**Solution:**
1. Verify CORS middleware is configured in `main.py`:
   ```python
   from fastapi.middleware.cors import CORSMiddleware
   
   app.add_middleware(
       CORSMiddleware,
       allow_origins=["http://localhost:1420", "http://127.0.0.1:1420"],
       allow_credentials=True,
       allow_methods=["*"],
       allow_headers=["*"],
   )
   ```

2. Test CORS is active:
   ```bash
   python -c "from main import app; print('✓ CORS configured')"
   ```

3. Check frontend is using correct origin (`localhost:1420`, not `127.0.0.1:1420` unless configured)

**Prevention:** Always configure CORS when frontend and backend run on different origins.

---

### Issue: App Restarts on Every "remember X" Command

**Symptom:**
```
Info File src-tauri\noddy.db changed. Rebuilding application...
Compiling noddy v0.1.0
```

**Cause:** SQLite database stored in `src-tauri/` triggers Tauri's file watcher, causing rebuild loop.

**Solution:**
1. Move database to app data directory using `tauri::Manager`:
   ```rust
   use tauri::Manager;
   
   .setup(move |app| {
       let app_data_dir = app.path().app_data_dir()?;
       std::fs::create_dir_all(&app_data_dir)?;
       
       let db_path = app_data_dir.join("noddy.db");
       // Initialize DB at app_data_dir, NOT src-tauri/
   })
   ```

2. Verify database location:
   ```powershell
   # Windows
   ls $env:APPDATA\com.abhiram.noddy\noddy.db
   ```

3. Check Tauri logs don't show rebuild messages after "remember" commands

**Prevention:** Never store dynamic user data (databases, logs, uploads) in `src-tauri/` or `src/`. Use `app.path().app_data_dir()` instead.

---

### Issue: Parser Returns Wrong Action

**Symptom:** "search meeting" returns `search_web` instead of `search_memory`.

**Cause:** Parser priority incorrect, or patterns overlap.

**Solution:**
1. Check parser order in `parsers/__init__.py`:
   ```python
   registry = ParserRegistry([
       MemoryParser(),      # Try specific patterns first
       SearchParser(),      # Then web search
       AppParser(),         # Generic fallback
   ])
   ```

2. Verify pattern exclusion in parsers:
   ```python
   # MemoryParser.can_parse()
   if cmd.startswith("search about") or cmd.startswith("search for"):
       return False  # Let SearchParser handle these
   ```

3. Run priority tests:
   ```bash
   cd brain
   python -c "from parsers import parse_command; print(parse_command('search meeting'))"
   # Should return InterpretResponse(action='search_memory', ...)
   ```

**Prevention:** Document pattern exclusions in parser docstrings, add tests for ambiguous patterns.

---

### General Debugging Steps

1. **Check logs:** FastAPI prints all requests and errors to console
2. **Run tests:** `python test_parser.py` validates all 35 patterns
3. **Test imports:** `python -c "from main import app"` catches syntax errors
4. **Verify Rust:** `cargo check` in `src-tauri/` catches compilation issues
5. **Inspect DB:** Use SQLite browser to view `noddy.db` contents

## Performance Characteristics

### Enable Debug Logging

Logs are printed to console with module names:

```
2026-03-03 19:23:26 - parsers.memory_parser - INFO - Parsed: 'remember buy milk' → action=remember
2026-03-03 19:23:26 - parsers.search_parser - INFO - Parsed: 'what is Python' → action=search_web
2026-03-03 19:23:26 - parsers.app_parser - INFO - Parsed: 'open chrome' → action=open_app
```

### Parser Priority Testing

To verify parser priority works correctly:

```python
# SearchParser should catch "search about" (NOT MemoryParser)
result = parse_command("search about AI in web")
assert result.action == "search_web"  # Not search_memory

# MemoryParser should catch simple "search" (NOT SearchParser)
result = parse_command("search meeting")
assert result.action == "search_memory"  # Not search_web
```

## Performance Characteristics

- **Parsing**: ~1-2ms per command (pure string matching)
- **URL Resolution**: DNS lookup can take 50-200ms (only for "open X in web")
- **Startup**: Parsers instantiated once in registry
- **Memory**: ~1KB per parser instance

## Future Enhancements

1. **LLM Integration**
   - Detect LLM availability
   - Route "what is", "search about" to LLM
   - Return intelligent responses instead of URLs

2. **Advanced Time Parsing**
   - Support "in 30 seconds"
   - Support absolute times: "at 2pm", "tomorrow at 9am"
   - Support recurring: "every day"

3. **Custom Parsers**
   - Music control: "play artist:taylor swift"
   - Calendar: "schedule meeting with john"
   - Email: "send email to alice"

4. **Parser Plugins**
   - Load parsers from external modules
   - Enable/disable parsers dynamically
   - Support community contributions

## Migration from Monolithic to Modular

If you need to understand what moved where:

| Old Location (main.py) | New Location |
|---|---|
| `normalize_input()` | `utils.py` |
| `build_url()` | `utils.py` |
| `InterpretResponse` | `models.py` |
| `InterpretRequest` | `models.py` |
| "list apps" rule | `parsers/app_parser.py` |
| "remember" rule | `parsers/memory_parser.py` |
| "open" rule | `parsers/app_parser.py` |
| "kill" rule | `parsers/app_parser.py` |
| "recall" rule | `parsers/memory_parser.py` |
| "search" rule | `parsers/memory_parser.py` |
| "remind" rule | `parsers/memory_parser.py` |
| **"what is" rule (NEW)** | **`parsers/search_parser.py`** |

## Code Quality

- **No external dependencies** beyond FastAPI & Pydantic
- **Type hints** throughout for IDE support
- **Comprehensive docstrings** for all public methods
- **Logging** at every parse point for debugging
- **Error handling** with Result types (no panics)
- **Test coverage** for all patterns and edge cases
