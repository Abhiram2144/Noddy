import re
from typing import Optional
from domain import Intent
from parsers.base import BaseParser

class SystemParser(BaseParser):
    """
    Parser for Windows System Operations like volume, brightness, and power.
    Uses robust regex to handle natural language variations.
    """
    
    def can_parse(self, text: str) -> bool:
        """Check if the text contain system control keywords."""
        keywords = [
            "volume", "mute", "unmute", "sound", "loud", "quiet", "up", "down", "raise", "lower",
            "brightness", "screen", "bright", "dark", "lock", "shutdown", "restart", "reboot", "sleep"
        ]
        text_lower = text.lower()
        return any(word in text_lower for word in keywords)

    def parse(self, text: str) -> Intent:
        """Parse system control commands using robust regex."""
        normalized = text.lower()
        
        # 1. Volume Control (Absolute level)
        # Check for Get Volume first
        if re.search(r'(what|how much|check|current).*?(volume|sound)', normalized):
            return Intent(name="get_volume", payload={}, confidence=0.9)
            
        # Matches: "set volume to 50", "volume at 80", "volume 100"
        vol_level_match = re.search(r'volume.*?\b(\d+)\b', normalized)
        if vol_level_match:
            return Intent(
                name="set_volume",
                payload={"level": int(vol_level_match.group(1))},
                confidence=0.9
            )
        
        # Volume Up (Relative)
        # Matches: "raise the volume", "turn it up", "increase volume", "volume up"
        if re.search(r'(raise|increase|up|louder).*?volume|volume.*?up|turn.*?up', normalized):
            return Intent(name="set_volume", payload={"action": "increase"}, confidence=0.9)
            
        # Volume Down (Relative)
        # Matches: "lower volume", "turn it down", "quieter", "volume down"
        if re.search(r'(lower|decrease|down|quieter).*?volume|volume.*?down|turn.*?down', normalized):
            return Intent(name="set_volume", payload={"action": "decrease"}, confidence=0.9)
            
        if "mute" in normalized and "unmute" not in normalized:
            return Intent(name="set_volume", payload={"action": "mute"}, confidence=0.9)
            
        if "unmute" in normalized:
            return Intent(name="set_volume", payload={"action": "unmute"}, confidence=0.9)

        # 2. Brightness Control (Absolute)
        # Check for Get Brightness first
        if re.search(r'(what|how much|check|current).*?(brightness|light|screen)', normalized):
            return Intent(name="get_brightness", payload={}, confidence=0.9)
            
        # Matches: "brightness to 80", "screen at 50", "set brightness 100"
        bright_level_match = re.search(r'(brightness|screen).*?\b(\d+)\b', normalized)
        if bright_level_match:
            return Intent(
                name="set_brightness",
                payload={"level": int(bright_level_match.group(2))},
                confidence=0.9
            )
        
        # Brightness (Relative)
        # Matches: "make it brighter", "increase brightness", "screen up"
        if re.search(r'(brighter|increase|raise|up).*?(brightness|screen|light)|(brightness|screen|light).*?up|make.*?brighter', normalized):
            return Intent(name="set_brightness", payload={"level": 80}, confidence=0.8)
            
        if re.search(r'(darker|decrease|lower|down).*?(brightness|screen|light)|(brightness|screen|light).*?down|make.*?darker', normalized):
            return Intent(name="set_brightness", payload={"level": 20}, confidence=0.8)

        # 3. Power & Lock
        if re.search(r'lock.*?(screen|computer|pc|my)', normalized) or normalized == "lock":
            return Intent(name="system_control", payload={"command": "lock"}, confidence=0.9)
            
        if re.search(r'shutdown|turn.*?off.*?(pc|computer|system)', normalized):
            return Intent(name="system_control", payload={"command": "shutdown"}, confidence=0.9)
            
        if re.search(r'restart|reboot', normalized):
            return Intent(name="system_control", payload={"command": "restart"}, confidence=0.9)
            
        if "sleep" in normalized and "remind" not in normalized:
             return Intent(name="system_control", payload={"command": "sleep"}, confidence=0.8)

        return Intent(
            name="unknown",
            payload={"text": text},
            confidence=0.1
        )
