"""
Utility functions for Noddy Brain Layer.
"""

import socket
from urllib.parse import quote_plus
from config import get_logger

logger = get_logger(__name__)


def normalize_input(text: str) -> str:
    """Normalize input: strip whitespace and convert to lowercase"""
    return text.strip().lower()


def build_url(value: str) -> str:
    """
    Build full URL from search term using intelligent domain resolution.
    
    Strategy:
    1. Try common domain patterns (www.X.com, X.com)
    2. Use DNS lookup to verify domain exists
    3. Return the first valid domain
    4. Fall back to https://www.{value}.com if none resolve
    """
    value = value.lower().strip()
    
    # Create list of domain candidates to try
    candidates = [
        f"www.{value}.com",
        f"{value}.com",
        f"www.{value}.io",
        f"{value}.io",
    ]
    
    # Try each candidate via DNS resolution
    for domain in candidates:
        try:
            socket.gethostbyname(domain)
            logger.info(f"Domain resolved: {domain}")
            return f"https://{domain}"
        except (socket.gaierror, socket.error):
            # Domain doesn't resolve, try next candidate
            continue
    
    # If no domain resolves, use fallback pattern
    fallback_url = f"https://www.{value}.com"
    logger.info(f"No domain resolved, using fallback: {fallback_url}")
    return fallback_url


def build_google_search_url(query: str) -> str:
    """
    Build a Google search URL for the given query.
    
    Args:
        query: The search query string
    
    Returns:
        Properly encoded Google search URL
    """
    encoded_query = quote_plus(query)
    return f"https://www.google.com/search?q={encoded_query}"
