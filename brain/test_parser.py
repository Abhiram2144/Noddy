"""
Test cases for Noddy Brain intent parser - Modular Architecture.

Tests the full pipeline:
1. parse_command(text) → Intent (domain model)
2. intent_to_response(intent) → InterpretResponse (API response)

This ensures both internal domain modeling and external API contract work correctly.

Run with: python -m pytest test_parser.py -v
Or: python test_parser.py
"""

import sys
from pathlib import Path

# Add parent directory to path to import modules
sys.path.insert(0, str(Path(__file__).parent))

from parsers import parse_command
from models import InterpretResponse
from domain import Intent


# Helper function to get API response (same logic as main.py)
def get_response(text: str) -> InterpretResponse:
    """
    Parse text to Intent and convert to InterpretResponse.
    Mirrors the logic in main.py /interpret endpoint.
    """
    from main import intent_to_response
    intent = parse_command(text)
    return intent_to_response(intent)


def test_list_apps():
    """Test 'list apps' command"""
    result = get_response("list apps")
    assert result.action == "list_apps"
    assert result.value == ""
    assert result.confidence == 1.0


def test_list_apps_case_insensitive():
    """Test 'list apps' with different cases"""
    result = get_response("LIST APPS")
    assert result.action == "list_apps"


def test_open_app():
    """Test 'open <app>' command"""
    result = get_response("open chrome")
    assert result.action == "open_app"
    assert result.value == "chrome"


def test_open_app_preserves_case():
    """Test that app value preserves original case"""
    result = get_response("open Discord")
    assert result.action == "open_app"
    assert result.value == "Discord"


def test_open_url_http():
    """Test 'open <http url>' command"""
    result = get_response("open http://example.com")
    assert result.action == "open_url"
    assert result.value == "http://example.com"


def test_open_url_https():
    """Test 'open <https url>' command"""
    result = get_response("open https://github.com")
    assert result.action == "open_url"
    assert result.value == "https://github.com"


def test_open_in_web_youtube():
    """Test 'open youtube in web' command with DNS resolution"""
    result = get_response("open youtube in web")
    assert result.action == "open_url"
    # URL should start with https:// (actual domain depends on DNS resolution)
    assert result.value.startswith("https://")
    assert "youtube" in result.value


def test_open_in_web_generic():
    """Test 'open <term> in web' command for generic domain with DNS resolution"""
    result = get_response("open github in web")
    assert result.action == "open_url"
    # URL should start with https:// and contain github (regardless of exact subdomain)
    assert result.value.startswith("https://")
    assert "github" in result.value


def test_open_in_web_case_insensitive():
    """Test 'open <term> in web' is case insensitive with DNS resolution"""
    result = get_response("OPEN GITHUB IN WEB")
    assert result.action == "open_url"
    # URL should start with https:// and contain github
    assert result.value.startswith("https://")
    assert "github" in result.value


def test_kill_process():
    """Test 'kill <process>' command"""
    result = get_response("kill notepad.exe")
    assert result.action == "kill_process"
    assert result.value == "notepad.exe"


def test_kill_process_case_insensitive_trigger():
    """Test 'kill' is case insensitive"""
    result = get_response("KILL chrome.exe")
    assert result.action == "kill_process"
    assert result.value == "chrome.exe"


def test_unknown_command():
    """Test unknown command"""
    result = get_response("hello world")
    assert result.action == "unknown"
    assert result.value == "hello world"


def test_empty_command():
    """Test that empty strings are handled"""
    result = get_response("   ")
    # Should be treated as unknown since normalize_input returns empty string
    # or should be rejected at API level
    assert result is not None  # Just ensure no crash


def test_open_with_spaces():
    """Test 'open' with value containing spaces"""
    result = get_response("open hollow knight")
    assert result.action == "open_app"
    assert result.value == "hollow knight"


def test_priority_url_over_app():
    """Test that URL detection has priority over app detection"""
    result = get_response("open https://discord.com")
    assert result.action == "open_url"
    assert result.value == "https://discord.com"


def test_remember_command():
    """Test 'remember <content>' command"""
    result = get_response("remember buy milk")
    assert result.action == "remember"
    assert result.value == "buy milk"


def test_remember_preserves_case():
    """Test that remember preserves original case"""
    result = get_response("remember Meeting with Sarah at 3pm")
    assert result.action == "remember"
    assert result.value == "Meeting with Sarah at 3pm"


def test_recall_command():
    """Test 'recall' command"""
    result = get_response("recall")
    assert result.action == "recall_memory"
    assert result.value == ""


def test_recall_variants():
    """Test various recall command phrasings"""
    variants = [
        "what do you remember",
        "What do you remember?",
        "recall memory",
        "recall memories",
        "show memories"
    ]
    for variant in variants:
        result = get_response(variant)
        assert result.action == "recall_memory", f"Failed for: {variant}"
        assert result.value == ""


def test_search_memory():
    """Test 'search <keyword>' command"""
    result = get_response("search meeting")
    assert result.action == "search_memory"
    assert result.value == "meeting"


def test_search_memory_preserves_case():
    """Test that search preserves keyword case"""
    result = get_response("search Project Alpha")
    assert result.action == "search_memory"
    assert result.value == "Project Alpha"


def test_set_reminder_minutes():
    """Test 'remind me to <X> in <minutes>' command"""
    result = get_response("remind me to call mom in 30 minutes")
    assert result.action == "set_reminder"
    # Value should be valid JSON with content and trigger_at
    import json
    parsed = json.loads(result.value)
    assert "content" in parsed
    assert "trigger_at" in parsed
    assert parsed["content"] == "call mom"
    assert isinstance(parsed["trigger_at"], int)


def test_set_reminder_hours():
    """Test 'remind me to <X> in <hours>' command"""
    result = get_response("remind me to submit report in 2 hours")
    assert result.action == "set_reminder"
    import json
    parsed = json.loads(result.value)
    assert parsed["content"] == "submit report"


def test_set_reminder_days():
    """Test 'remind me to <X> in <days>' command"""
    result = get_response("remind me to pay bills in 5 days")
    assert result.action == "set_reminder"
    import json
    parsed = json.loads(result.value)
    assert parsed["content"] == "pay bills"


# ===== Web Search Tests (NEW) =====

def test_what_is_query():
    """Test 'what is <X>' command"""
    result = get_response("what is Python")
    assert result.action == "search_web"
    assert "google.com/search" in result.value
    assert "Python" in result.value or "python" in result.value.lower()


def test_what_is_case_insensitive():
    """Test 'what is' is case insensitive"""
    result = get_response("What is Machine Learning")
    assert result.action == "search_web"
    assert "google.com/search" in result.value


def test_whats_query():
    """Test 'what's <X>' command"""
    result = get_response("what's artificial intelligence")
    assert result.action == "search_web"
    assert "google.com/search" in result.value
    assert "artificial" in result.value.lower()


def test_search_about_in_web():
    """Test 'search about <X> in web' command"""
    result = get_response("search about quantum computing in web")
    assert result.action == "search_web"
    assert "google.com/search" in result.value
    assert "quantum" in result.value.lower()


def test_search_about_on_web():
    """Test 'search about <X> on web' command"""
    result = get_response("search about blockchain on web")
    assert result.action == "search_web"
    assert "google.com/search" in result.value


def test_search_for_on_google():
    """Test 'search for <X> on google' command"""
    result = get_response("search for machine learning on google")
    assert result.action == "search_web"
    assert "google.com/search" in result.value


def test_google_query():
    """Test 'google <X>' command"""
    result = get_response("google rust programming")
    assert result.action == "search_web"
    assert "google.com/search" in result.value
    assert "rust" in result.value.lower()


def test_look_up_query():
    """Test 'look up <X>' command"""
    result = get_response("look up neural networks")
    assert result.action == "search_web"
    assert "google.com/search" in result.value


def test_find_information_query():
    """Test 'find information about <X>' command"""
    result = get_response("find information about climate change")
    assert result.action == "search_web"
    assert "google.com/search" in result.value


def test_search_query_url_encoding():
    """Test that search queries with spaces are properly URL encoded"""
    result = get_response("what is machine learning")
    assert result.action == "search_web"
    # URL should have encoded spaces (either + or %20)
    assert "google.com/search" in result.value
    assert ("machine+learning" in result.value or "machine%20learning" in result.value)


def test_search_vs_memory_search_priority():
    """Test that 'search <X>' goes to memory_parser, not search_parser"""
    result = get_response("search meeting")
    assert result.action == "search_memory"  # Should be memory search, not web search
    assert result.value == "meeting"


if __name__ == "__main__":
    # Simple test runner
    import inspect
    
    test_functions = [
        obj for name, obj in inspect.getmembers(sys.modules[__name__])
        if inspect.isfunction(obj) and name.startswith("test_")
    ]
    
    passed = 0
    failed = 0
    
    print("Running Noddy Brain parser tests...\n")
    
    for test_func in test_functions:
        try:
            test_func()
            print(f"✓ {test_func.__name__}")
            passed += 1
        except AssertionError as e:
            print(f"✗ {test_func.__name__}: {e}")
            failed += 1
        except Exception as e:
            print(f"✗ {test_func.__name__}: {type(e).__name__}: {e}")
            failed += 1
    
    print(f"\n{passed} passed, {failed} failed")
    sys.exit(0 if failed == 0 else 1)

