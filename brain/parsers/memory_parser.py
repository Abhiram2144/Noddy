"""
Memory system parser: handles remember, recall, search, and reminder commands.
"""

import json
from datetime import datetime, timedelta
from parsers.base import BaseParser
from domain import Intent
from utils import normalize_input
from config import get_logger

logger = get_logger(__name__)


class MemoryParser(BaseParser):
    """
    Handles memory system commands:
    - "remember <X>" → save memory
    - "recall" / "what do you remember" → retrieve memories
    - "search <keyword>" → search memories by keyword
    - "remind me to <X> in <time>" → set reminder with expiration
    """
    
    def can_parse(self, text: str) -> bool:
        """Check if input is memory-related command."""
        normalized = normalize_input(text)
        
        # Check for exact memory commands
        if normalized.startswith("remember "):
            return True
        if normalized in ["recall", "what do you remember", "what do you remember?", 
                         "recall memory", "recall memories", "show memories"]:
            return True
        if normalized.startswith("remind me to "):
            return True
        
        # Check for memory search: "search <keyword>"
        # But NOT "search about", "search for" (those are web searches)
        if normalized.startswith("search "):
            # Make sure it's not a web search pattern
            if not (normalized.startswith("search about ") or 
                   normalized.startswith("search for ")):
                return True
        
        return False
    
    def parse(self, text: str) -> Intent:
        """Parse memory command."""
        normalized = normalize_input(text)
        
        # "remember ..."
        if normalized.startswith("remember "):
            value = text[9:].strip()  # Preserve original case
            logger.info(f"Parsed: '{text}' → action=remember, value={value}")
            return Intent(
                name="remember",
                payload={"content": value},
                confidence=1.0
            )
        
        # "recall" or variants
        if normalized in ["recall", "what do you remember", "what do you remember?", 
                          "recall memory", "recall memories", "show memories"]:
            logger.info(f"Parsed: '{text}' → action=recall_memory")
            return Intent(
                name="recall_memory",
                payload={},
                confidence=1.0
            )
        
        # "search <keyword>"
        if normalized.startswith("search "):
            keyword = text[7:].strip()
            logger.info(f"Parsed: '{text}' → action=search_memory, value={keyword}")
            return Intent(
                name="search_memory",
                payload={"keyword": keyword},
                confidence=1.0
            )
        
        # "remind me to <content> in <time>"
        if normalized.startswith("remind me to "):
            content_part = text[13:].strip()  # Remove "remind me to "
            
            # Parse time specification
            if " in " in content_part.lower():
                parts = content_part.rsplit(" in ", 1)
                if len(parts) == 2:
                    content = parts[0].strip()
                    time_spec = parts[1].strip().lower()
                    
                    # Parse time specification (minutes, hours, days)
                    trigger_at = None
                    if "minute" in time_spec:
                        try:
                            minutes = int(''.join(filter(str.isdigit, time_spec)))
                            trigger_at = int((datetime.now() + timedelta(minutes=minutes)).timestamp())
                        except ValueError:
                            pass
                    elif "hour" in time_spec:
                        try:
                            hours = int(''.join(filter(str.isdigit, time_spec)))
                            trigger_at = int((datetime.now() + timedelta(hours=hours)).timestamp())
                        except ValueError:
                            pass
                    elif "day" in time_spec:
                        try:
                            days = int(''.join(filter(str.isdigit, time_spec)))
                            trigger_at = int((datetime.now() + timedelta(days=days)).timestamp())
                        except ValueError:
                            pass
                    
                    if trigger_at:
                        logger.info(f"Parsed: '{text}' → action=set_reminder, trigger_at={trigger_at}")
                        return Intent(
                            name="set_reminder",
                            payload={
                                "content": content,
                                "trigger_at": trigger_at
                            },
                            confidence=1.0
                        )
        
        # Should never reach here if can_parse is correct
        raise ValueError(f"MemoryParser cannot parse: {text}")
