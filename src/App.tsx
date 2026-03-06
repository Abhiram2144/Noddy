import { useState } from "react";
import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { sendNotification } from "@tauri-apps/plugin-notification";
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
  Network,
  List,
} from "lucide-react";
import "./App.css";
import { MemoryListView } from "./components/MemoryListView";
import { MemoryGraphView } from "./components/MemoryGraphView";
import { CommandHistoryView } from "./components/CommandHistoryView";
import { LoginPage } from "./components/LoginPage";
import { SignupPage } from "./components/SignupPage";
import { useAuth } from "./auth/AuthContext";

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
  trigger_at: number;
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
  enabled: boolean;
  description: string;
  provider: string;
  capabilities: string[];
  config_json?: string | null;
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
  { id: "memory-list", label: "Memory Bank", icon: List },
  { id: "memory-graph", label: "Knowledge Graph", icon: Network },
  { id: "integrations", label: "Integrations", icon: Zap },
  { id: "test", label: "Test Commands", icon: Beaker },
  { id: "settings", label: "Settings", icon: Settings },
];

// ============================================================================
// DATA FETCHING UTILITIES
// ============================================================================

async function fetchReminders(accessToken: string): Promise<Reminder[]> {
  try {
    const data = await invoke<any>("get_reminders", { accessToken });
    if (Array.isArray(data)) {
      return data.map((reminder: any) => ({
        id: reminder.id || Math.random().toString(),
        content: reminder.content || reminder.title || "Untitled reminder",
        trigger_at: reminder.trigger_at || Math.floor(Date.now() / 1000) + 3600,
        time: reminder.time || reminder.due_date || "No time set",
        source: reminder.source || "Local",
      }));
    }
    return [];
  } catch (error) {
    console.warn("Failed to fetch reminders:", error);
    return [];
  }
}

async function fetchCommandHistory(accessToken: string): Promise<CommandHistory[]> {
  try {
    const data = await invoke<any>("get_command_history", { limit: 3, accessToken });
    if (Array.isArray(data)) {
      return data.map((cmd: any) => ({
        id: cmd.id || Math.random().toString(),
        command: cmd.command || cmd.text || "Unknown command",
        intent: cmd.intent || cmd.action || "unknown",
        timestamp: cmd.timestamp || "Unknown time",
        success: cmd.success !== false,
        duration: cmd.duration || 0,
      }));
    }
    return [];
  } catch (error) {
    console.warn("Failed to fetch command history:", error);
    return [];
  }
}

async function fetchMemories(accessToken: string): Promise<Memory[]> {
  try {
    const data = await invoke<any>("get_memories", { limit: 2, accessToken });
    if (Array.isArray(data)) {
      return data.map((memory: any) => ({
        id: memory.id || Math.random().toString(),
        content: memory.content || memory.text || "Empty memory",
        timestamp: memory.timestamp || "Unknown time",
      }));
    }
    return [];
  } catch (error) {
    console.warn("Failed to fetch memories:", error);
    return [];
  }
}

function mapPluginRecord(plugin: any): Integration {
  return {
    id: plugin.id || Math.random().toString(),
    name: plugin.name || "Unknown Plugin",
    enabled: plugin.enabled === true,
    description: plugin.description || "No description available.",
    provider: plugin.provider || "custom",
    capabilities: Array.isArray(plugin.capabilities) ? plugin.capabilities : [],
    config_json: plugin.config_json ?? "{}",
  };
}

async function fetchIntegrations(accessToken: string): Promise<Integration[]> {
  try {
    const data = await invoke<any>("get_plugins", { accessToken });
    if (Array.isArray(data)) {
      return data.map(mapPluginRecord);
    }
    return [];
  } catch (error) {
    console.warn("Failed to fetch plugins:", error);
    return [];
  }
}

// ============================================================================
// MOCK DATA (Fallback for development)
// ============================================================================

const integrationVisuals: Record<string, { icon: LucideIcon; color: string }> = {
  google: { icon: Calendar, color: "#4285F4" },
  outlook: { icon: Mail, color: "#0078D4" },
  custom: { icon: Zap, color: "#8b7bea" },
};

function getIntegrationVisual(integration: Integration) {
  return integrationVisuals[integration.provider] || integrationVisuals.custom;
}

// ============================================================================
// MAIN APP COMPONENT
// ============================================================================

function App() {
  const { user, loading, login, signup, logout, getAccessToken } = useAuth();
  const [authMode, setAuthMode] = useState<"login" | "signup">("login");
  const [currentView, setCurrentView] = useState("dashboard");
  const [reminders, setReminders] = useState<Reminder[]>([]);
  const [commandHistory, setCommandHistory] = useState<CommandHistory[]>([]);
  const [memories, setMemories] = useState<Memory[]>([]);
  const [integrations, setIntegrations] = useState<Integration[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [testResults, setTestResults] = useState<TestCommandResult[]>([]);
  const [isLoadingDashboard, setIsLoadingDashboard] = useState(true);
  const [pendingReminderNavigation, setPendingReminderNavigation] = useState(false);
  const [activeReminderAction, setActiveReminderAction] = useState<{ id: number; content: string } | null>(null);
  const [isActionProcessing, setIsActionProcessing] = useState(false);

  const invokeAuthed = async <T,>(command: string, payload: Record<string, unknown> = {}): Promise<T> => {
    const accessToken = await getAccessToken();
    return invoke<T>(command, { ...payload, accessToken });
  };

  // Fetch dashboard data on component mount
  useEffect(() => {
    if (!user) return;

    const loadDashboardData = async () => {
      setIsLoadingDashboard(true);
      try {
        const accessToken = await getAccessToken();
        const [remindersData, historyData, memoriesData] = await Promise.all([
          fetchReminders(accessToken),
          fetchCommandHistory(accessToken),
          fetchMemories(accessToken),
        ]);
        const integrationsData = await fetchIntegrations(accessToken);
        
        setReminders(remindersData);
        setCommandHistory(historyData);
        setMemories(memoriesData);
        setIntegrations(integrationsData);
      } finally {
        setIsLoadingDashboard(false);
      }
    };

    loadDashboardData();
  }, [getAccessToken, user]);

  // Handle reminder events globally so navigation works from any screen.
  useEffect(() => {
    if (!user) return;

    let unlisten: (() => void) | undefined;

    const setup = async () => {
      unlisten = await listen("reminder_fired", async (event: any) => {
        const reminderId = Number(event?.payload?.id);
        const reminderContent = event?.payload?.content || "You have a reminder.";

        const accessToken = await getAccessToken();
        if (event?.payload?.user_id && event.payload.user_id !== user.id) {
          return;
        }
        const remindersData = await fetchReminders(accessToken);
        setReminders(remindersData);
        setPendingReminderNavigation(true);

        if (Number.isFinite(reminderId)) {
          setActiveReminderAction({ id: reminderId, content: reminderContent });
        }

        // Show desktop notification using Tauri's notification API
        // This uses the system's native notification service
        try {
          await sendNotification({
            title: "Reminder",
            body: reminderContent,
          });
        } catch (notifError) {
          console.warn("Failed to send notification:", notifError);
        }
      });
    };

    setup();
    return () => {
      if (unlisten) unlisten();
    };
  }, [getAccessToken, user?.id]);

  // Desktop notification click usually focuses the app window.
  // When focus returns and a reminder fired, route to Reminders view once.
  useEffect(() => {
    const onFocus = () => {
      if (pendingReminderNavigation) {
        setCurrentView("reminders");
        setPendingReminderNavigation(false);
      }
    };

    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [pendingReminderNavigation]);

  const refreshReminderList = async () => {
    const accessToken = await getAccessToken();
    const remindersData = await fetchReminders(accessToken);
    setReminders(remindersData);
  };

  const finishActiveReminder = async () => {
    if (!activeReminderAction) return;
    setIsActionProcessing(true);
    try {
      await invokeAuthed("finish_reminder", { reminderId: activeReminderAction.id });
      await refreshReminderList();
      setActiveReminderAction(null);
      setPendingReminderNavigation(false);
    } catch (error) {
      console.error("Failed to finish reminder:", error);
    } finally {
      setIsActionProcessing(false);
    }
  };

  const snoozeActiveReminder = async (snoozeMinutes: number) => {
    if (!activeReminderAction) return;
    setIsActionProcessing(true);
    try {
      await invokeAuthed("snooze_reminder", {
        reminderId: activeReminderAction.id,
        snoozeMinutes,
      });
      await refreshReminderList();
      setActiveReminderAction(null);
      setPendingReminderNavigation(false);
    } catch (error) {
      console.error("Failed to snooze reminder:", error);
    } finally {
      setIsActionProcessing(false);
    }
  };

  if (loading) {
    return (
      <div style={{ minHeight: "100vh", display: "grid", placeItems: "center", background: "#0d0d0d", color: "#eaeaea" }}>
        Loading secure workspace...
      </div>
    );
  }

  if (!user) {
    if (authMode === "login") {
      return <LoginPage onLogin={login} onSwitchToSignup={() => setAuthMode("signup")} />;
    }
    return <SignupPage onSignup={signup} onSwitchToLogin={() => setAuthMode("login")} />;
  }

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
          <div style={{ marginTop: "12px", fontSize: "12px", color: "var(--text-secondary)" }}>{user.email}</div>
          <button
            type="button"
            onClick={() => void logout()}
            className="btn btn-secondary"
            style={{ marginTop: "12px", width: "100%" }}
          >
            Logout
          </button>
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
            <DashboardView key="dashboard" reminders={reminders} history={commandHistory} memories={memories} integrations={integrations} isLoading={isLoadingDashboard} />
          )}
          {currentView === "reminders" && (
            <RemindersView key="reminders" reminders={reminders} setReminders={setReminders} invokeAuthed={invokeAuthed} />
          )}
          {currentView === "history" && (
            <CommandHistoryView key="history" />
          )}
          {currentView === "memory-list" && (
            <MemoryListView key="memory-list" />
          )}
          {currentView === "memory-graph" && (
            <MemoryGraphView key="memory-graph" />
          )}
          {currentView === "memory" && (
            <MemoryView key="memory" memories={memories} setMemories={setMemories} searchQuery={searchQuery} setSearchQuery={setSearchQuery} invokeAuthed={invokeAuthed} />
          )}
          {currentView === "test" && (
            <TestCommandsView key="test" testResults={testResults} setTestResults={setTestResults} invokeAuthed={invokeAuthed} />
          )}
          {currentView === "integrations" && (
            <IntegrationsView key="integrations" integrations={integrations} setIntegrations={setIntegrations} invokeAuthed={invokeAuthed} />
          )}
          {currentView === "settings" && (
            <SettingsView key="settings" />
          )}
        </AnimatePresence>
      </main>

      <AnimatePresence>
        {activeReminderAction && (
          <motion.div
            initial={{ opacity: 0, y: 30 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 20 }}
            transition={{ duration: 0.2 }}
            style={{
              position: "fixed",
              right: "24px",
              bottom: "24px",
              width: "360px",
              zIndex: 2000,
              background: "rgba(20, 24, 31, 0.96)",
              border: "1px solid rgba(74, 158, 255, 0.35)",
              borderRadius: "14px",
              boxShadow: "0 12px 28px rgba(0, 0, 0, 0.35)",
              backdropFilter: "blur(8px)",
              padding: "16px",
            }}
          >
            <div style={{ fontSize: "12px", color: "#8fb8ff", marginBottom: "6px", letterSpacing: "0.2px" }}>
              Reminder
            </div>
            <div style={{ fontSize: "14px", color: "#f3f6fb", marginBottom: "14px", lineHeight: 1.4 }}>
              {activeReminderAction.content}
            </div>
            <div style={{ display: "flex", gap: "8px" }}>
              <button
                className="btn btn-secondary"
                onClick={() => snoozeActiveReminder(10)}
                disabled={isActionProcessing}
                style={{ flex: 1 }}
              >
                Snooze 10m
              </button>
              <button
                className="btn btn-primary"
                onClick={finishActiveReminder}
                disabled={isActionProcessing}
                style={{ flex: 1 }}
              >
                Finish
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

// ============================================================================
// DASHBOARD VIEW
// ============================================================================

function DashboardView({ reminders, history, memories, integrations, isLoading }: { reminders: Reminder[], history: CommandHistory[], memories: Memory[], integrations: Integration[], isLoading: boolean }) {
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

      {isLoading ? (
        <motion.div
          className="empty-state"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
        >
          <div style={{ fontSize: "48px", marginBottom: "16px", animation: "pulse 1.5s infinite" }}>⏳</div>
          <p>Loading your dashboard...</p>
        </motion.div>
      ) : (
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
            {integrations.length === 0 ? (
              <div className="list-item">
                <div style={{ fontSize: "13px", color: "var(--text-secondary)" }}>No plugins installed yet</div>
              </div>
            ) : (
              integrations.map((integration) => {
                const visual = getIntegrationVisual(integration);
                return (
                  <div key={integration.id} className="list-item">
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                      <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
                        <div style={{ color: visual.color, display: "flex", alignItems: "center" }}>
                          <visual.icon size={20} />
                        </div>
                        <span style={{ fontSize: "14px", color: "var(--text-primary)" }}>{integration.name}</span>
                      </div>
                      <span className={`badge ${integration.enabled ? "badge-success" : "badge-error"}`}>
                        {integration.enabled ? "Enabled" : "Disabled"}
                      </span>
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </motion.div>
      </div>
      )}
    </motion.div>
  );
}

// ============================================================================
// REMINDERS VIEW
// ============================================================================

function RemindersView({ reminders, setReminders, invokeAuthed }: { reminders: Reminder[], setReminders: React.Dispatch<React.SetStateAction<Reminder[]>>, invokeAuthed: <T>(command: string, payload?: Record<string, unknown>) => Promise<T> }) {
  const [showModal, setShowModal] = useState(false);
  const [reminderContent, setReminderContent] = useState("");
  const [reminderDate, setReminderDate] = useState("");
  const [reminderTime, setReminderTime] = useState("");
  const [isCreating, setIsCreating] = useState(false);
  const [currentTime, setCurrentTime] = useState(Math.floor(Date.now() / 1000));

  // Initialize date and time with tomorrow at current time
  useEffect(() => {
    const tomorrow = new Date();
    tomorrow.setDate(tomorrow.getDate() + 1);
    const dateStr = tomorrow.toISOString().split('T')[0];
    const timeStr = new Date().toTimeString().slice(0, 5);
    setReminderDate(dateStr);
    setReminderTime(timeStr);
  }, []);

  // Update current time every minute for dynamic time display
  useEffect(() => {
    const interval = setInterval(() => {
      setCurrentTime(Math.floor(Date.now() / 1000));
    }, 60000); // Update every 60 seconds

    return () => clearInterval(interval);
  }, []);

  // Calculate trigger_at timestamp from date and time inputs
  const calculateTriggerAt = () => {
    if (!reminderDate || !reminderTime) return null;
    const dateTimeStr = `${reminderDate}T${reminderTime}:00`;
    const timestamp = Math.floor(new Date(dateTimeStr).getTime() / 1000);
    return timestamp;
  };

  // Format relative time without seconds (minutes, hours, days only)
  const formatRelativeTime = (expiresAt: number) => {
    const diff = expiresAt - currentTime;
    
    if (diff < 0) {
      // Past time
      const absDiff = Math.abs(diff);
      if (absDiff < 3600) {
        const mins = Math.floor(absDiff / 60);
        return mins === 0 ? "just now" : `${mins} minute${mins !== 1 ? 's' : ''} ago`;
      } else if (absDiff < 86400) {
        const hours = Math.floor(absDiff / 3600);
        return `${hours} hour${hours !== 1 ? 's' : ''} ago`;
      } else {
        const days = Math.floor(absDiff / 86400);
        return `${days} day${days !== 1 ? 's' : ''} ago`;
      }
    } else {
      // Future time
      if (diff < 3600) {
        const mins = Math.floor(diff / 60);
        return mins === 0 ? "in less than a minute" : `in ${mins} minute${mins !== 1 ? 's' : ''}`;
      } else if (diff < 86400) {
        const hours = Math.floor(diff / 3600);
        return `in ${hours} hour${hours !== 1 ? 's' : ''}`;
      } else {
        const days = Math.floor(diff / 86400);
        return `in ${days} day${days !== 1 ? 's' : ''}`;
      }
    }
  };

  // Format Unix timestamp to absolute date/time (e.g., "Mar 5, 2026 3:45 PM")
  const formatAbsoluteDateTime = (timestamp: number) => {
    const date = new Date(timestamp * 1000);
    const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
    const month = months[date.getMonth()];
    const day = date.getDate();
    const year = date.getFullYear();
    let hours = date.getHours();
    const minutes = date.getMinutes();
    const ampm = hours >= 12 ? 'PM' : 'AM';
    hours = hours % 12;
    hours = hours ? hours : 12;
    const minuteStr = minutes < 10 ? '0' + minutes : minutes;
    return `${month} ${day}, ${year} ${hours}:${minuteStr} ${ampm}`;
  };

  const deleteReminder = async (id: string) => {
    try {
      await invokeAuthed<string>("finish_reminder", { reminderId: id });
    } catch (e) {
      console.warn("Failed to delete reminder from DB:", e);
    }
    setReminders(reminders.filter(r => r.id !== id));
  };

  const createReminder = async () => {
    if (!reminderContent.trim()) {
      alert("Please enter a reminder message");
      return;
    }

    if (!reminderDate || !reminderTime) {
      alert("Please select a date and time");
      return;
    }

    setIsCreating(true);
    try {
      const trigger_at = calculateTriggerAt();
      if (!trigger_at) {
        alert("Invalid date or time");
        setIsCreating(false);
        return;
      }

      const intentJson = JSON.stringify({
        name: "set_reminder",
        payload: {
          content: reminderContent,
          trigger_at: trigger_at
        }
      });

      const result = await invokeAuthed<{ success: boolean; message: string }>("execute_action", {
        intentJson,
      });

      if (result.success) {
        // Close modal silently without confirmation message
        setShowModal(false);
        setReminderContent("");
        const tomorrow = new Date();
        tomorrow.setDate(tomorrow.getDate() + 1);
        setReminderDate(tomorrow.toISOString().split('T')[0]);
        setReminderTime(new Date().toTimeString().slice(0, 5));
        
        // Refresh reminders list
        const updated = await invokeAuthed<Reminder[]>("get_reminders", { limit: 10 });
        setReminders(updated);
      } else {
        alert(`❌ Failed: ${result.message}`);
      }
    } catch (error) {
      alert(`❌ Error: ${error}`);
    } finally {
      setIsCreating(false);
    }
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
          <motion.button 
            className="btn btn-primary"
            onClick={() => setShowModal(true)}
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.98 }}
          >
            <Plus style={{ width: "16px", height: "16px" }} />
            New Reminder
          </motion.button>
        </div>
      </div>

      {/* New Reminder Modal */}
      <AnimatePresence>
        {showModal && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            style={{
              position: "fixed",
              top: 0,
              left: 0,
              right: 0,
              bottom: 0,
              background: "rgba(0, 0, 0, 0.7)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              zIndex: 1000,
            }}
            onClick={() => setShowModal(false)}
          >
            <motion.div
              initial={{ scale: 0.9, y: 20 }}
              animate={{ scale: 1, y: 0 }}
              exit={{ scale: 0.9, y: 20 }}
              style={{
                background: "var(--bg-tertiary)",
                borderRadius: "16px",
                padding: "32px",
                width: "90%",
                maxWidth: "500px",
                border: "1px solid var(--bg-elevated)",
              }}
              onClick={(e) => e.stopPropagation()}
            >
              <h2 style={{ fontSize: "24px", fontWeight: 600, marginBottom: "24px", color: "var(--text-primary)" }}>
                🔔 Create New Reminder
              </h2>

              <div style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
                <div>
                  <label style={{ display: "block", fontSize: "13px", fontWeight: 500, marginBottom: "8px", color: "var(--text-secondary)" }}>
                    What do you want to be reminded about?
                  </label>
                  <textarea
                    value={reminderContent}
                    onChange={(e) => setReminderContent(e.target.value)}
                    placeholder="e.g., Call mom, Take medicine, Team meeting..."
                    className="search-input"
                    style={{
                      minHeight: "80px",
                      resize: "vertical",
                      width: "100%",
                    }}
                  />
                </div>

                <div>
                  <label style={{ display: "block", fontSize: "13px", fontWeight: 500, marginBottom: "8px", color: "var(--text-secondary)" }}>
                    When should I remind you?
                  </label>
                  <div style={{ display: "flex", gap: "12px" }}>
                    <input
                      type="date"
                      value={reminderDate}
                      onChange={(e) => setReminderDate(e.target.value)}
                      className="search-input"
                      style={{ flex: 1 }}
                    />
                    <input
                      type="time"
                      value={reminderTime}
                      onChange={(e) => setReminderTime(e.target.value)}
                      className="search-input"
                      style={{ flex: 1 }}
                    />
                  </div>
                  <p style={{ fontSize: "12px", color: "var(--text-secondary)", marginTop: "8px" }}>
                    {reminderDate && reminderTime
                      ? (() => {
                          const trigger_at = calculateTriggerAt();
                          return trigger_at ? `Reminder set for ${formatAbsoluteDateTime(trigger_at)}` : "Invalid date/time";
                        })()
                      : "Select date and time"}
                  </p>
                </div>

                <div style={{ display: "flex", gap: "12px", marginTop: "12px" }}>
                  <motion.button
                    className="btn btn-primary"
                    onClick={createReminder}
                    disabled={isCreating}
                    whileHover={{ scale: 1.02 }}
                    whileTap={{ scale: 0.98 }}
                    style={{ flex: 1 }}
                  >
                    {isCreating ? "Creating..." : "Create Reminder"}
                  </motion.button>
                  <motion.button
                    className="btn btn-secondary"
                    onClick={() => setShowModal(false)}
                    whileHover={{ scale: 1.02 }}
                    whileTap={{ scale: 0.98 }}
                  >
                    Cancel
                  </motion.button>
                </div>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>

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
                      {formatRelativeTime(reminder.trigger_at)}
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
// MEMORY VIEW
// ============================================================================

function MemoryView({ 
  memories, 
  setMemories, 
  searchQuery, 
  setSearchQuery,
  invokeAuthed
}: { 
  memories: Memory[], 
  setMemories: React.Dispatch<React.SetStateAction<Memory[]>>, 
  searchQuery: string, 
  setSearchQuery: React.Dispatch<React.SetStateAction<string>>,
  invokeAuthed: <T>(command: string, payload?: Record<string, unknown>) => Promise<T>
}) {
  const filteredMemories = memories.filter(m => 
    m.content.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const deleteMemory = async (id: string) => {
    try {
      await invokeAuthed<string>("delete_memory", { memoryId: id });
    } catch (e) {
      console.warn("Failed to delete memory from DB:", e);
    }
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
  setIntegrations,
  invokeAuthed 
}: { 
  integrations: Integration[], 
  setIntegrations: React.Dispatch<React.SetStateAction<Integration[]>>,
  invokeAuthed: <T>(command: string, payload?: Record<string, unknown>) => Promise<T>
}) {
  const [selectedPluginId, setSelectedPluginId] = useState<string | null>(integrations[0]?.id ?? null);
  const [configDraft, setConfigDraft] = useState("{}");
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    if (!selectedPluginId && integrations.length > 0) {
      setSelectedPluginId(integrations[0].id);
      return;
    }

    if (selectedPluginId && !integrations.some((integration) => integration.id === selectedPluginId)) {
      setSelectedPluginId(integrations[0]?.id ?? null);
    }
  }, [integrations, selectedPluginId]);

  const selectedPlugin = integrations.find((integration) => integration.id === selectedPluginId) ?? null;

  useEffect(() => {
    setConfigDraft(selectedPlugin?.config_json || "{}");
  }, [selectedPluginId, selectedPlugin?.config_json]);

  const refreshIntegrations = async () => {
    const updated = await invokeAuthed<any[]>("get_plugins");
    setIntegrations(updated.map(mapPluginRecord));
  };

  const togglePlugin = async (plugin: Integration) => {
    setIsSaving(true);
    try {
      if (plugin.enabled) {
        await invokeAuthed("disable_plugin", { pluginId: plugin.id });
      } else {
        await invokeAuthed("enable_plugin", { pluginId: plugin.id });
      }
      await refreshIntegrations();
    } catch (error) {
      console.error("Failed to toggle plugin:", error);
      alert(`Failed to update plugin state: ${error}`);
    } finally {
      setIsSaving(false);
    }
  };

  const savePluginConfig = async () => {
    if (!selectedPlugin) return;

    setIsSaving(true);
    try {
      JSON.parse(configDraft || "{}");
      await invokeAuthed("update_plugin_config", {
        pluginId: selectedPlugin.id,
        configJson: configDraft,
      });
      await refreshIntegrations();
    } catch (error) {
      console.error("Failed to save plugin config:", error);
      alert(`Failed to save plugin config: ${error}`);
    } finally {
      setIsSaving(false);
    }
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
        <p className="panel-subtitle">Manage modular plugins and external service integrations</p>
      </div>

      <div className="grid grid-2" style={{ alignItems: "start" }}>
        <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
          {integrations.map((integration, index) => {
            const visual = getIntegrationVisual(integration);
            return (
              <motion.div
                key={integration.id}
                className="card"
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: index * 0.08 }}
                whileHover={{ y: -4 }}
                style={{
                  border: selectedPluginId === integration.id ? `1px solid ${visual.color}` : undefined,
                  cursor: "pointer",
                }}
                onClick={() => setSelectedPluginId(integration.id)}
              >
                <div style={{ display: "flex", alignItems: "start", justifyContent: "space-between", marginBottom: "20px", gap: "16px" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
                    <div style={{ width: "48px", height: "48px", borderRadius: "12px", background: `${visual.color}15`, display: "flex", alignItems: "center", justifyContent: "center", color: visual.color }}>
                      <visual.icon size={24} />
                    </div>
                    <div>
                      <h3 style={{ fontSize: "16px", fontWeight: 600, color: "var(--text-primary)", marginBottom: "4px" }}>{integration.name}</h3>
                      <p style={{ fontSize: "13px", color: "var(--text-secondary)", marginBottom: "8px", lineHeight: 1.5 }}>{integration.description}</p>
                      <span className={`badge ${integration.enabled ? "badge-success" : "badge-error"}`}>
                        {integration.enabled ? "Enabled" : "Disabled"}
                      </span>
                    </div>
                  </div>
                </div>
                <motion.button
                  className={`btn ${integration.enabled ? "btn-secondary" : "btn-primary"}`}
                  onClick={(event) => {
                    event.stopPropagation();
                    void togglePlugin(integration);
                  }}
                  whileHover={{ scale: 1.02 }}
                  whileTap={{ scale: 0.98 }}
                  style={{ width: "100%" }}
                  disabled={isSaving}
                >
                  {integration.enabled ? "Disable Plugin" : "Enable Plugin"}
                </motion.button>
              </motion.div>
            );
          })}
        </div>

        <motion.div
          className="card"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.15 }}
          style={{ position: "sticky", top: "24px" }}
        >
          {selectedPlugin ? (
            <>
              <div style={{ marginBottom: "20px" }}>
                <h3 style={{ fontSize: "18px", fontWeight: 600, color: "var(--text-primary)", marginBottom: "6px" }}>{selectedPlugin.name}</h3>
                <p style={{ fontSize: "13px", color: "var(--text-secondary)", lineHeight: 1.6 }}>{selectedPlugin.description}</p>
              </div>

              <div style={{ marginBottom: "20px" }}>
                <p style={{ fontSize: "12px", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.5px", marginBottom: "10px", fontWeight: 500 }}>Capabilities</p>
                <div style={{ display: "flex", flexWrap: "wrap", gap: "8px" }}>
                  {selectedPlugin.capabilities.map((capability) => (
                    <span key={capability} className="badge badge-primary">
                      {capability}
                    </span>
                  ))}
                </div>
              </div>

              <div>
                <p style={{ fontSize: "12px", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.5px", marginBottom: "10px", fontWeight: 500 }}>Configuration JSON</p>
                <textarea
                  value={configDraft}
                  onChange={(event) => setConfigDraft(event.target.value)}
                  className="search-input"
                  style={{ minHeight: "240px", width: "100%", resize: "vertical", fontFamily: "monospace", fontSize: "13px" }}
                />
                <p style={{ fontSize: "12px", color: "var(--text-secondary)", marginTop: "8px", lineHeight: 1.5 }}>
                  Example keys: Google Calendar uses calendar_id. Outlook uses task_list. Additional keys can be added without changing the core system.
                </p>
              </div>

              <motion.button
                className="btn btn-primary"
                onClick={() => void savePluginConfig()}
                whileHover={{ scale: 1.02 }}
                whileTap={{ scale: 0.98 }}
                style={{ width: "100%", marginTop: "16px" }}
                disabled={isSaving}
              >
                {isSaving ? "Saving..." : "Save Configuration"}
              </motion.button>
            </>
          ) : (
            <div className="empty-state">
              <Zap className="empty-state-icon" />
              <p>Select a plugin to view or edit its configuration</p>
            </div>
          )}
        </motion.div>
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

  // remind me to X [time spec]
  if (trimmed.startsWith("remind me to ")) {
    const remainder = input.slice(13).trim();
    let content = remainder;
    let delaySeconds = 3600; // Default: 1 hour

    // Parse time specifications: "in X minutes/hours/days", "tomorrow", "at HH:mm"
    const inMatch = remainder.match(/^(.+?)\s+in\s+(\d+)\s+(minute|hour|day)s?$/i);
    if (inMatch) {
      content = inMatch[1].trim();
      const amount = parseInt(inMatch[2], 10);
      const unit = inMatch[3].toLowerCase();
      if (unit === "minute") delaySeconds = amount * 60;
      else if (unit === "hour") delaySeconds = amount * 3600;
      else if (unit === "day") delaySeconds = amount * 86400;
    } else {
      const tomorrowMatch = remainder.match(/^(.+?)\s+tomorrow\s+at\s+(\d{1,2}):(\d{2})(am|pm)?$/i);
      if (tomorrowMatch) {
        content = tomorrowMatch[1].trim();
        let hour = parseInt(tomorrowMatch[2], 10);
        const minute = parseInt(tomorrowMatch[3], 10);
        const ampm = tomorrowMatch[4]?.toLowerCase();
        
        if (ampm === "pm" && hour !== 12) hour += 12;
        if (ampm === "am" && hour === 12) hour = 0;
        
        const tomorrow = new Date();
        tomorrow.setDate(tomorrow.getDate() + 1);
        tomorrow.setHours(hour, minute, 0, 0);
        delaySeconds = Math.floor((tomorrow.getTime() - Date.now()) / 1000);
      }
    }

    return JSON.stringify({ name: "set_reminder", payload: { content, trigger_at: Math.floor(Date.now() / 1000) + delaySeconds } });
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
  setTestResults,
  invokeAuthed,
}: { 
  testResults: TestCommandResult[], 
  setTestResults: React.Dispatch<React.SetStateAction<TestCommandResult[]>>,
  invokeAuthed: <T>(command: string, payload?: Record<string, unknown>) => Promise<T>,
}) {
  const [commandInput, setCommandInput] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  const executeTestCommand = async () => {
    if (!commandInput.trim()) return;

    setIsLoading(true);
    const startTime = performance.now();

    try {
      const intentJson = parseTestCommand(commandInput);
      const result = await invokeAuthed<TestCommandResponse>("execute_action", {
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

  const checkRemindersNow = async () => {
    try {
      const result = await invokeAuthed<string>("check_reminders_now");
      console.log("✓ Manual reminder check:", result);
      alert("Reminder check completed! Check console for details.");
    } catch (error) {
      console.error("❌ Reminder check failed:", error);
      alert(`Failed to check reminders: ${error}`);
    }
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
            
            <div style={{ display: "flex", gap: "12px" }}>
              <motion.button
                className="btn btn-secondary"
                onClick={checkRemindersNow}
                whileHover={{ scale: 1.02 }}
                whileTap={{ scale: 0.98 }}
                title="Manually trigger reminder check (for debugging)"
              >
                🔔 Check Reminders
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