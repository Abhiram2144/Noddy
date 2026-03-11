import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion } from "framer-motion";
import {
  Settings,
  Shield,
  Sparkles,
  Bell,
  Zap,
  Database,
  Code2,
  Eye,
  Cpu,
  Wifi,
  MonitorDot,
  Terminal,
  CheckCircle2,
  AlertCircle,
  RefreshCw,
} from "lucide-react";

// ─── Types ────────────────────────────────────────────────────────────────────

interface AppSettings {
  first_run: boolean;
  onboarding_completed: boolean;
  theme: string;
  suggestions_enabled: boolean;
  auto_start: boolean;
}

interface UserPermissions {
  access_running_apps: boolean;
  launch_apps: boolean;
  plugin_access: boolean;
  network_access: boolean;
  background_suggestions: boolean;
}

interface SettingsPageProps {
  onNavigate: (view: string) => void;
  accessToken: string;
  onRestartOnboarding: () => void;
}

// ─── Section definitions ──────────────────────────────────────────────────────

const SECTIONS = [
  { id: "general", label: "General", icon: Settings },
  { id: "permissions", label: "Permissions", icon: Shield },
  { id: "ai", label: "AI & Assistant", icon: Sparkles },
  { id: "notifications", label: "Notifications", icon: Bell },
  { id: "plugins", label: "Plugins", icon: Zap },
  { id: "data", label: "Data & Privacy", icon: Database },
  { id: "developer", label: "Developer", icon: Code2 },
];

const PERMISSION_ITEMS: {
  key: keyof UserPermissions;
  label: string;
  description: string;
  icon: React.ReactNode;
}[] = [
  {
    key: "background_suggestions",
    label: "Smart Suggestions",
    description: "Allow proactive reminders and context surfacing while you work.",
    icon: <Sparkles size={16} />,
  },
  {
    key: "network_access",
    label: "Network Access",
    description: "Allow web search and AI model calls.",
    icon: <Wifi size={16} />,
  },
  {
    key: "plugin_access",
    label: "Plugin Integrations",
    description: "Allow plugins to connect to Calendar, Tasks, and other services.",
    icon: <Zap size={16} />,
  },
  {
    key: "access_running_apps",
    label: "Running App Awareness",
    description: "See which apps are currently open to provide smarter context.",
    icon: <MonitorDot size={16} />,
  },
  {
    key: "launch_apps",
    label: "Launch Applications",
    description: "Open apps on your behalf when asked.",
    icon: <Terminal size={16} />,
  },
];

// ─── Small shared components ──────────────────────────────────────────────────

function ToggleSwitch({
  checked,
  onChange,
  disabled = false,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <motion.button
      onClick={() => !disabled && onChange(!checked)}
      whileTap={disabled ? {} : { scale: 0.95 }}
      style={{
        width: "40px",
        height: "22px",
        borderRadius: "11px",
        background: checked ? "var(--accent-primary)" : "var(--border-medium)",
        border: "none",
        cursor: disabled ? "not-allowed" : "pointer",
        position: "relative",
        flexShrink: 0,
        opacity: disabled ? 0.5 : 1,
        transition: "background 0.2s ease",
      }}
    >
      <motion.div
        animate={{ x: checked ? 18 : 2 }}
        style={{
          position: "absolute",
          top: "2px",
          width: "18px",
          height: "18px",
          borderRadius: "50%",
          background: "#fff",
        }}
        transition={{ type: "spring", stiffness: 500, damping: 30 }}
      />
    </motion.button>
  );
}

function SectionCard({
  title,
  icon: Icon,
  children,
  delay = 0,
}: {
  title: string;
  icon: React.ElementType;
  children: React.ReactNode;
  delay?: number;
}) {
  return (
    <motion.div
      className="card"
      initial={{ opacity: 0, y: 16 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "10px",
          marginBottom: "20px",
          paddingBottom: "14px",
          borderBottom: "1px solid var(--border-medium)",
        }}
      >
        <div
          style={{
            width: "32px",
            height: "32px",
            borderRadius: "8px",
            background: "rgba(91,142,244,0.12)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            color: "var(--accent-primary)",
          }}
        >
          <Icon size={16} />
        </div>
        <h3
          style={{
            fontSize: "15px",
            fontWeight: 600,
            color: "var(--text-primary)",
          }}
        >
          {title}
        </h3>
      </div>
      {children}
    </motion.div>
  );
}

function SettingRow({
  label,
  description,
  children,
}: {
  label: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: "16px",
        padding: "10px 0",
        borderBottom: "1px solid rgba(255,255,255,0.04)",
      }}
    >
      <div style={{ flex: 1, minWidth: 0 }}>
        <p
          style={{
            fontSize: "13px",
            fontWeight: 500,
            color: "var(--text-primary)",
            marginBottom: description ? "2px" : 0,
          }}
        >
          {label}
        </p>
        {description && (
          <p style={{ fontSize: "12px", color: "var(--text-secondary)", lineHeight: 1.4 }}>
            {description}
          </p>
        )}
      </div>
      <div style={{ flexShrink: 0 }}>{children}</div>
    </div>
  );
}

function Toast({
  message,
  type,
}: {
  message: string;
  type: "success" | "error";
}) {
  return (
    <motion.div
      initial={{ opacity: 0, y: -12 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -12 }}
      style={{
        position: "fixed",
        top: "20px",
        right: "20px",
        display: "flex",
        alignItems: "center",
        gap: "8px",
        padding: "10px 16px",
        borderRadius: "10px",
        background: type === "success" ? "rgba(34,197,94,0.15)" : "rgba(239,68,68,0.15)",
        border: `1px solid ${type === "success" ? "rgba(34,197,94,0.3)" : "rgba(239,68,68,0.3)"}`,
        color: type === "success" ? "#4ade80" : "#f87171",
        fontSize: "13px",
        fontWeight: 500,
        zIndex: 9999,
        boxShadow: "0 4px 16px rgba(0,0,0,0.3)",
      }}
    >
      {type === "success" ? <CheckCircle2 size={15} /> : <AlertCircle size={15} />}
      {message}
    </motion.div>
  );
}

// ─── Main Component ───────────────────────────────────────────────────────────

export function SettingsPage({ onNavigate, accessToken, onRestartOnboarding }: SettingsPageProps) {
  const [activeSection, setActiveSection] = useState("general");
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [permissions, setPermissions] = useState<UserPermissions | null>(null);
  const [toast, setToast] = useState<{ message: string; type: "success" | "error" } | null>(null);
  const [saving, setSaving] = useState(false);
  const [rebuildingGraph, setRebuildingGraph] = useState(false);

  useEffect(() => {
    invoke<AppSettings>("get_settings")
      .then(setSettings)
      .catch((e) => console.error("Failed to load settings:", e));

    invoke<UserPermissions>("get_user_permissions")
      .then(setPermissions)
      .catch((e) => console.error("Failed to load permissions:", e));
  }, []);

  const showToast = useCallback(
    (message: string, type: "success" | "error") => {
      setToast({ message, type });
      setTimeout(() => setToast(null), 3000);
    },
    []
  );

  const updateSetting = useCallback(
    async <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => {
      if (!settings) return;
      const updated = { ...settings, [key]: value };
      setSaving(true);
      try {
        const saved = await invoke<AppSettings>("update_settings", {
          newSettings: updated,
        });
        setSettings(saved);
        showToast("Setting saved", "success");
      } catch (e) {
        showToast("Failed to save setting", "error");
        console.error(e);
      } finally {
        setSaving(false);
      }
    },
    [settings, showToast]
  );

  const updatePermission = useCallback(
    async (key: keyof UserPermissions, value: boolean) => {
      try {
        const updated = await invoke<UserPermissions>("update_user_permission", {
          permission: key,
          value,
        });
        setPermissions(updated);
        showToast("Permission updated", "success");
      } catch (e) {
        showToast("Failed to update permission", "error");
        console.error(e);
      }
    },
    [showToast]
  );

  const handleRebuildGraph = useCallback(async () => {
    setRebuildingGraph(true);
    try {
      await invoke("rebuild_memory_graph", { accessToken });
      showToast("Memory graph rebuilt successfully", "success");
    } catch (e) {
      showToast("Failed to rebuild graph", "error");
      console.error(e);
    } finally {
      setRebuildingGraph(false);
    }
  }, [accessToken, showToast]);

  if (!settings || !permissions) {
    return (
      <motion.div
        className="panel-container"
        initial={{ opacity: 0, x: 40 }}
        animate={{ opacity: 1, x: 0 }}
        exit={{ opacity: 0, x: -40 }}
        transition={{ duration: 0.4 }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            height: "200px",
            color: "var(--text-secondary)",
            fontSize: "14px",
          }}
        >
          Loading settings…
        </div>
      </motion.div>
    );
  }

  const renderSection = () => {
    switch (activeSection) {
      case "general":
        return (
          <SectionCard title="General" icon={Settings} delay={0.05}>
            <SettingRow
              label="Theme"
              description="Choose your preferred color scheme."
            >
              <select
                value={settings.theme}
                onChange={(e) => void updateSetting("theme", e.target.value)}
                className="search-input"
                style={{ fontSize: "13px", padding: "6px 10px", width: "130px" }}
                disabled={saving}
              >
                <option value="dark">Dark</option>
                <option value="light">Light</option>
                <option value="system">System</option>
              </select>
            </SettingRow>
            <SettingRow
              label="Smart Suggestions"
              description="Get proactive context-aware suggestions."
            >
              <ToggleSwitch
                checked={settings.suggestions_enabled}
                onChange={(v) => void updateSetting("suggestions_enabled", v)}
                disabled={saving}
              />
            </SettingRow>
            <SettingRow
              label="Auto Start"
              description="Launch Noddy automatically when you log in."
            >
              <ToggleSwitch
                checked={settings.auto_start}
                onChange={(v) => void updateSetting("auto_start", v)}
                disabled={saving}
              />
            </SettingRow>
          </SectionCard>
        );

      case "permissions":
        return (
          <SectionCard title="Permissions" icon={Shield} delay={0.05}>
            <p
              style={{
                fontSize: "13px",
                color: "var(--text-secondary)",
                marginBottom: "16px",
                lineHeight: 1.5,
              }}
            >
              Control what Noddy is allowed to do. Changes apply immediately.
            </p>
            {PERMISSION_ITEMS.map((item) => (
              <SettingRow key={item.key} label={item.label} description={item.description}>
                <ToggleSwitch
                  checked={permissions[item.key]}
                  onChange={(v) => void updatePermission(item.key, v)}
                />
              </SettingRow>
            ))}
          </SectionCard>
        );

      case "ai":
        return (
          <SectionCard title="AI & Assistant" icon={Sparkles} delay={0.05}>
            <SettingRow
              label="Smart Suggestions"
              description="Allow Noddy to surface proactive suggestions based on your context."
            >
              <ToggleSwitch
                checked={settings.suggestions_enabled}
                onChange={(v) => void updateSetting("suggestions_enabled", v)}
                disabled={saving}
              />
            </SettingRow>
            <SettingRow label="AI Model" description="The language model powering Noddy.">
              <span
                className="badge badge-primary"
                style={{ fontSize: "12px" }}
              >
                Gemini Flash
              </span>
            </SettingRow>
            <SettingRow
              label="API Key"
              description="Set via GEMINI_API_KEY environment variable in your .env file."
            >
              <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                <Eye size={14} color="var(--text-muted)" />
                <span style={{ fontSize: "12px", color: "var(--text-muted)", fontFamily: "monospace" }}>
                  ••••••••
                </span>
              </div>
            </SettingRow>
            <SettingRow label="Background Suggestions" description="Allow Noddy to generate context in the background.">
              <ToggleSwitch
                checked={permissions.background_suggestions}
                onChange={(v) => void updatePermission("background_suggestions", v)}
              />
            </SettingRow>
          </SectionCard>
        );

      case "notifications":
        return (
          <SectionCard title="Notifications" icon={Bell} delay={0.05}>
            <div
              style={{
                padding: "24px 0",
                textAlign: "center",
                color: "var(--text-secondary)",
              }}
            >
              <Bell size={28} style={{ opacity: 0.4, marginBottom: "10px" }} />
              <p style={{ fontSize: "13px" }}>
                Notification preferences coming soon. Reminders always notify when due.
              </p>
            </div>
          </SectionCard>
        );

      case "plugins":
        return (
          <SectionCard title="Plugins" icon={Zap} delay={0.05}>
            <p
              style={{
                fontSize: "13px",
                color: "var(--text-secondary)",
                marginBottom: "20px",
                lineHeight: 1.5,
              }}
            >
              Manage your plugin integrations and configurations.
            </p>
            <motion.button
              className="btn btn-primary"
              onClick={() => onNavigate("integrations")}
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
              style={{ display: "flex", alignItems: "center", gap: "8px" }}
            >
              <Zap size={15} />
              Open Integrations
            </motion.button>
          </SectionCard>
        );

      case "data":
        return (
          <SectionCard title="Data & Privacy" icon={Database} delay={0.05}>
            <SettingRow
              label="Storage Location"
              description="All data is stored locally on this device only."
            >
              <span
                className="badge"
                style={{
                  fontSize: "12px",
                  background: "rgba(34,197,94,0.12)",
                  color: "#4ade80",
                  border: "1px solid rgba(34,197,94,0.2)",
                  padding: "3px 8px",
                  borderRadius: "6px",
                }}
              >
                Local Only
              </span>
            </SettingRow>
            <SettingRow
              label="Memory Graph"
              description="Rebuild the relationship graph from stored memories."
            >
              <motion.button
                className="btn btn-secondary"
                onClick={() => void handleRebuildGraph()}
                disabled={rebuildingGraph}
                whileHover={{ scale: 1.02 }}
                whileTap={{ scale: 0.98 }}
                style={{ display: "flex", alignItems: "center", gap: "6px", fontSize: "12px", padding: "6px 12px" }}
              >
                <RefreshCw size={13} className={rebuildingGraph ? "spin" : ""} />
                {rebuildingGraph ? "Rebuilding…" : "Rebuild Graph"}
              </motion.button>
            </SettingRow>
            <div style={{ paddingTop: "16px", borderTop: "1px solid var(--border-medium)", marginTop: "6px" }}>
              <p style={{ fontSize: "12px", color: "var(--text-muted)", lineHeight: 1.5 }}>
                To delete all data, remove the Noddy app data directory. Individual memories can be deleted from the Memory List view.
              </p>
            </div>
          </SectionCard>
        );

      case "developer":
        return (
          <SectionCard title="Developer" icon={Code2} delay={0.05}>
            <SettingRow
              label="Test Commands"
              description="Open the command testing panel to test actions manually."
            >
              <motion.button
                className="btn btn-secondary"
                onClick={() => onNavigate("test")}
                whileHover={{ scale: 1.02 }}
                whileTap={{ scale: 0.98 }}
                style={{ fontSize: "12px", padding: "6px 12px", display: "flex", alignItems: "center", gap: "6px" }}
              >
                <Cpu size={13} />
                Open Test Panel
              </motion.button>
            </SettingRow>
            <SettingRow
              label="Rebuild Memory Graph"
              description="Force a full rebuild of the memory relationship graph."
            >
              <motion.button
                className="btn btn-secondary"
                onClick={() => void handleRebuildGraph()}
                disabled={rebuildingGraph}
                whileHover={{ scale: 1.02 }}
                whileTap={{ scale: 0.98 }}
                style={{ fontSize: "12px", padding: "6px 12px", display: "flex", alignItems: "center", gap: "6px" }}
              >
                <RefreshCw size={13} />
                Rebuild
              </motion.button>
            </SettingRow>
            <SettingRow
              label="Onboarding Wizard"
              description="Re-run the initial setup wizard."
            >
              <motion.button
                className="btn btn-secondary"
                onClick={onRestartOnboarding}
                whileHover={{ scale: 1.02 }}
                whileTap={{ scale: 0.98 }}
                style={{ fontSize: "12px", padding: "6px 12px", display: "flex", alignItems: "center", gap: "6px" }}
              >
                <Sparkles size={13} />
                Run Wizard
              </motion.button>
            </SettingRow>
            <div style={{ paddingTop: "16px", borderTop: "1px solid var(--border-medium)", marginTop: "6px" }}>
              <SettingRow label="Onboarding Status" description="">
                <span
                  className="badge"
                  style={{
                    fontSize: "12px",
                    background: settings.onboarding_completed
                      ? "rgba(34,197,94,0.12)"
                      : "rgba(234,179,8,0.12)",
                    color: settings.onboarding_completed ? "#4ade80" : "#facc15",
                    border: `1px solid ${settings.onboarding_completed ? "rgba(34,197,94,0.25)" : "rgba(234,179,8,0.25)"}`,
                    padding: "3px 8px",
                    borderRadius: "6px",
                  }}
                >
                  {settings.onboarding_completed ? "Completed" : "Pending"}
                </span>
              </SettingRow>
            </div>
          </SectionCard>
        );

      default:
        return null;
    }
  };

  return (
    <motion.div
      className="panel-container"
      initial={{ opacity: 0, x: 40 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: -40 }}
      transition={{ duration: 0.4 }}
      style={{ position: "relative" }}
    >
      {toast && <Toast message={toast.message} type={toast.type} />}

      <div className="panel-header">
        <h1 className="panel-title">Settings</h1>
        <p className="panel-subtitle">Configure your assistant</p>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "200px 1fr", gap: "20px", alignItems: "start" }}>
        {/* Section nav */}
        <motion.div
          className="card"
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.05 }}
          style={{ padding: "8px", position: "sticky", top: "0" }}
        >
          {SECTIONS.map((section) => {
            const Icon = section.icon;
            const isActive = activeSection === section.id;
            return (
              <motion.button
                key={section.id}
                onClick={() => setActiveSection(section.id)}
                whileHover={{ scale: 1.01 }}
                whileTap={{ scale: 0.99 }}
                style={{
                  width: "100%",
                  display: "flex",
                  alignItems: "center",
                  gap: "10px",
                  padding: "9px 12px",
                  borderRadius: "8px",
                  border: "none",
                  background: isActive ? "rgba(91,142,244,0.12)" : "transparent",
                  color: isActive ? "var(--accent-primary)" : "var(--text-secondary)",
                  fontSize: "13px",
                  fontWeight: isActive ? 600 : 400,
                  cursor: "pointer",
                  textAlign: "left",
                  transition: "all 0.15s ease",
                }}
              >
                <Icon size={15} />
                {section.label}
              </motion.button>
            );
          })}
        </motion.div>

        {/* Section content */}
        <div>{renderSection()}</div>
      </div>
    </motion.div>
  );
}
