import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import "./App.css";

interface ActionResponse {
  success: boolean;
  message: string;
  requires_confirmation: boolean;
  fallback_action: string | null;
  fallback_value: string | null;
  data: string[] | null;
}

function App() {
  const [command, setCommand] = useState("");
  const [response, setResponse] = useState<ActionResponse | null>(null);
  const [confirmation, setConfirmation] = useState(false);
  const [fallbackAction, setFallbackAction] = useState("");
  const [fallbackValue, setFallbackValue] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  const parseCommand = (cmd: string): { action: string; value: string } | null => {
    const trimmed = cmd.trim().toLowerCase();

    if (trimmed === "list apps") {
      return { action: "list_apps", value: "" };
    }

    if (trimmed.startsWith("open ")) {
      const value = cmd.slice(5).trim();
      
      // Check for "open X in chrome" pattern
      const inChromeMatch = value.match(/^(.+?)\s+in\s+chrome$/i);
      if (inChromeMatch) {
        const searchTerm = inChromeMatch[1].trim();
        let url: string;
        if (searchTerm === "youtube") {
          url = "https://www.youtube.com";
        } else {
          url = `https://www.${searchTerm}.com`;
        }
        return { action: "open_url", value: url };
      }
      
      if (value.startsWith("http://") || value.startsWith("https://")) {
        return { action: "open_url", value };
      }
      return { action: "open_app", value };
    }

    if (trimmed.startsWith("kill ")) {
      const value = cmd.slice(5).trim();
      return { action: "kill_process", value };
    }

    return null;
  };

  const executeCommand = async (action: string, value: string) => {
    setIsLoading(true);
    try {
      const result = await invoke<ActionResponse>("execute_action", {
        action,
        value,
      });
      setResponse(result);

      if (result.requires_confirmation) {
        setConfirmation(true);
        setFallbackAction(result.fallback_action || "");
        setFallbackValue(result.fallback_value || "");
      } else {
        setConfirmation(false);
      }
    } catch (error) {
      setResponse({
        success: false,
        message: `Error: ${error}`,
        requires_confirmation: false,
        fallback_action: null,
        fallback_value: null,
        data: null,
      });
      setConfirmation(false);
    } finally {
      setIsLoading(false);
    }
  };

  const handleRun = () => {
    if (!command.trim()) return;
    const parsed = parseCommand(command);
    if (!parsed) {
      setResponse({
        success: false,
        message: 'Invalid command. Try: "open chrome", "open youtube in chrome", "kill chrome.exe", "list apps"',
        requires_confirmation: false,
        fallback_action: null,
        fallback_value: null,
        data: null,
      });
      return;
    }
    executeCommand(parsed.action, parsed.value);
  };

  const handleConfirmYes = () => {
    if (fallbackAction && fallbackValue) {
      executeCommand(fallbackAction, fallbackValue);
    }
    setConfirmation(false);
  };

  const handleConfirmNo = () => {
    setConfirmation(false);
  };

  const handleListApps = () => {
    executeCommand("list_apps", "");
    setCommand("");
  };

  const handleRefreshApps = () => {
    // For now, this would require backend support for dynamic refresh
    // For testing, we can just call list_apps again
    executeCommand("list_apps", "");
  };

  return (
    <div style={{ padding: "20px", fontFamily: "monospace", backgroundColor: "#1e1e1e", color: "#e0e0e0", minHeight: "100vh" }}>
      <h1>Noddy 🧠 Test Console</h1>

      <div style={{ marginBottom: "20px", border: "1px solid #444", padding: "10px", backgroundColor: "#252526" }}>
        <h2>Command Input</h2>
        <div style={{ marginBottom: "10px" }}>
          <input
            type="text"
            value={command}
            onChange={(e) => setCommand(e.target.value)}
            onKeyPress={(e) => e.key === "Enter" && handleRun()}
            placeholder="e.g., open chrome, open youtube, kill notepad.exe, list apps"
            style={{
              width: "100%",
              padding: "8px",
              fontSize: "14px",
              boxSizing: "border-box",
              backgroundColor: "#3c3c3c",
              color: "#e0e0e0",
              border: "1px solid #555",
            }}
          />
        </div>
        <div style={{ display: "flex", gap: "10px" }}>
          <button onClick={handleRun} disabled={isLoading || !command.trim()} style={{ backgroundColor: "#007acc", color: "white", padding: "8px 16px" }}>
            {isLoading ? "Running..." : "Run"}
          </button>
          <button onClick={handleListApps} disabled={isLoading} style={{ backgroundColor: "#007acc", color: "white", padding: "8px 16px" }}>
            List Apps
          </button>
          <button onClick={handleRefreshApps} disabled={isLoading} style={{ backgroundColor: "#007acc", color: "white", padding: "8px 16px" }}>
            Refresh Apps
          </button>
        </div>
      </div>

      {confirmation && (
        <div style={{ marginBottom: "20px", border: "2px solid #ff9800", padding: "10px", backgroundColor: "#3c2c00" }}>
          <h3>⚠️ Confirmation Required</h3>
          <p>{response?.message}</p>
          <div style={{ display: "flex", gap: "10px" }}>
            <button onClick={handleConfirmYes} style={{ backgroundColor: "#4caf50", color: "white", padding: "8px 16px" }}>
              Yes
            </button>
            <button onClick={handleConfirmNo} style={{ backgroundColor: "#f44336", color: "white", padding: "8px 16px" }}>
              No
            </button>
          </div>
        </div>
      )}

      {response && (
        <div style={{ marginBottom: "20px", border: "1px solid #444", padding: "10px", backgroundColor: "#252526" }}>
          <h2>Response</h2>
          {response.data && (
            <div style={{ marginBottom: "10px", backgroundColor: "#1e1e1e", padding: "10px", border: "1px solid #444" }}>
              <h3>Installed Apps:</h3>
              <ul>
                {response.data.map((app) => (
                  <li key={app}>{app}</li>
                ))}
              </ul>
            </div>
          )}
          <pre
            style={{
              backgroundColor: "#1e1e1e",
              color: "#d4d4d4",
              padding: "10px",
              borderRadius: "4px",
              overflow: "auto",
              maxHeight: "400px",
              border: "1px solid #444",
            }}
          >
            {JSON.stringify(response, null, 2)}
          </pre>
        </div>
      )}
    </div>
  );
}

export default App;