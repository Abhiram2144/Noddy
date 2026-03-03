"""
Parser registry and orchestration for Noddy Brain Layer.

This module manages all parsers and determines which parser
should handle each incoming command.
"""

from typing import List, Optional
from parsers.base import BaseParser
from parsers.memory_parser import MemoryParser
from parsers.search_parser import SearchParser
from parsers.app_parser import AppParser
from models import InterpretResponse
from utils import normalize_input
from config import get_logger

logger = get_logger(__name__)


class ParserRegistry:
    """
    Registry of all available parsers.
    
    Parsers are tried in order:
    1. MemoryParser - Specific memory/reminder commands
    2. SearchParser - Web search and information lookup
    3. AppParser - Application control (catch-all for "open", "kill", etc.)
    
    Order matters: More specific parsers should come before general ones.
    """
    
    def __init__(self):
        self.parsers: List[BaseParser] = [
            MemoryParser(),
            SearchParser(),
            AppParser(),
        ]
    
    def parse(self, text: str) -> InterpretResponse:
        """
        Parse input text using the first matching parser.
        
        Args:
            text: User input text
        
        Returns:
            InterpretResponse with action, value, and confidence
        
        Raises:
            ValueError: If no parser can handle the input
        """
        normalized = normalize_input(text)
        
        # Try each parser in order
        for parser in self.parsers:
            if parser.can_parse(normalized):
                return parser.parse(text)
        
        # No parser matched - return unknown action
        logger.info(f"Parsed: '{text}' → action=unknown (no parser matched)")
        return InterpretResponse(
            action="unknown",
            value=text,
            confidence=1.0
        )


# Global parser registry instance
parser_registry = ParserRegistry()


def parse_command(text: str) -> InterpretResponse:
    """
    Main entry point for parsing commands.
    
    Args:
        text: User input text
    
    Returns:
        InterpretResponse with action, value, and confidence
    """
    return parser_registry.parse(text)
