"""
Application control parser: handles open, list, and kill commands.
"""

from parsers.base import BaseParser
from domain import Intent
from utils import normalize_input, build_url
from config import get_logger

logger = get_logger(__name__)


class AppParser(BaseParser):
    """
    Handles application control commands:
    - "list apps" → list installed applications
    - "open <app>" → open application
    - "open <url>" → open URL
    - "open <X> in web" → build URL and open in browser
    - "kill <process>" → kill process
    """
    
    def can_parse(self, text: str) -> bool:
        """Check if input is app-related command."""
        normalized = normalize_input(text)
        return (
            normalized == "list apps"
            or normalized.startswith("open ")
            or normalized.startswith("kill ")
        )
    
    def parse(self, text: str) -> Intent:
        """Parse app control command."""
        normalized = normalize_input(text)
        
        # "list apps"
        if normalized == "list apps":
            logger.info(f"Parsed: '{text}' → action=list_apps")
            return Intent(
                name="list_apps",
                payload={},
                confidence=1.0
            )
        
        # "open ..."
        if normalized.startswith("open "):
            value = text[5:].strip()  # Preserve original case for value
            
            # Sub-rule: "open <X> in web" → open in browser
            in_web_match = value.lower()
            if " in web" in in_web_match:
                search_term = value[:value.lower().rfind(" in web")].strip()
                url = build_url(search_term.lower())
                logger.info(f"Parsed: '{text}' → action=open_url, value={url}")
                return Intent(
                    name="open_url",
                    payload={"url": url, "search_term": search_term},
                    confidence=1.0
                )
            
            # Sub-rule: value is already a URL
            if value.startswith("http://") or value.startswith("https://"):
                logger.info(f"Parsed: '{text}' → action=open_url, value={value}")
                return Intent(
                    name="open_url",
                    payload={"url": value},
                    confidence=1.0
                )
            
            # Sub-rule: value is an app name
            logger.info(f"Parsed: '{text}' → action=open_app, value={value}")
            return Intent(
                name="open_app",
                payload={"target": value},
                confidence=1.0
            )
        
        # "kill ..."
        if normalized.startswith("kill "):
            value = text[5:].strip()
            logger.info(f"Parsed: '{text}' → action=kill_process, value={value}")
            return Intent(
                name="kill_process",
                payload={"process": value},
                confidence=1.0
            )
        
        # Should never reach here if can_parse is correct
        raise ValueError(f"AppParser cannot parse: {text}")
