"""
Intent domain model for Noddy Brain Layer.

An Intent represents a structured interpretation of user input,
decoupled from the external API response format.
"""

from dataclasses import dataclass
from typing import Dict, Any


@dataclass
class Intent:
    """
    Structured representation of user intent.
    
    Attributes:
        name: The intent name (e.g., "remember", "search_web", "open_app")
        payload: Structured data specific to the intent
        confidence: Confidence score (0.0 to 1.0)
        source: Source of the intent detection ("rule" or "llm")
    
    Examples:
        >>> Intent(name="remember", payload={"content": "buy milk"})
        >>> Intent(name="search_web", payload={"query": "Python", "url": "https://..."})
        >>> Intent(name="open_app", payload={"target": "chrome"})
    """
    name: str
    payload: Dict[str, Any]
    confidence: float = 1.0
    source: str = "rule"  # future: "llm"
    
    def __post_init__(self):
        """Validate intent fields."""
        if not self.name:
            raise ValueError("Intent name cannot be empty")
        if not isinstance(self.payload, dict):
            raise ValueError("Intent payload must be a dictionary")
        if not 0.0 <= self.confidence <= 1.0:
            raise ValueError("Confidence must be between 0.0 and 1.0")
        if self.source not in ("rule", "llm"):
            raise ValueError("Source must be 'rule' or 'llm'")
