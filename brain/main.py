"""
Noddy Brain Layer - Modular Intent-Based Architecture
FastAPI application for intent interpretation using modular parsers.

Architecture:
- config.py: Logging and configuration
- models.py: Pydantic request/response models
- utils.py: Helper functions (normalization, URL building)
- domain/: Domain models
  - intent.py: Intent dataclass (internal structured representation)
- parsers/: Modular parser system
  - base.py: BaseParser interface
  - app_parser.py: Application control (open, list, kill)
  - memory_parser.py: Memory system (remember, recall, search, remind)
  - search_parser.py: Web search (what is, search about) [NEW]
  - __init__.py: Parser registry and orchestration
"""

import json
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from models import InterpretRequest, InterpretResponse
from domain import Intent
from parsers import parse_command
from config import get_logger

# Initialize logger
logger = get_logger(__name__)


def intent_to_response(intent: Intent) -> InterpretResponse:
    """
    Convert internal Intent domain model to external API response.
    
    This maintains backward compatibility with the existing API contract
    while using structured Intent objects internally.
    
    Args:
        intent: Internal Intent object
    
    Returns:
        InterpretResponse for API consumption
    
    Conversion Rules:
        - remember: payload["content"] → value
        - search_memory: payload["keyword"] → value
        - set_reminder: entire payload as JSON → value
        - search_web: payload["url"] → value
        - open_url: payload["url"] → value
        - open_app: payload["target"] → value
        - kill_process: payload["process"] → value
        - list_apps: "" → value
        - recall_memory: "" → value
        - unknown: payload["text"] → value
    """
    name = intent.name
    payload = intent.payload
    
    # Extract appropriate value based on intent name
    if name == "remember":
        value = payload.get("content", "")
    elif name == "search_memory":
        value = payload.get("keyword", "")
    elif name == "set_reminder":
        # Keep full JSON for set_reminder (includes content + trigger_at)
        value = json.dumps(payload)
    elif name == "search_web":
        # Return URL for frontend to open
        value = payload.get("url", "")
    elif name == "open_url":
        value = payload.get("url", "")
    elif name == "open_app":
        value = payload.get("target", "")
    elif name == "kill_process":
        value = payload.get("process", "")
    elif name in ("list_apps", "recall_memory", "list_calendar_events"):
        value = json.dumps(payload)
    elif name == "unknown":
        value = payload.get("text", "")
    else:
        # Fallback: serialize entire payload
        value = json.dumps(payload)
    
    return InterpretResponse(
        action=name,
        value=value,
        confidence=intent.confidence
    )


# Initialize FastAPI app
app = FastAPI(
    title="Noddy Brain",
    description="Modular intent interpreter for Noddy desktop assistant",
    version="0.2.0"
)

# Add CORS middleware to allow requests from Tauri frontend
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:1420", "http://127.0.0.1:1420"],  # Tauri dev server
    allow_credentials=True,
    allow_methods=["*"],  # Allow all HTTP methods (GET, POST, OPTIONS, etc.)
    allow_headers=["*"],  # Allow all headers
)


@app.post("/interpret", response_model=InterpretResponse)
async def interpret(request: InterpretRequest) -> InterpretResponse:
    """
    Interpret user text and return structured action.
    
    Supported Commands:
    
    App Control:
    - "open chrome" → {"action": "open_app", "value": "chrome"}
    - "open https://example.com" → {"action": "open_url", "value": "https://example.com"}
    - "open youtube in web" → {"action": "open_url", "value": "https://www.youtube.com"}
    - "kill notepad.exe" → {"action": "kill_process", "value": "notepad.exe"}
    - "list apps" → {"action": "list_apps", "value": ""}
    
    Memory System:
    - "remember buy milk" → {"action": "remember", "value": "buy milk"}
    - "recall" → {"action": "recall_memory", "value": ""}
    - "search meeting" → {"action": "search_memory", "value": "meeting"}
    - "remind me to call mom in 30 minutes" → {"action": "set_reminder", "value": "{...}"}
    
    Web Search (NEW):
    - "what is Python" → {"action": "search_web", "value": "https://www.google.com/search?q=Python"}
    - "search about AI in web" → {"action": "search_web", "value": "https://www.google.com/search?q=AI"}
    - "google machine learning" → {"action": "search_web", "value": "https://www.google.com/search?q=machine+learning"}
    
    Note: Web search queries will be handled by LLM in future versions.
    
    Internal Architecture:
    1. parse_command(text) → Intent (structured domain model)
    2. intent_to_response(intent) → InterpretResponse (API response)
    
    This separation allows for:
    - Clean domain modeling internally
    - Backward-compatible API externally
    - Future LLM integration without breaking changes
    """
    # Validate input
    if not request.text or not request.text.strip():
        logger.warning("Received empty text")
        raise HTTPException(status_code=400, detail="Text cannot be empty")
    
    # Log incoming request
    logger.info(f"Received: '{request.text}'")
    
    # Parse to Intent (internal domain model)
    intent = parse_command(request.text)
    
    # Convert Intent → InterpretResponse (external API)
    response = intent_to_response(intent)
    
    return response


@app.get("/health")
async def health_check():
    """Health check endpoint"""
    return {"status": "ok", "version": "0.2.0"}


@app.get("/system/volume")
async def get_system_volume():
    """Get exact master volume level via pycaw."""
    try:
        from pycaw.pycaw import AudioUtilities
        ev = AudioUtilities.GetSpeakers().EndpointVolume
        level = int(ev.GetMasterVolumeLevelScalar() * 100)
        return {"status": "success", "level": level}
    except Exception as e:
        logger.error(f"Failed to read volume: {e}")
        return {"status": "error", "message": str(e)}


@app.get("/")
async def root():
    """Root endpoint with API info"""
    return {
        "name": "Noddy Brain",
        "version": "0.2.0",
        "architecture": "Modular Intent-Based",
        "description": "Intent interpreter for Noddy desktop assistant with modular parser system",
        "parsers": [
            "MemoryParser (remember, recall, search, remind)",
            "SearchParser (what is, search about - Google/LLM)",
            "AppParser (open, list, kill)"
        ],
        "endpoints": {
            "POST /interpret": "Convert user text to structured action",
            "GET /health": "Health check",
            "GET /docs": "Interactive API docs (Swagger UI)",
            "GET /redoc": "Alternative API docs (ReDoc)"
        }
    }


if __name__ == "__main__":
    import uvicorn
    logger.info("Starting Noddy Brain Layer v0.2.0 with modular architecture")
    uvicorn.run(app, host="127.0.0.1", port=8000)
