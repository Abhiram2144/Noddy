"""
Noddy Brain Layer - Phase 2
Deterministic intent interpretation using rule-based parsing.
"""

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import Optional
import logging
import socket

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Initialize FastAPI app
app = FastAPI(
    title="Noddy Brain",
    description="Intent interpreter for Noddy desktop assistant",
    version="0.1.0"
)

# Pydantic models
class InterpretRequest(BaseModel):
    """Request model for interpret endpoint"""
    text: str


class InterpretResponse(BaseModel):
    """Response model for interpret endpoint"""
    action: str
    value: str
    confidence: float = 1.0  # Rule-based = deterministic = 100% confidence


def normalize_input(text: str) -> str:
    """Normalize input: strip whitespace and convert to lowercase"""
    return text.strip().lower()


def build_url(value: str) -> str:
    """
    Build full URL from search term using intelligent domain resolution.
    
    Strategy:
    1. Try common domain patterns (www.X.com, X.com)
    2. Use DNS lookup to verify domain exists
    3. Return the first valid domain
    4. Fall back to https://www.{value}.com if none resolve
    """
    value = value.lower().strip()
    
    # Create list of domain candidates to try
    candidates = [
        f"www.{value}.com",
        f"{value}.com",
        f"www.{value}.io",
        f"{value}.io",
    ]
    
    # Try each candidate via DNS resolution
    for domain in candidates:
        try:
            socket.gethostbyname(domain)
            logger.info(f"Domain resolved: {domain}")
            return f"https://{domain}"
        except (socket.gaierror, socket.error):
            # Domain doesn't resolve, try next candidate
            continue
    
    # If no domain resolves, use fallback pattern
    fallback_url = f"https://www.{value}.com"
    logger.info(f"No domain resolved, using fallback: {fallback_url}")
    return fallback_url


def parse_command(text: str) -> InterpretResponse:
    """
    Parse command text and return structured action.
    
    Rules:
    1. "open <value>" → check if value is URL or app
    2. "open <value> in web" → build URL and open in browser
    3. "kill <process>" → kill process
    4. "list apps" → list installed applications
    5. Otherwise → unknown action
    """
    normalized = normalize_input(text)
    
    # Rule 1: "list apps"
    if normalized == "list apps":
        logger.info(f"Parsed: '{text}' → action=list_apps")
        return InterpretResponse(
            action="list_apps",
            value="",
            confidence=1.0
        )
    
    # Rule 2: "open ..."
    if normalized.startswith("open "):
        value = text[5:].strip()  # Preserve original case for value
        
        # Sub-rule: "open <X> in web" → open in browser
        in_web_match = value.lower()
        if " in web" in in_web_match:
            search_term = value[:value.lower().rfind(" in web")].strip()
            url = build_url(search_term.lower())
            logger.info(f"Parsed: '{text}' → action=open_url, value={url}")
            return InterpretResponse(
                action="open_url",
                value=url,
                confidence=1.0
            )
        
        # Sub-rule: value is already a URL
        if value.startswith("http://") or value.startswith("https://"):
            logger.info(f"Parsed: '{text}' → action=open_url, value={value}")
            return InterpretResponse(
                action="open_url",
                value=value,
                confidence=1.0
            )
        
        # Sub-rule: value is an app name
        logger.info(f"Parsed: '{text}' → action=open_app, value={value}")
        return InterpretResponse(
            action="open_app",
            value=value,
            confidence=1.0
        )
    
    # Rule 3: "kill ..."
    if normalized.startswith("kill "):
        value = text[5:].strip()
        logger.info(f"Parsed: '{text}' → action=kill_process, value={value}")
        return InterpretResponse(
            action="kill_process",
            value=value,
            confidence=1.0
        )
    
    # Rule 4: Unknown command
    logger.info(f"Parsed: '{text}' → action=unknown")
    return InterpretResponse(
        action="unknown",
        value=text,
        confidence=1.0
    )


@app.post("/interpret", response_model=InterpretResponse)
async def interpret(request: InterpretRequest) -> InterpretResponse:
    """
    Interpret user text and return structured action.
    
    Examples:
    - "open chrome" → {"action": "open_app", "value": "chrome"}
    - "open https://example.com" → {"action": "open_url", "value": "https://example.com"}
    - "open youtube in web" → {"action": "open_url", "value": "https://www.youtube.com"}
    - "kill notepad.exe" → {"action": "kill_process", "value": "notepad.exe"}
    - "list apps" → {"action": "list_apps", "value": ""}
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
    return {"status": "ok"}


@app.get("/")
async def root():
    """Root endpoint with API info"""
    return {
        "name": "Noddy Brain",
        "version": "0.1.0",
        "description": "Intent interpreter for Noddy desktop assistant",
        "endpoints": {
            "POST /interpret": "Convert user text to structured action",
            "GET /health": "Health check",
            "GET /docs": "Interactive API docs (Swagger UI)",
            "GET /redoc": "Alternative API docs (ReDoc)"
        }
    }


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="127.0.0.1", port=8000)
