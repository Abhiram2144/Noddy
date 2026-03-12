"""
Parser registry and orchestration for Noddy Brain Layer.

This module manages all parsers and determines which parser
should handle each incoming command.

Architecture: Core parsers are always tried first (priority order),
then plugin parsers are attempted. This ensures deterministic behavior
while allowing extensibility.
"""

import sys
from pathlib import Path
from typing import List
import pkgutil
import importlib

from parsers.base import BaseParser
from parsers.memory_parser import MemoryParser
from parsers.search_parser import SearchParser
from parsers.app_parser import AppParser
from parsers.calendar_parser import CalendarParser
from parsers.system_parser import SystemParser
from parsers.llm_parser import LLMParser
from domain import Intent
from utils import normalize_input
from config import get_logger

logger = get_logger(__name__)


class ParserRegistry:
    """
    Registry of all available parsers (core + plugins).
    
    Core Parsers (tried in order - deterministic priority):
    1. MemoryParser - Specific memory/reminder commands
    2. SearchParser - Web search and information lookup
    3. AppParser - Application control (catch-all for "open", "kill", etc.)
    
    Plugin Parsers (loaded dynamically from parsers/plugins/):
    - Any parser that inherits from BaseParser in the plugins directory
    - Tried after core parsers
    
    Order matters: More specific parsers should come before general ones.
    Plugin system allows community contributions without modifying core.
    """
    
    def __init__(self):
        # Core parsers - always loaded, always in this priority order
        self.core_parsers: List[BaseParser] = [
            MemoryParser(),
            CalendarParser(),
            SystemParser(),
            SearchParser(),
            AppParser(),
            LLMParser(),
        ]
        
        # Plugin parsers - loaded dynamically
        self.plugin_parsers: List[BaseParser] = []
        
        # Load plugins
        self._load_plugins()
        
        # Combined list: core first (deterministic), then plugins
        self.parsers = self.core_parsers + self.plugin_parsers
    
    def _load_plugins(self) -> None:
        """
        Dynamically load parsers from parsers/plugins/ directory.
        
        Each Python module in the plugins directory is searched for classes
        that inherit from BaseParser. Gracefully skips errors.
        
        Future:
        - Enable/disable parsers via configuration
        - Version compatibility checks
        - Plugin metadata validation
        """
        plugins_dir = Path(__file__).parent / "plugins"
        
        if not plugins_dir.exists():
            logger.debug("No plugins directory found")
            return
        
        # Add plugins directory to sys.path if not already there
        plugins_path_str = str(plugins_dir.parent)
        if plugins_path_str not in sys.path:
            sys.path.insert(0, plugins_path_str)
        
        try:
            # Import parsers.plugins package to make it discoverable
            import parsers.plugins
            
            # Discover all modules in the plugins package
            for importer, modname, ispkg in pkgutil.iter_modules(
                parsers.plugins.__path__,
                parsers.plugins.__name__ + "."
            ):
                try:
                    # Dynamically import the module
                    module = importlib.import_module(modname)
                    
                    # Look for BaseParser subclasses in the module
                    for attr_name in dir(module):
                        attr = getattr(module, attr_name)
                        
                        # Check if it's a class and inherits from BaseParser
                        try:
                            if (isinstance(attr, type) and 
                                issubclass(attr, BaseParser) and 
                                attr is not BaseParser):
                                # Instantiate and register the parser
                                parser_instance = attr()
                                self.plugin_parsers.append(parser_instance)
                                logger.info(f"✓ Loaded plugin parser: {modname}.{attr_name}")
                        except (TypeError, AttributeError):
                            # Not a valid parser class, skip
                            pass
                
                except Exception as e:
                    logger.warning(f"⚠️  Failed to load plugin {modname}: {e}")
                    # Continue loading other plugins if one fails
                    continue
        
        except ImportError:
            # plugins package not properly set up
            logger.debug("Could not import parsers.plugins")
    
    def parse(self, text: str) -> Intent:
        """
        Parse input text using the first matching parser.
        
        Core parsers are tried first (deterministic), then plugins.
        
        Args:
            text: User input text
        
        Returns:
            Intent with name, payload, confidence, and source
        """
        normalized = normalize_input(text)
        
        # Try each parser in order (core first, then plugins)
        for parser in self.parsers:
            # Skip LLMParser for now, we'll use it as a final fallback
            if isinstance(parser, LLMParser):
                continue
                
            if parser.can_parse(normalized):
                intent = parser.parse(text)
                # If we have a high-confidence non-unknown intent, use it
                if intent.name != "unknown" and intent.confidence >= 0.8:
                    return intent
        
        # If no high-confidence rule-based intent found, use LLMParser
        llm_parser = next((p for p in self.parsers if isinstance(p, LLMParser)), None)
        if llm_parser:
            logger.info(f"Falling back to dynamic LLM interpretation for: '{text}' (LLMParser found)")
            return llm_parser.parse(text)
        else:
            logger.warning(f"LLMParser NOT FOUND in parsers list! Current parsers: {[type(p).__name__ for p in self.parsers]}")
        
        # No parser matched - return unknown intent
        logger.info(f"Parsed: '{text}' → action=unknown (no parser matched)")
        return Intent(
            name="unknown",
            payload={"text": text},
            confidence=1.0
        )


# Global parser registry instance
parser_registry = ParserRegistry()


def parse_command(text: str) -> Intent:
    """
    Main entry point for parsing commands.
    
    Args:
        text: User input text
    
    Returns:
        Intent with name, payload, confidence, and source
    """
    return parser_registry.parse(text)
