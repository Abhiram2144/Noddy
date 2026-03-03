"""
Base parser interface for Noddy Brain Layer.
"""

from abc import ABC, abstractmethod
from domain import Intent


class BaseParser(ABC):
    """
    Abstract base class for all parsers.
    
    Each parser implements:
    - can_parse(text: str) -> bool: Check if this parser can handle the input
    - parse(text: str) -> Intent: Parse the input and return structured intent
    """
    
    @abstractmethod
    def can_parse(self, text: str) -> bool:
        """
        Check if this parser can handle the given input.
        
        Args:
            text: Normalized input text (lowercase, stripped)
        
        Returns:
            True if this parser should handle the input
        """
        pass
    
    @abstractmethod
    def parse(self, text: str) -> Intent:
        """
        Parse the input and return structured intent.
        
        Args:
            text: Original input text (preserves case)
        
        Returns:
            Intent with name, payload, confidence, and source
        """
        pass
