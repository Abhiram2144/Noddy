"""
Noddy Brain Layer - Modular Intent-Based Architecture
FastAPI application for intent interpretation using modular parsers.

Architecture:
- config.py: Logging and configuration
- models.py: Pydantic request/response models
- utils.py: Helper functions (normalization, URL building)
- parsers/: Modular parser system
  - base.py: BaseParser interface
  - app_parser.py: Application control (open, list, kill)
  - memory_parser.py: Memory system (remember, recall, search, remind)
  - search_parser.py: Web search (what is, search about) [NEW]
  - __init__.py: Parser registry and orchestration
"""

from fastapi import FastAPI, HTTPException
from models import InterpretRequest, InterpretResponse
from parsers import parse_command
from config import get_logger

# Initialize logger
logger = get_logger(__name__)

# Initialize FastAPI app
app = FastAPI(
    title="Noddy Brain",
    description="Modular intent interpreter for Noddy desktop assistant",
    version="0.2.0"
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
    """
    # Validate input
    if not request.text or not request.text.strip():
        logger.warning("Received empty text")
        raise HTTPException(status_code=400, detail="Text cannot be empty")
    
    # Log incoming request
    logger.info(f"Received: '{request.text}'")
    
    # Parse and return response
    response = parse_command(request.text)
    return response


@app.get("/health")
async def health_check():
    """Health check endpoint"""
    return {"status": "ok", "version": "0.2.0"}


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
