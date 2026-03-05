import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import {
  LayoutDashboard,
  Bell,
  History,
  Brain,
  Zap,
  Settings,
  Clock,
  CheckCircle2,
  XCircle,
  Trash2,
  Search,
  Calendar,
  Mail,
  Plus,
  LucideIcon,
  Beaker,
} from "lucide-react";
import "./App.css";

// ============================================================================
// TYPES
// ============================================================================

interface NavItem {
  id: string;
  label: string;
  icon: LucideIcon;
}

interface Reminder {
  id: string;
  content: string;
  time: string;
  source: "Local" | "Google" | "Outlook";
}

interface CommandHistory {
  id: string;
  command: string;
  intent: string;
  timestamp: string;
  success: boolean;
  duration: number;
}

interface Memory {
  id: string;
  content: string;
  timestamp: string;
}

interface Integration {
  id: string;
  name: string;
  icon: LucideIcon;
  connected: boolean;
  color: string;
}

interface TestCommandResponse {
  success: boolean;
  message: string;
  data?: any;
  duration?: number;
  timestamp: string;
}

interface TestCommandResult {
  id: string;
  command: string;
  response: TestCommandResponse;
}

// ============================================================================
// NAVIGATION ITEMS
// ============================================================================

const navItems: NavItem[] = [
  { id: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { id: "reminders", label: "Reminders", icon: Bell },
  { id: "history", label: "History", icon: History },
  { id: "memory", label: "Memory", icon: Brain },
  { id: "integrations", label: "Integrations", icon: Zap },
  { id: "test", label: "Test Commands", icon: Beaker },
  { id: "settings", label: "Settings", icon: Settings },
];

// ============================================================================
// MOCK DATA (Replace with real API calls)
// ============================================================================

const mockReminders: Reminder[] = [
  { id: "1", content: "Team meeting at 3 PM", time: "Today, 3:00 PM", source: "Local" },
  { id: "2", content: "Call with client", time: "Tomorrow, 10:00 AM", source: "Google" },
  { id: "3", content: "Submit report", time: "Friday, 5:00 PM", source: "Outlook" },
];

const mockHistory: CommandHistory[] = [
  { id: "1", command: "open chrome", intent: "open_app", timestamp: "2 minutes ago", success: true, duration: 42 },
  { id: "2", command: "remember meeting notes", intent: "remember", timestamp: "5 minutes ago", success: true, duration: 18 },
  { id: "3", command: "kill notepad.exe", intent: "kill_process", timestamp: "10 minutes ago", success: false, duration: 125 },
];

const mockMemories: Memory[] = [
  { id: "1", content: "Project deadline is March 15th. Need to coordinate with design team.", timestamp: "2 hours ago" },
  { id: "2", content: "Client prefers communication via email, not phone calls.", timestamp: "Yesterday" },
  { id: "3", content: "API keys stored in LastPass under 'Production Environment'", timestamp: "3 days ago" },
];

const mockIntegrations: Integration[] = [
  { id: "1", name: "Google Calendar", icon: Calendar, connected: false, color: "#4285F4" },
  { id: "2", name: "Outlook", icon: Mail, connected: false, color: "#0078D4" },
];

// ============================================================================
// MAIN APP COMPONENT
// ============================================================================

function App() {
  const [currentView, setCurrentView] = useState("dashboard");
  const [reminders, setReminders] = useState(mockReminders);
  const [memories, setMemories] = useState(mockMemories);
  const [integrations, setIntegrations] = useState(mockIntegrations);
  const [searchQuery, setSearchQuery] = useState("");
  const [testResults, setTestResults] = useState<TestCommandResult[]>([]);

  return (
    <div className="app-container">
      {/* SIDEBAR */}
      <motion.aside
        className="sidebar"
        initial={{ x: -240, opacity: 0 }}
        animate={{ x: 0, opacity: 1 }}
        transition={{ duration: 0.5, ease: "easeOut" }}
      >
        <div className="sidebar-header">
          <div className="sidebar-logo">
            <div className="sidebar-logo-icon">🎯</div>
            <span>Noddy</span>
          </div>
        </div>

        <nav className="sidebar-nav">
          {navItems.map((item, index) => (
            <motion.div
              key={item.id}
              className={`nav-item ${currentView === item.id ? "active" : ""}`}
              onClick={() => setCurrentView(item.id)}
              initial={{ x: -20, opacity: 0 }}
              animate={{ x: 0, opacity: 1 }}
              transition={{ delay: index * 0.05, duration: 0.3 }}
              whileHover={{ scale: 1.03, x: 4 }}
              whileTap={{ scale: 0.98 }}
            >
              <item.icon className="nav-item-icon" />
              <span>{item.label}</span>
            </motion.div>
          ))}
        </nav>
      </motion.aside>

      {/* MAIN PANEL */}
      <main className="main-panel">
        <AnimatePresence mode="wait">
          {currentView === "dashboard" && (
            <DashboardView key="dashboard" reminders={reminders} history={mockHistory} memories={memories} />
          )}
          {currentView === "reminders" && (
            <RemindersView key="reminders" reminders={reminders} setReminders={setReminders} />
          )}
          {currentView === "history" && (
            <HistoryView key="history" history={mockHistory} />
          )}
          {currentView === "memory" && (
            <MemoryView key="memory" memories={memories} setMemories={setMemories} searchQuery={searchQuery} setSearchQuery={setSearchQuery} />
          )}
          {currentView === "test" && (
            <TestCommandsView key="test" testResults={testResults} setTestResults={setTestResults} />
          )}
          {currentView === "integrations" && (
            <IntegrationsView key="integrations" integrations={integrations} setIntegrations={setIntegrations} />
          )}
          {currentView === "settings" && (
            <SettingsView key="settings" />
          )}
        </AnimatePresence>
      </main>
    </div>
  );
}

// ============================================================================
// DASHBOARD VIEW
// ============================================================================

function DashboardView({ reminders, history, memories }: { reminders: Reminder[], history: CommandHistory[], memories: Memory[] }) {
  return (
    <motion.div
      className="panel-container"
      initial={{ opacity: 0, x: 40 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: -40 }}
      transition={{ duration: 0.4 }}
    >
      <div className="panel-header">
        <h1 className="panel-title">Dashboard</h1>
        <p className="panel-subtitle">Overview of your assistant's activity</p>
      </div>

      <div className="grid grid-2">
        {/* Upcoming Reminders Card */}
        <motion.div
          className="card"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
          whileHover={{ y: -4 }}
        >
          <div className="card-header">
            <h3 className="card-title">
              <Bell className="card-icon" />
              Upcoming Reminders
            </h3>
            <span className="badge badge-warning">{reminders.length}</span>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
            {reminders.slice(0, 3).map((reminder) => (
              <div key={reminder.id} className="list-item">
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "start" }}>
                  <div>
                    <div style={{ fontSize: "14px", color: "var(--text-primary)", marginBottom: "4px" }}>{reminder.content}</div>
                    <div style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
                      <Clock style={{ width: "12px", height: "12px", display: "inline", marginRight: "4px" }} />
                      {reminder.time}
                    </div>
                  </div>
                  <span className="badge badge-success" style={{ fontSize: "10px" }}>{reminder.source}</span>
                </div>
              </div>
            ))}
          </div>
        </motion.div>

        {/* Recent Commands Card */}
        <motion.div
          className="card"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.2 }}
          whileHover={{ y: -4 }}
        >
          <div className="card-header">
            <h3 className="card-title">
              <History className="card-icon" />
              Recent Commands
            </h3>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
            {history.slice(0, 3).map((cmd) => (
              <div key={cmd.id} className="list-item">
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <div>
                    <div style={{ fontSize: "13px", fontFamily: "monospace", color: "var(--text-primary)", marginBottom: "4px" }}>{cmd.command}</div>
                    <div style={{ fontSize: "12px", color: "var(--text-secondary)" }}>{cmd.timestamp} • {cmd.duration}ms</div>
                  </div>
                  {cmd.success ? (
                    <CheckCircle2 style={{ width: "16px", height: "16px", color: "var(--success)" }} />
                  ) : (
                    <XCircle style={{ width: "16px", height: "16px", color: "var(--error)" }} />
                  )}
                </div>
              </div>
            ))}
          </div>
        </motion.div>

        {/* Memory Summary Card */}
        <motion.div
          className="card"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.3 }}
          whileHover={{ y: -4 }}
        >
          <div className="card-header">
            <h3 className="card-title">
              <Brain className="card-icon" />
              Memory Vault
            </h3>
            <span className="badge badge-success">{memories.length}</span>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
            {memories.slice(0, 2).map((memory) => (
              <div key={memory.id} className="list-item">
                <div style={{ fontSize: "13px", color: "var(--text-primary)", marginBottom: "4px", lineHeight: "1.5" }}>
                  {memory.content.substring(0, 80)}...
                </div>
                <div style={{ fontSize: "12px", color: "var(--text-secondary)" }}>{memory.timestamp}</div>
              </div>
            ))}
          </div>
        </motion.div>

        {/* Integration Status Card */}
        <motion.div
          className="card"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.4 }}
          whileHover={{ y: -4 }}
        >
          <div className="card-header">
            <h3 className="card-title">
              <Zap className="card-icon" />
              Integrations
            </h3>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
            {mockIntegrations.map((integration) => (
              <div key={integration.id} className="list-item">
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
                    <div style={{ color: integration.color, display: "flex", alignItems: "center" }}>
                      <integration.icon size={20} />
                    </div>
                    <span style={{ fontSize: "14px", color: "var(--text-primary)" }}>{integration.name}</span>
                  </div>
                  <span className={`badge ${integration.connected ? "badge-success" : "badge-error"}`}>
                    {integration.connected ? "Connected" : "Disconnected"}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </motion.div>
      </div>
    </motion.div>
  );
}

// ============================================================================
// REMINDERS VIEW
// ============================================================================

function RemindersView({ reminders, setReminders }: { reminders: Reminder[], setReminders: React.Dispatch<React.SetStateAction<Reminder[]>> }) {
  const deleteReminder = (id: string) => {
    setReminders(reminders.filter(r => r.id !== id));
  };

  return (
    <motion.div
      className="panel-container"
      initial={{ opacity: 0, x: 40 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: -40 }}
      transition={{ duration: 0.4 }}
    >
      <div className="panel-header">
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div>
            <h1 className="panel-title">Reminders</h1>
            <p className="panel-subtitle">Manage your upcoming reminders</p>
          </div>
          <button className="btn btn-primary">
            <Plus style={{ width: "16px", height: "16px" }} />
            New Reminder
          </button>
        </div>
      </div>

      <AnimatePresence>
        {reminders.length === 0 ? (
          <motion.div
            className="empty-state"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
          >
            <Bell className="empty-state-icon" />
            <p>No reminders yet</p>
          </motion.div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
            {reminders.map((reminder, index) => (
              <motion.div
                key={reminder.id}
                className="list-item"
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: 20, height: 0 }}
                transition={{ delay: index * 0.05 }}
                whileHover={{ scale: 1.01 }}
                style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}
              >
                <div>
                  <div style={{ fontSize: "15px", fontWeight: 500, color: "var(--text-primary)", marginBottom: "6px" }}>{reminder.content}</div>
                  <div style={{ fontSize: "13px", color: "var(--text-secondary)", display: "flex", alignItems: "center", gap: "12px" }}>
                    <span>
                      <Clock style={{ width: "14px", height: "14px", display: "inline", marginRight: "4px" }} />
                      {reminder.time}
                    </span>
                    <span className="badge badge-success" style={{ fontSize: "11px" }}>{reminder.source}</span>
                  </div>
                </div>
                <motion.button
                  className="btn btn-secondary"
                  onClick={() => deleteReminder(reminder.id)}
                  whileHover={{ scale: 1.05 }}
                  whileTap={{ scale: 0.95 }}
                >
                  <Trash2 style={{ width: "16px", height: "16px" }} />
                </motion.button>
              </motion.div>
            ))}
          </div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}

// ============================================================================
// HISTORY VIEW
// ============================================================================

function HistoryView({ history }: { history: CommandHistory[] }) {
  return (
    <motion.div
      className="panel-container"
      initial={{ opacity: 0, x: 40 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: -40 }}
      transition={{ duration: 0.4 }}
    >
      <div className="panel-header">
        <h1 className="panel-title">Command History</h1>
        <p className="panel-subtitle">Recent command execution logs</p>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
        {history.map((cmd, index) => (
          <motion.div
            key={cmd.id}
            className="list-item"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: index * 0.05 }}
            whileHover={{ x: 4 }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "start", marginBottom: "10px" }}>
              <div style={{ flex: 1 }}>
                <div style={{ fontSize: "14px", fontFamily: "monospace", color: "var(--accent-primary)", marginBottom: "6px", background: "var(--bg-primary)", padding: "8px 12px", borderRadius: "6px" }}>
                  {cmd.command}
                </div>
                <div style={{ fontSize: "13px", color: "var(--text-secondary)", display: "flex", alignItems: "center", gap: "16px" }}>
                  <span>Intent: <strong style={{ color: "var(--text-primary)" }}>{cmd.intent}</strong></span>
                  <span>Duration: <strong style={{ color: "var(--text-primary)" }}>{cmd.duration}ms</strong></span>
                  <span>{cmd.timestamp}</span>
                </div>
              </div>
              <span className={`badge ${cmd.success ? "badge-success" : "badge-error"}`}>
                {cmd.success ? "Success" : "Failed"}
              </span>
            </div>
          </motion.div>
        ))}
      </div>
    </motion.div>
  );
}

// ============================================================================
// MEMORY VIEW
// ============================================================================

function MemoryView({ 
  memories, 
  setMemories, 
  searchQuery, 
  setSearchQuery 
}: { 
  memories: Memory[], 
  setMemories: React.Dispatch<React.SetStateAction<Memory[]>>, 
  searchQuery: string, 
  setSearchQuery: React.Dispatch<React.SetStateAction<string>> 
}) {
  const filteredMemories = memories.filter(m => 
    m.content.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const deleteMemory = (id: string) => {
    setMemories(memories.filter(m => m.id !== id));
  };

  return (
    <motion.div
      className="panel-container"
      initial={{ opacity: 0, x: 40 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: -40 }}
      transition={{ duration: 0.4 }}
    >
      <div className="panel-header">
        <h1 className="panel-title">Memory Vault</h1>
        <p className="panel-subtitle">Search and manage stored memories</p>
      </div>

      <div style={{ marginBottom: "24px" }}>
        <div style={{ position: "relative" }}>
          <Search style={{ position: "absolute", left: "16px", top: "50%", transform: "translateY(-50%)", width: "18px", height: "18px", color: "var(--text-tertiary)" }} />
          <input
            type="text"
            className="search-input"
            placeholder="Search memories..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            style={{ paddingLeft: "44px" }}
          />
        </div>
      </div>

      <AnimatePresence>
        {filteredMemories.length === 0 ? (
          <motion.div
            className="empty-state"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
          >
            <Brain className="empty-state-icon" />
            <p>{searchQuery ? "No memories found" : "No memories stored yet"}</p>
          </motion.div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
            {filteredMemories.map((memory, index) => (
              <motion.div
                key={memory.id}
                className="list-item"
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, x: -20, height: 0 }}
                transition={{ delay: index * 0.03 }}
                whileHover={{ scale: 1.01 }}
                style={{ display: "flex", justifyContent: "space-between", alignItems: "start", gap: "16px" }}
              >
                <div style={{ flex: 1 }}>
                  <div style={{ fontSize: "14px", color: "var(--text-primary)", marginBottom: "8px", lineHeight: "1.6" }}>
                    {memory.content}
                  </div>
                  <div style={{ fontSize: "12px", color: "var(--text-secondary)" }}>{memory.timestamp}</div>
                </div>
                <motion.button
                  className="btn btn-secondary"
                  onClick={() => deleteMemory(memory.id)}
                  whileHover={{ scale: 1.05 }}
                  whileTap={{ scale: 0.95 }}
                >
                  <Trash2 style={{ width: "16px", height: "16px" }} />
                </motion.button>
              </motion.div>
            ))}
          </div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}

// ============================================================================
// INTEGRATIONS VIEW
// ============================================================================

function IntegrationsView({ 
  integrations, 
  setIntegrations 
}: { 
  integrations: Integration[], 
  setIntegrations: React.Dispatch<React.SetStateAction<Integration[]>> 
}) {
  const toggleConnection = (id: string) => {
    setIntegrations(integrations.map(int => 
      int.id === id ? { ...int, connected: !int.connected } : int
    ));
  };

  return (
    <motion.div
      className="panel-container"
      initial={{ opacity: 0, x: 40 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: -40 }}
      transition={{ duration: 0.4 }}
    >
      <div className="panel-header">
        <h1 className="panel-title">Integrations</h1>
        <p className="panel-subtitle">Connect external services</p>
      </div>

      <div className="grid grid-2">
        {integrations.map((integration, index) => (
          <motion.div
            key={integration.id}
            className="card"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: index * 0.1 }}
            whileHover={{ y: -4 }}
          >
            <div style={{ display: "flex", alignItems: "start", justifyContent: "space-between", marginBottom: "20px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
                <div style={{ width: "48px", height: "48px", borderRadius: "12px", background: `${integration.color}15`, display: "flex", alignItems: "center", justifyContent: "center", color: integration.color }}>
                  <integration.icon size={24} />
                </div>
                <div>
                  <h3 style={{ fontSize: "16px", fontWeight: 600, color: "var(--text-primary)", marginBottom: "4px" }}>{integration.name}</h3>
                  <span className={`badge ${integration.connected ? "badge-success" : "badge-error"}`}>
                    {integration.connected ? "Connected" : "Disconnected"}
                  </span>
                </div>
              </div>
            </div>
            <motion.button
              className={`btn ${integration.connected ? "btn-secondary" : "btn-primary"}`}
              onClick={() => toggleConnection(integration.id)}
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
              style={{ width: "100%" }}
            >
              {integration.connected ? "Disconnect" : "Connect"}
            </motion.button>
          </motion.div>
        ))}
      </div>
    </motion.div>
  );
}

// ============================================================================
// SETTINGS VIEW
// ============================================================================

function SettingsView() {
  return (
    <motion.div
      className="panel-container"
      initial={{ opacity: 0, x: 40 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: -40 }}
      transition={{ duration: 0.4 }}
    >
      <div className="panel-header">
        <h1 className="panel-title">Settings</h1>
        <p className="panel-subtitle">Configure your assistant</p>
      </div>

      <motion.div
        className="card"
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.1 }}
      >
        <div className="empty-state">
          <Settings className="empty-state-icon" />
          <p>Settings panel coming soon</p>
        </div>
      </motion.div>
    </motion.div>
  );
}

// ============================================================================
// TEST COMMANDS VIEW
// ============================================================================

function parseTestCommand(input: string): string {
  const trimmed = input.trim().toLowerCase();

  // remember X
  if (trimmed.startsWith("remember ")) {
    const content = input.slice(9).trim();
    return JSON.stringify({ name: "remember", payload: { content } });
  }

  // recall / what do you remember / etc
  if (["recall", "what do you remember", "what do you remember?", "recall memory", "recall memories", "show memories"].includes(trimmed)) {
    return JSON.stringify({ name: "recall_memory" });
  }

  // search X / search for X
  if (trimmed.startsWith("search ")) {
    const keyword = input.slice(7).trim();
    return JSON.stringify({ name: "search_memory", payload: { keyword } });
  }

  // remind me to X
  if (trimmed.startsWith("remind me to ")) {
    const content = input.slice(13).trim();
    return JSON.stringify({ name: "set_reminder", payload: { content, trigger_at: Math.floor(Date.now() / 1000) + 3600 } });
  }

  // google X / search web X / what is X / what's X
  if (trimmed.startsWith("google ") || trimmed.startsWith("search web ") || trimmed.startsWith("what is ") || trimmed.startsWith("what's ")) {
    let query = "";
    if (trimmed.startsWith("google ")) query = input.slice(7).trim();
    else if (trimmed.startsWith("search web ")) query = input.slice(11).trim();
    else if (trimmed.startsWith("what is ")) query = input.slice(8).trim();
    else if (trimmed.startsWith("what's ")) query = input.slice(7).trim();
    
    const url = `https://www.google.com/search?q=${encodeURIComponent(query)}`;
    return JSON.stringify({ name: "search_web", payload: { url } });
  }

  // open X
  if (trimmed.startsWith("open ")) {
    const value = input.slice(5).trim();
    
    // Check for URLs
    if (value.startsWith("http://") || value.startsWith("https://")) {
      return JSON.stringify({ name: "open_url", payload: { url: value } });
    }
    
    // Check for "open X in web"
    const inWebMatch = value.match(/^(.+?)\s+in\s+web$/i);
    if (inWebMatch) {
      const searchTerm = inWebMatch[1].trim();
      const url = `https://www.${searchTerm.toLowerCase()}.com`;
      return JSON.stringify({ name: "open_url", payload: { url } });
    }
    
    return JSON.stringify({ name: "open_app", payload: { target: value } });
  }

  // kill X / kill process X
  if (trimmed.startsWith("kill ")) {
    const process = input.slice(5).trim();
    return JSON.stringify({ name: "kill_process", payload: { process } });
  }

  // list apps
  if (["list apps", "show apps", "available apps"].includes(trimmed)) {
    return JSON.stringify({ name: "list_apps" });
  }

  // Default to unknown
  return JSON.stringify({ name: "unknown", payload: { text: input } });
}

function TestCommandsView({ 
  testResults, 
  setTestResults 
}: { 
  testResults: TestCommandResult[], 
  setTestResults: React.Dispatch<React.SetStateAction<TestCommandResult[]>> 
}) {
  const [commandInput, setCommandInput] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  const executeTestCommand = async () => {
    if (!commandInput.trim()) return;

    setIsLoading(true);
    const startTime = performance.now();

    try {
      const intentJson = parseTestCommand(commandInput);
      const result = await invoke<TestCommandResponse>("execute_action", {
        intentJson,
      });

      const duration = Math.round(performance.now() - startTime);
      const newResult: TestCommandResult = {
        id: Date.now().toString(),
        command: commandInput,
        response: {
          ...result,
          duration,
          timestamp: new Date().toLocaleTimeString(),
        },
      };

      setTestResults([newResult, ...testResults]);
      setCommandInput("");
    } catch (error) {
      const duration = Math.round(performance.now() - startTime);
      const newResult: TestCommandResult = {
        id: Date.now().toString(),
        command: commandInput,
        response: {
          success: false,
          message: `Error: ${error}`,
          duration,
          timestamp: new Date().toLocaleTimeString(),
        },
      };

      setTestResults([newResult, ...testResults]);
      setCommandInput("");
    } finally {
      setIsLoading(false);
    }
  };

  const clearResults = () => {
    setTestResults([]);
  };

  return (
    <motion.div
      className="panel-container"
      initial={{ opacity: 0, x: 40 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: -40 }}
      transition={{ duration: 0.4 }}
    >
      <div className="panel-header">
        <h1 className="panel-title">Test Commands</h1>
        <p className="panel-subtitle">Execute and debug commands</p>
      </div>

      {/* Input Section */}
      <motion.div
        className="card"
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.1 }}
        style={{ marginBottom: "24px" }}
      >
        <div className="card-header">
          <h3 className="card-title">
            <Beaker className="card-icon" />
            Command Input
          </h3>
        </div>
        
        <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
          <textarea
            value={commandInput}
            onChange={(e) => setCommandInput(e.target.value)}
            placeholder="Try: 'open chrome', 'remember meeting notes', 'recall', 'search python', 'kill notepad', 'list apps', 'what is machine learning'"
            className="search-input"
            style={{
              minHeight: "80px",
              fontFamily: "monospace",
              fontSize: "13px",
              resize: "vertical",
              padding: "12px 16px",
            }}
            onKeyPress={(e) => {
              if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
                executeTestCommand();
              }
            }}
          />
          
          <div style={{ display: "flex", gap: "12px", justifyContent: "space-between" }}>
            <motion.button
              className="btn btn-primary"
              onClick={executeTestCommand}
              disabled={isLoading || !commandInput.trim()}
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
            >
              {isLoading ? "Executing..." : "Execute Command"}
            </motion.button>
            
            {testResults.length > 0 && (
              <motion.button
                className="btn btn-secondary"
                onClick={clearResults}
                whileHover={{ scale: 1.02 }}
                whileTap={{ scale: 0.98 }}
              >
                Clear Results
              </motion.button>
            )}
          </div>
        </div>
      </motion.div>

      {/* Results Section */}
      <AnimatePresence>
        {testResults.length === 0 ? (
          <motion.div
            className="empty-state"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
          >
            <Beaker className="empty-state-icon" />
            <p>No test results yet. Enter a command and click Execute to test.</p>
          </motion.div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
            <h3 style={{ fontSize: "16px", fontWeight: 600, color: "var(--text-primary)", marginBottom: "12px" }}>
              Test Results ({testResults.length})
            </h3>
            
            {testResults.map((result, index) => (
              <motion.div
                key={result.id}
                className="card"
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -20, height: 0 }}
                transition={{ delay: index * 0.05 }}
              >
                <div style={{ marginBottom: "16px" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "start", marginBottom: "12px" }}>
                    <div style={{ flex: 1 }}>
                      <div style={{ fontSize: "14px", fontFamily: "monospace", color: "var(--accent-primary)", marginBottom: "8px", background: "var(--bg-primary)", padding: "10px 12px", borderRadius: "6px", wordBreak: "break-word" }}>
                        {result.command}
                      </div>
                      <div style={{ fontSize: "12px", color: "var(--text-secondary)", display: "flex", alignItems: "center", gap: "16px" }}>
                        <span>Duration: <strong style={{ color: "var(--text-primary)" }}>{result.response.duration}ms</strong></span>
                        <span>{result.response.timestamp}</span>
                      </div>
                    </div>
                    <span className={`badge ${result.response.success ? "badge-success" : "badge-error"}`}>
                      {result.response.success ? "Success" : "Failed"}
                    </span>
                  </div>
                </div>

                {/* Response Message */}
                <div style={{ marginBottom: "16px" }}>
                  <h4 style={{ fontSize: "12px", fontWeight: 600, color: "var(--text-secondary)", marginBottom: "6px", textTransform: "uppercase" }}>Message</h4>
                  <div style={{ background: "var(--bg-primary)", padding: "12px", borderRadius: "6px", fontSize: "13px", color: "var(--text-primary)", lineHeight: "1.6", wordBreak: "break-word" }}>
                    {result.response.message}
                  </div>
                </div>

                {/* Response Data */}
                {result.response.data && (
                  <div>
                    <h4 style={{ fontSize: "12px", fontWeight: 600, color: "var(--text-secondary)", marginBottom: "6px", textTransform: "uppercase" }}>Response Data</h4>
                    <pre style={{ background: "var(--bg-primary)", padding: "12px", borderRadius: "6px", fontSize: "12px", color: "var(--accent-primary)", overflow: "auto", maxHeight: "200px", fontFamily: "monospace", margin: 0 }}>
                      {typeof result.response.data === "string"
                        ? result.response.data
                        : JSON.stringify(result.response.data, null, 2)}
                    </pre>
                  </div>
                )}
              </motion.div>
            ))}
          </div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}

export default App;