# Noddy Brain - Quick Start Guide

## File Structure

```
brain/
├── main.py                 ← Start here (FastAPI app)
├── config.py              ← Logging setup
├── models.py              ← API request/response models
├── utils.py               ← Helper functions
├── parsers/
│   ├── __init__.py        ← Parser registry (orchestration)
│   ├── base.py            ← BaseParser interface
│   ├── app_parser.py      ← "open", "list", "kill" commands
│   ├── memory_parser.py   ← "remember", "recall", "search", "remind"
│   └── search_parser.py   ← "what is", "search about" (NEW!)
└── test_parser.py         ← Comprehensive test suite
```

## Running the Brain

### Start the API Server

```bash
cd brain
python main.py
```

Server starts at `http://127.0.0.1:8000`

### Test a Command

```bash
curl -X POST http://127.0.0.1:8000/interpret \
  -H "Content-Type: application/json" \
  -d '{"text": "what is Python"}'
```

Response:
```json
{
  "action": "search_web",
  "value": "https://www.google.com/search?q=Python",
  "confidence": 1.0
}
```

## Running Tests

```bash
cd brain

# Using pytest (recommended)
python -m pytest test_parser.py -v

# Using built-in test runner
python test_parser.py
```

Expected: **35 passed, 0 failed**

## Understanding the Code

### How Parsing Works

```
User Input
    ↓
Parser Registry (parsers/__init__.py)
    ↓
Try each parser in order:
  1. MemoryParser.can_parse() ?
  2. SearchParser.can_parse() ?
  3. AppParser.can_parse() ?
    ↓
Selected Parser.parse() → InterpretResponse
    ↓
Return to Rust/Frontend
```

### Adding a Command

**Example: Add "list reminders" command**

1. **Update MemoryParser** (`parsers/memory_parser.py`):

```python
def can_parse(self, text: str) -> bool:
    normalized = normalize_input(text)
    if normalized == "list reminders":  # NEW
        return True
    # ... existing checks

def parse(self, text: str) -> InterpretResponse:
    if normalized == "list reminders":  # NEW
        logger.info(f"Parsed: '{text}' → action=list_reminders")
        return InterpretResponse(
            action="list_reminders",
            value="",
            confidence=1.0
        )
    # ... existing parsing
```

2. **Add test** (`test_parser.py`):

```python
def test_list_reminders():
    result = parse_command("list reminders")
    assert result.action == "list_reminders"
```

3. **Run tests**:

```bash
python -m pytest test_parser.py::test_list_reminders -v
```

4. **Update Rust** (`src-tauri/src/lib.rs`):

```rust
"list_reminders" => {
    // Fetch and return reminders from SQLite
    let reminders = /* SQL query */;
    ActionResponse {
        success: true,
        message: format!("Found {} reminders", reminders.len()),
        data: Some(json!(reminders))
    }
}
```

That's it! No main.py changes needed.

## Key Concepts

### can_parse() vs parse()

**can_parse()**
- Input: normalized text (lowercase, stripped)
- Output: `True` if this parser handles it
- Fast string check only

**parse()**
- Input: original text (preserves case)
- Output: `InterpretResponse`
- Extract value, build JSON, etc.

### Parser Priority

Order matters because patterns can overlap:

```python
self.parsers = [
    MemoryParser(),      # Specific patterns first
    SearchParser(),      # Medium specificity
    AppParser(),         # Generic catch-all last
]
```

**Why this order?**
- "search " could be memory_parser or search_parser
- MemoryParser needs to go first and explicitly exclude "search about"
- SearchParser only handles "what is", "google", "look up", etc.
- AppParser handles "open", "kill", "list" that nothing else matches

## Debugging

### Enable Detailed Logging

Logs are auto-printed to console:

```
2026-03-03 19:29:25 - parsers.memory_parser - INFO - Parsed: 'remember buy milk' → action=remember
2026-03-03 19:29:25 - parsers.search_parser - INFO - Parsed: 'what is Python' → action=search_web
```

### Print Which Parser Matched

Each parser logs its successful parse. If something isn't working:

```bash
python test_parser.py | grep "your command"
```

### Test a Specific Command

```python
# In test_parser.py or interactive Python
from parsers import parse_command

result = parse_command("what is machine learning")
print(f"Action: {result.action}")
print(f"Value: {result.value}")
```

## Common Patterns

### Web Search Patterns

```python
"what is X"                    → search_web
"what's X"                     → search_web
"search about X in web"        → search_web
"search for X on google"       → search_web
"google X"                     → search_web
"look up X"                    → search_web
```

### Memory Patterns

```python
"remember X"                   → remember
"recall"                       → recall_memory
"search X"                     → search_memory (note: not web search!)
"remind me to X in Y"          → set_reminder
```

### App Control Patterns

```python
"open X"                       → open_app
"open https://X"               → open_url
"open X in web"                → open_url
"list apps"                    → list_apps
"kill X"                       → kill_process
```

## Future: LLM Integration

The SearchParser is ready for LLM:

```python
class SearchParser(BaseParser):
    LLM_AVAILABLE = False  # Toggle when LLM is ready
    
    def parse(self, text: str) -> InterpretResponse:
        # When LLM is available, return llm_query instead of search_web
        if self.LLM_AVAILABLE:
            return InterpretResponse(action="llm_query", value=query)
```

When LLM arrives:
1. Set `LLM_AVAILABLE = True`
2. Rust handler uses LLM API
3. Return intelligent answers instead of URLs
4. No refactoring of parsers needed!

## Performance Tips

- Parsing takes ~1-2ms (string matching only)
- DNS resolution for "open X in web" takes 50-200ms (cached by OS)
- Startup instant (<100ms)
- No external dependencies except FastAPI

## Troubleshooting

### Tests failing?

```bash
# Check what's wrong
python -m pytest test_parser.py -v

# Run a specific test with output
python -c "from test_parser import *; test_what_is_query()"
```

### Brain API not starting?

```
Error: Port 8000 already in use
```

Kill existing process:
```bash
# PowerShell
Get-Process | Where-Object {$_.Port -eq 8000} | Stop-Process

# Or use a different port
python main.py --port 8001
```

### Rust compilation failures?

```bash
cd src-tauri
cargo check  # Verify compilation
```

If search_web handler fails, check quotes and syntax in lib.rs.

## Resources

- **Architecture details**: See `ARCHITECTURE.md`
- **Refactoring history**: See `REFACTORING_SUMMARY.md`
- **Test examples**: See `test_parser.py`
- **API docs**: Visit `http://127.0.0.1:8000/docs` (Swagger UI)

---

**Happy coding! 🚀**

If you have questions about extending Noddy, check `ARCHITECTURE.md` first - it covers common scenarios for adding new parsers, LLM integration, and more.
