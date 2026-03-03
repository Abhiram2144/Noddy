"""
Test cases for Noddy Brain intent parser.
Run with: python -m pytest test_parser.py -v
"""

import sys
from pathlib import Path

# Add parent directory to path to import main
sys.path.insert(0, str(Path(__file__).parent))

from main import parse_command, InterpretResponse


def test_list_apps():
    """Test 'list apps' command"""
    result = parse_command("list apps")
    assert result.action == "list_apps"
    assert result.value == ""
    assert result.confidence == 1.0


def test_list_apps_case_insensitive():
    """Test 'list apps' with different cases"""
    result = parse_command("LIST APPS")
    assert result.action == "list_apps"


def test_open_app():
    """Test 'open <app>' command"""
    result = parse_command("open chrome")
    assert result.action == "open_app"
    assert result.value == "chrome"


def test_open_app_preserves_case():
    """Test that app value preserves original case"""
    result = parse_command("open Discord")
    assert result.action == "open_app"
    assert result.value == "Discord"


def test_open_url_http():
    """Test 'open <http url>' command"""
    result = parse_command("open http://example.com")
    assert result.action == "open_url"
    assert result.value == "http://example.com"


def test_open_url_https():
    """Test 'open <https url>' command"""
    result = parse_command("open https://github.com")
    assert result.action == "open_url"
    assert result.value == "https://github.com"


def test_open_in_web_youtube():
    """Test 'open youtube in web' command with DNS resolution"""
    result = parse_command("open youtube in web")
    assert result.action == "open_url"
    # URL should start with https:// (actual domain depends on DNS resolution)
    assert result.value.startswith("https://")
    assert "youtube" in result.value


def test_open_in_web_generic():
    """Test 'open <term> in web' command for generic domain with DNS resolution"""
    result = parse_command("open github in web")
    assert result.action == "open_url"
    # URL should start with https:// and contain github (regardless of exact subdomain)
    assert result.value.startswith("https://")
    assert "github" in result.value


def test_open_in_web_case_insensitive():
    """Test 'open <term> in web' is case insensitive with DNS resolution"""
    result = parse_command("OPEN GITHUB IN WEB")
    assert result.action == "open_url"
    # URL should start with https:// and contain github
    assert result.value.startswith("https://")
    assert "github" in result.value


def test_kill_process():
    """Test 'kill <process>' command"""
    result = parse_command("kill notepad.exe")
    assert result.action == "kill_process"
    assert result.value == "notepad.exe"


def test_kill_process_case_insensitive_trigger():
    """Test 'kill' is case insensitive"""
    result = parse_command("KILL chrome.exe")
    assert result.action == "kill_process"
    assert result.value == "chrome.exe"


def test_unknown_command():
    """Test unknown command"""
    result = parse_command("hello world")
    assert result.action == "unknown"
    assert result.value == "hello world"


def test_empty_command():
    """Test that empty strings are handled"""
    result = parse_command("   ")
    # Should be treated as unknown since normalize_input returns empty string
    # or should be rejected at API level
    assert result is not None  # Just ensure no crash


def test_open_with_spaces():
    """Test 'open' with value containing spaces"""
    result = parse_command("open hollow knight")
    assert result.action == "open_app"
    assert result.value == "hollow knight"


def test_priority_url_over_app():
    """Test that URL detection has priority over app detection"""
    result = parse_command("open https://discord.com")
    assert result.action == "open_url"
    assert result.value == "https://discord.com"


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
