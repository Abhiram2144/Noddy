import os
import json
import google.generativeai as genai
from domain import Intent
from parsers.base import BaseParser
from config import get_logger
from dotenv import load_dotenv

logger = get_logger(__name__)

# Load environment variables (API Key)
load_dotenv()
# If not found in current dir, try parent
if not os.getenv("GEMINI_API_KEY"):
    parent_env = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))), '.env')
    load_dotenv(parent_env)

GEMINI_API_KEY = os.getenv("GEMINI_API_KEY")

if GEMINI_API_KEY:
    genai.configure(api_key=GEMINI_API_KEY)
else:
    logger.warning("GEMINI_API_KEY not found in .env")

SYSTEM_PROMPT = """
You are the intent interpreter for Noddy, a desktop assistant.
Your job is to convert user natural language into a structured JSON intent.

AVAILABLE INTENTS:
- open_app: {"target": "app name"}
- open_url: {"url": "full url"}
- get_volume: {}
- set_volume: {"level": 0-100 (optional), "action": "increase"|"decrease"|"mute"|"unmute" (optional)}
- get_brightness: {}
- set_brightness: {"level": 0-100}
- system_control: {"command": "lock"|"shutdown"|"restart"|"sleep"}
- create_calendar_event: {"title": "...", "start_time": "ISO format", "duration_minutes": 60}
- delete_calendar_event: {"query": "event title or part of it"}
- remember: {"content": "text to save"}
- search_web: {"url": "https://www.google.com/search?q=query"}

RULES:
1. Return ONLY a valid JSON object.
2. If the intent is not clear, return {"name": "unknown", "payload": {"text": "..."}}.
3. For queries asking for current volume or brightness, use get_volume or get_brightness.
4. For "increase the light", "brighten screen", etc., use "set_brightness" with an appropriate level (e.g. 80).
5. For volume relative changes, use "action": "increase" or "decrease".
6. Be concise.

STRUCTURE:
{
  "name": "intent_name",
  "payload": { ... },
  "confidence": 0.0-1.0
}
"""

class LLMParser(BaseParser):
    def __init__(self):
        self.model = None
        if GEMINI_API_KEY:
            try:
                self.model = genai.GenerativeModel('gemini-2.5-flash')
            except Exception as e:
                logger.error(f"Failed to initialize Gemini model: {e}")

    def can_parse(self, text: str) -> bool:
        # LLMParser is a catch-all fallback
        return self.model is not None

    def parse(self, text: str) -> Intent:
        if not self.model:
            return Intent(name="unknown", payload={"text": text}, confidence=0.0)
            
        try:
            prompt = f"{SYSTEM_PROMPT}\n\nUSER INPUT: \"{text}\"\nJSON RESPONSE:"
            response = self.model.generate_content(prompt)
            
            # Extract JSON from response (handling potential markdown fences)
            res_text = response.text.strip()
            logger.debug(f"Raw Gemini response for '{text}': {res_text}")
            if res_text.startswith("```json"):
                res_text = res_text[7:-3].strip()
            elif res_text.startswith("```"):
                res_text = res_text[3:-3].strip()
                
            data = json.loads(res_text)
            
            return Intent(
                name=data.get("name", "unknown"),
                payload=data.get("payload", {}),
                confidence=data.get("confidence", 0.9)
            )
        except Exception as e:
            logger.error(f"Gemini parsing failed: {e}")
            return Intent(name="unknown", payload={"text": text}, confidence=0.0)
