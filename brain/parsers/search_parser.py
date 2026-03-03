"""
Web search parser: handles information lookup queries.

This parser handles queries that should be searched on Google.
In the future, these queries can be delegated to an LLM for more intelligent responses.
"""

from parsers.base import BaseParser
from domain import Intent
from utils import normalize_input, build_google_search_url
from config import get_logger

logger = get_logger(__name__)


class SearchParser(BaseParser):
    """
    Handles web search commands:
    - "what is <X>" / "what's <X>" → Google search
    - "search about <X> in web" / "search about <X> on web" → Google search
    - "search for <X> on google" / "search for <X> in google" → Google search
    - "google <X>" → Google search
    
    Future: These queries will be handled by LLM when available.
    The system will check for LLM availability and fall back to Google search.
    """
    
    # LLM availability flag (future implementation)
    LLM_AVAILABLE = False
    
    def can_parse(self, text: str) -> bool:
        """Check if input is a web search query."""
        normalized = normalize_input(text)
        return (
            normalized.startswith("what is ")
            or normalized.startswith("what's ")
            or normalized.startswith("search about ")
            or normalized.startswith("search for ")
            or normalized.startswith("google ")
            or normalized.startswith("look up ")
            or normalized.startswith("find information about ")
        )
    
    def parse(self, text: str) -> Intent:
        """
        Parse web search command.
        
        Future LLM Integration:
        When LLM_AVAILABLE is True, return Intent with name="llm_query" instead of "search_web"
        For now, all queries go to Google search.
        """
        normalized = normalize_input(text)
        query = None
        
        # Extract query from different patterns
        if normalized.startswith("what is "):
            query = text[8:].strip()
        elif normalized.startswith("what's "):
            query = text[7:].strip()
        elif normalized.startswith("search about "):
            # Remove trailing "in web", "on web", "in google", "on google"
            query_part = text[13:].strip()
            for suffix in [" in web", " on web", " in google", " on google"]:
                if query_part.lower().endswith(suffix):
                    query = query_part[:-len(suffix)].strip()
                    break
            if not query:
                query = query_part
        elif normalized.startswith("search for "):
            # Remove trailing "on google", "in google", "on web", "in web"
            query_part = text[11:].strip()
            for suffix in [" on google", " in google", " on web", " in web"]:
                if query_part.lower().endswith(suffix):
                    query = query_part[:-len(suffix)].strip()
                    break
            if not query:
                query = query_part
        elif normalized.startswith("google "):
            query = text[7:].strip()
        elif normalized.startswith("look up "):
            query = text[8:].strip()
        elif normalized.startswith("find information about "):
            query = text[23:].strip()
        
        if not query:
            raise ValueError(f"SearchParser cannot extract query from: {text}")
        
        # Future: Check LLM availability
        # if self.LLM_AVAILABLE:
        #     logger.info(f"Parsed: '{text}' → action=llm_query, value={query}")
        #     return Intent(
        #         name="llm_query",
        #         payload={"query": query},
        #         confidence=1.0,
        #         source="llm"
        #     )
        
        # Current: Use Google search
        search_url = build_google_search_url(query)
        logger.info(f"Parsed: '{text}' → action=search_web, value={search_url}")
        logger.info(f"💡 Future: This query '{query}' will be handled by LLM")
        
        return Intent(
            name="search_web",
            payload={"query": query, "url": search_url},
            confidence=1.0
        )
