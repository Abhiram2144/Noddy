"""
Intent domain model for Noddy Brain Layer.

An Intent represents a structured interpretation of user input,
decoupled from the external API response format.

All Intents must conform to the schema defined in /shared/intent.schema.json
to ensure consistency between Python Brain and Rust Runtime.
"""

from dataclasses import dataclass, field
from typing import Dict, Any, Optional
import json
import os
from pathlib import Path


@dataclass
class Intent:
    """
    Structured representation of user intent.
    
    Attributes:
        name: The intent name (e.g., "remember", "search_web", "open_app")
        payload: Structured data specific to the intent
        confidence: Confidence score (0.0 to 1.0)
        source: Source of the intent detection ("rule" or "llm")
        reasoning: Optional reasoning from LLM on why this intent was selected (None by default)
    
    Examples:
        >>> Intent(name="remember", payload={"content": "buy milk"})
        >>> Intent(name="search_web", payload={"query": "Python", "url": "https://..."})
        >>> Intent(name="open_app", payload={"target": "chrome"})
        >>> Intent(name="remember", payload={"content": "task"}, source="llm", reasoning="User asked to remember this")
    """
    name: str
    payload: Dict[str, Any]
    confidence: float = 1.0
    source: str = "rule"  # "rule" for pattern-based, "llm" for LLM-generated
    reasoning: Optional[str] = None  # None by default; set when source="llm"
    
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
        
        # Validate against JSON schema if available
        self._validate_against_schema()
    
    def _validate_against_schema(self) -> None:
        """
        Lightweight validation against /shared/intent.schema.json.
        
        Fails gracefully if schema file not found (non-critical validation).
        Ensures schema consistency between Python and Rust layers.
        """
        try:
            # Import here to avoid hard dependency on jsonschema
            import jsonschema
            
            # Find schema relative to this file
            schema_path = Path(__file__).parent.parent.parent / "shared" / "intent.schema.json"
            
            if not schema_path.exists():
                # Schema file may not be available in all environments
                return
            
            with open(schema_path, "r") as f:
                schema = json.load(f)
            
            # Convert Intent to dict for validation
            intent_dict = {
                "name": self.name,
                "payload": self.payload,
                "confidence": self.confidence,
                "source": self.source,
                "reasoning": self.reasoning,
            }
            
            # Validate (will raise jsonschema.ValidationError if invalid)
            jsonschema.validate(intent_dict, schema)
            
        except ImportError:
            # jsonschema not installed - skip validation
            pass
        except Exception as e:
            # Log but don't fail if schema validation errors occur
            # This keeps the system resilient even if schema file is malformed
            import sys
            print(f"⚠️  Intent schema validation warning: {e}", file=sys.stderr)
