# Noddy Brain - Phase 2

Deterministic intent interpretation server for Noddy desktop assistant.

## Quick Start

### 1. Install Dependencies
```bash
cd brain
pip install -r requirements.txt
```

### 2. Run Server
```bash
python main.py
```

Server will listen on `http://127.0.0.1:8000`

### 3. Access API

**Interactive Docs:** http://127.0.0.1:8000/docs

**Health Check:**
```bash
curl http://127.0.0.1:8000/health
```

## API Endpoint

### POST /interpret

Converts user text into structured actions.

**Example Request:**
```bash
curl -X POST http://127.0.0.1:8000/interpret \
  -H "Content-Type: application/json" \
  -d '{"text": "open discord"}'
```

**Example Response:**
```json
{
  "action": "open_app",
  "value": "discord",
  "confidence": 1.0
}
```

## Supported Commands

| Command | Action | Output |
|---------|--------|--------|
| `open chrome` | Open app | `{"action": "open_app", "value": "chrome"}` |
| `open https://example.com` | Open URL | `{"action": "open_url", "value": "https://example.com"}` |
| `open youtube in web` | Open in browser | `{"action": "open_url", "value": "https://www.youtube.com"}` |
| `kill notepad.exe` | Kill process | `{"action": "kill_process", "value": "notepad.exe"}` |
| `list apps` | List apps | `{"action": "list_apps", "value": ""}` |

## Design Principles

- **Deterministic:** No randomness, no LLMs, no external APIs
- **Minimal:** Only rule-based parsing
- **Stateless:** Each request is independent
- **Testable:** Pure functions, predictable behavior
- **Foundation:** Phase 2 of brain development (future: NLP, context, memory)

## Rules

1. Input is normalized: stripped and lowercased
2. Commands are parsed in order: list_apps → open → kill → unknown
3. URL detection happens before app detection
4. Special handling for "in web" pattern
5. Empty text returns HTTP 400

## Architecture

```
Frontend (React)
     ↓
  [HTTP Request]
     ↓
Brain Server (Python/FastAPI)
     ↓
Intent Parser (Rule-based)
     ↓
  [Structured JSON]
     ↓
Rust Backend (Tauri)
     ↓
    [Execute]
```

## Future Enhancements

- Natural Language Processing (Phase 3)
- Fuzzy command matching
- User preferences/shortcuts
- Command history
- Context awareness
