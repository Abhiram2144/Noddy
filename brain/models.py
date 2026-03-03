"""
Pydantic models for Noddy Brain Layer API.
"""

from pydantic import BaseModel


class InterpretRequest(BaseModel):
    """Request model for interpret endpoint"""
    text: str


class InterpretResponse(BaseModel):
    """Response model for interpret endpoint"""
    action: str
    value: str
    confidence: float = 1.0  # Rule-based = deterministic = 100% confidence
