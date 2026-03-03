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

  const parseCommand = async (cmd: string): Promise<{ action: string; value: string } | null> => {
    const trimmed = cmd.trim();
    
    if (!trimmed) {
      return null;
    }

    try {
      // Send to Python Brain for intent interpretation
      const response = await fetch("http://127.0.0.1:8000/interpret", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ text: trimmed }),
      });

      if (!response.ok) {
        console.error("Brain API error:", response.status);
        return null;
      }

      const data = await response.json();
      
      // If action is unknown, treat as invalid command
      if (data.action === "unknown") {
        return null;
      }

      return { action: data.action, value: data.value };
    } catch (error) {
      console.error("Failed to connect to Brain API:", error);
      // Fallback to simple local parsing if Brain is unavailable
      return parseFallback(cmd);
    }
  };

  const parseFallback = (cmd: string): { action: string; value: string } | null => {
    const trimmed = cmd.trim().toLowerCase();

    if (trimmed === "list apps") {
      return { action: "list_apps", value: "" };
    }

    if (trimmed.startsWith("remember ")) {
      const value = cmd.slice(9).trim();
      return { action: "remember", value };
    }

    if (["recall", "what do you remember", "what do you remember?", 
         "recall memory", "recall memories", "show memories"].includes(trimmed)) {
      return { action: "recall_memory", value: "" };
    }

    if (trimmed.startsWith("search ")) {
      const keyword = cmd.slice(7).trim();
      return { action: "search_memory", value: keyword };
    }

    if (trimmed.startsWith("remind me to ")) {
      // Simple fallback - just pass the whole command as value
      // Brain will handle the parsing properly
      console.warn("Brain unavailable - reminder parsing may fail");
      return { action: "set_reminder", value: cmd };
    }

    if (trimmed.startsWith("what is ") || trimmed.startsWith("what's ")) {
      const query = trimmed.startsWith("what is ") ? cmd.slice(8).trim() : cmd.slice(7).trim();
      const encoded = encodeURIComponent(query);
      const url = `https://www.google.com/search?q=${encoded}`;
      console.warn(`Brain unavailable - using fallback Google search: ${url}`);
      return { action: "search_web", value: url };
    }

    if (trimmed.startsWith("google ") || trimmed.startsWith("look up ")) {
      const query = trimmed.startsWith("google ") ? cmd.slice(7).trim() : cmd.slice(8).trim();
      const encoded = encodeURIComponent(query);
      const url = `https://www.google.com/search?q=${encoded}`;
      console.warn(`Brain unavailable - using fallback Google search: ${url}`);
      return { action: "search_web", value: url };
    }

    if (trimmed.startsWith("open ")) {
      const value = cmd.slice(5).trim();
      
      // Check for "open X in web" pattern - build URL locally if Brain is unavailable
      const inWebMatch = value.match(/^(.+?)\s+in\s+web$/i);
      if (inWebMatch) {
        const searchTerm = inWebMatch[1].trim();
        // Simple URL building as fallback (Brain does DNS lookup for better results)
        const url = `https://www.${searchTerm.toLowerCase()}.com`;
        console.warn(`Brain unavailable - using fallback URL: ${url}`);
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

  const handleRun = async () => {
    if (!command.trim()) return;
    const parsed = await parseCommand(command);
    if (!parsed) {
      setResponse({
        success: false,
        message: 'Invalid command. Try: "open chrome", "remember X", "recall", "search X", "remind me to X in Y", "what is X"',
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
            placeholder="e.g., open chrome, remember X, recall, search X, remind me to X in Y, what is X"
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