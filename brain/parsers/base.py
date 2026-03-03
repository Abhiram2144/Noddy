"""
Base parser interface for Noddy Brain Layer.
"""

from abc import ABC, abstractmethod
from models import InterpretResponse


class BaseParser(ABC):
    """
    Abstract base class for all parsers.
    
    Each parser implements:
    - can_parse(text: str) -> bool: Check if this parser can handle the input
    - parse(text: str) -> InterpretResponse: Parse the input and return structured response
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
    def parse(self, text: str) -> InterpretResponse:
        """
        Parse the input and return structured response.
        
        Args:
            text: Original input text (preserves case)
        
        Returns:
            InterpretResponse with action, value, and confidence
        """
        pass
