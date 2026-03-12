"""
Calendar parser: handles requests to view schedule or calendar events.
"""

from parsers.base import BaseParser
from domain import Intent
from utils import normalize_input
from config import get_logger

logger = get_logger(__name__)


class CalendarParser(BaseParser):
    """
    Handles calendar-related commands:
    - "what is my schedule today"
    - "show my schedule"
    - "fetch my calendar events"
    - "show my outlook calendar"
    """
    
    def can_parse(self, text: str) -> bool:
        """Check if input is a calendar query."""
        normalized = normalize_input(text)
        return (
            "schedule" in normalized
            or "calendar" in normalized
            or "events" in normalized
            or "meeting" in normalized
            or "appointment" in normalized
            or ("add" in normalized and ("event" in normalized or "meeting" in normalized or "calendar" in normalized))
        )
    
    def parse(self, text: str) -> Intent:
        """Parse calendar command."""
        import re
        from datetime import datetime, timedelta
        
        normalized = normalize_input(text)
        
        # Check for Delete/Cancel FIRST (highest priority if specific keywords are found)
        if any(word in normalized for word in ["delete", "cancel", "remove", "clear"]):
            # Extract what to delete - try to be specific
            query = text
            # Try to extract title from text (preserve casing/quotes)
            # Handle "the", "my", "your", "a", "an" as fillers
            delete_match = re.search(r"(?:delete|cancel|remove|clear)\s+(?:the|my|your|a|an)?\s*(?:meeting|event|appointment)?\s*(?:called|named|with\s+title)?\s*['\"]?(.*?)['\"]?(?:\s+(?:meeting|event|appointment|at|on|for)|$)", text, re.IGNORECASE)
            if delete_match:
                extracted = delete_match.group(1).strip()
                if extracted: query = extracted

            return Intent(
                name="delete_calendar_event",
                payload={"query": query, "original_text": text},
                confidence=0.9
            )

        # Check for Update/Reschedule
        if any(word in normalized for word in ["reschedule", "change", "update", "move"]):
            return Intent(
                name="update_calendar_event",
                payload={"original_text": text},
                confidence=0.85
            )

        # Check if it's a "create" request 
        # Avoid "schedule" unless paired with creation words like "new" or "add"
        create_triggers = ["add", "create", "new meeting", "new event", "put", "schedule a", "schedule an", "schedule new"]
        is_create = any(trigger in normalized for trigger in create_triggers)
        
        if is_create:
            # 1. Extract Title from raw text
            title = "New Meeting"
            title_patterns = [
                r"(?:meeting|event|appointment|title)(?:\s+(?:called|with\s+title|named))?\s+['\"]?(.+?)['\"]?(?:\s+for|\s+at|\s+tomorrow|\s+next|\s+today|$)",
                r"(?:create|add|schedule)\s+(?:a|an|the|my|your)?\s*['\"]?(.+?)['\"]?\s+(?:meeting|event|appointment)",
                r"add\s+['\"]?(.+?)['\"]?\s+to\s+my\s+calendar"
            ]
            for pattern in title_patterns:
                match = re.search(pattern, text, re.IGNORECASE)
                if match:
                    extracted = match.group(1).strip()
                    if extracted and len(extracted) < 100:
                        title = extracted
                        break
            
            # Simple date extraction
            start_time = "next_hour"
            if "tomorrow" in normalized:
                start_time = "tomorrow"
            
            # Simple duration extraction
            duration = 60
            if "2 hours" in normalized: duration = 120
            elif "30 minutes" in normalized: duration = 30
            
            return Intent(
                name="create_calendar_event",
                payload={
                    "original_text": text,
                    "title": title, # Keep parsed casing
                    "start_time": start_time,
                    "duration_minutes": duration
                },
                confidence=0.8
            )

        # Default to "list_calendar_events" (Read)
        time_period = "today"
        if "tomorrow" in normalized:
            time_period = "tomorrow"
        elif "week" in normalized:
            time_period = "this_week"
            
        logger.info(f"Parsed: '{text}' → action=list_calendar_events, period={time_period}")
        
        return Intent(
            name="list_calendar_events",
            payload={"period": time_period, "original_text": text},
            confidence=0.9
        )
