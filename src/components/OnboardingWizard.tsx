import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import {
  Sparkles,
  Shield,
  Zap,
  CheckCircle2,
  ChevronRight,
  ChevronLeft,
  Eye,
  Cpu,
  Wifi,
  MonitorDot,
  Terminal,
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

interface Plugin {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  capabilities: string[];
}

interface OnboardingWizardProps {
  settings: AppSettings;
  accessToken: string;
  onComplete: () => void;
  onSkip: () => void;
}

// ─── Permission definitions ───────────────────────────────────────────────────

const PERMISSION_ITEMS: {
  key: keyof UserPermissions;
  label: string;
  description: string;
  icon: React.ReactNode;
  defaultValue: boolean;
}[] = [
  {
    key: "background_suggestions",
    label: "Smart Suggestions",
    description: "Allow Noddy to proactively surface helpful reminders and context while you work.",
    icon: <Sparkles size={18} />,
    defaultValue: true,
  },
  {
    key: "network_access",
    label: "Network Access",
    description: "Allow web search and AI model calls to answer your questions.",
    icon: <Wifi size={18} />,
    defaultValue: true,
  },
  {
    key: "plugin_access",
    label: "Plugin Integrations",
    description: "Let plugins connect to Calendar, Tasks, and other services you configure.",
    icon: <Zap size={18} />,
    defaultValue: true,
  },
  {
    key: "access_running_apps",
    label: "Running App Awareness",
    description: "Allow Noddy to see which apps are currently open to give smarter context.",
    icon: <MonitorDot size={18} />,
    defaultValue: false,
  },
  {
    key: "launch_apps",
    label: "Launch Applications",
    description: "Allow Noddy to open apps on your behalf when you ask it to.",
    icon: <Terminal size={18} />,
    defaultValue: false,
  },
];

// ─── Sub-components ───────────────────────────────────────────────────────────

function ProgressDots({ total, current }: { total: number; current: number }) {
  return (
    <div style={{ display: "flex", gap: "8px", justifyContent: "center", marginBottom: "32px" }}>
      {Array.from({ length: total }).map((_, i) => (
        <motion.div
          key={i}
          animate={{ width: i === current ? 24 : 8, opacity: i <= current ? 1 : 0.3 }}
          style={{
            height: "8px",
            borderRadius: "4px",
            background: i === current ? "var(--accent-primary)" : "var(--border-medium)",
            transition: "all 0.3s ease",
          }}
        />
      ))}
    </div>
  );
}

function ToggleSwitch({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <motion.button
      onClick={() => onChange(!checked)}
      whileTap={{ scale: 0.95 }}
      style={{
        width: "44px",
        height: "24px",
        borderRadius: "12px",
        background: checked ? "var(--accent-primary)" : "var(--border-medium)",
        border: "none",
        cursor: "pointer",
        position: "relative",
        flexShrink: 0,
        transition: "background 0.2s ease",
      }}
    >
      <motion.div
        animate={{ x: checked ? 20 : 2 }}
        style={{
          position: "absolute",
          top: "2px",
          width: "20px",
          height: "20px",
          borderRadius: "50%",
          background: "#fff",
        }}
        transition={{ type: "spring", stiffness: 500, damping: 30 }}
      />
    </motion.button>
  );
}

// ─── Step Screens ─────────────────────────────────────────────────────────────

function WelcomeStep({ onNext }: { onNext: () => void }) {
  return (
    <div style={{ textAlign: "center", padding: "8px 0" }}>
      <motion.div
        initial={{ scale: 0.8, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        transition={{ delay: 0.1, type: "spring", stiffness: 200 }}
        style={{
          width: "80px",
          height: "80px",
          borderRadius: "24px",
          background: "linear-gradient(135deg, var(--accent-primary), #7c3aed)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          margin: "0 auto 28px",
          boxShadow: "0 8px 32px rgba(91,142,244,0.35)",
        }}
      >
        <Cpu size={36} color="#fff" />
      </motion.div>

      <h1 style={{ fontSize: "28px", fontWeight: 700, color: "var(--text-primary)", marginBottom: "12px" }}>
        Welcome to Noddy
      </h1>
      <p style={{ fontSize: "15px", color: "var(--text-secondary)", lineHeight: 1.6, maxWidth: "380px", margin: "0 auto 40px" }}>
        Your personal AI assistant that lives on your desktop — private, fast, and deeply integrated with how you work.
      </p>

      <motion.button
        className="btn btn-primary"
        onClick={onNext}
        whileHover={{ scale: 1.02 }}
        whileTap={{ scale: 0.98 }}
        style={{ padding: "12px 40px", fontSize: "15px", gap: "8px", display: "inline-flex", alignItems: "center" }}
      >
        Get Started <ChevronRight size={18} />
      </motion.button>
    </div>
  );
}

function PrivacyStep({ onNext, onBack }: { onNext: () => void; onBack: () => void }) {
  const items = [
    { icon: <Eye size={18} />, title: "Your data stays local", desc: "Memories, reminders, and chat history are stored only on this device." },
    { icon: <Shield size={18} />, title: "No cloud sync", desc: "Nothing is uploaded to external servers unless you explicitly use a plugin that does so." },
    { icon: <Sparkles size={18} />, title: "AI calls are minimal", desc: "Only your chat message and relevant context are sent to the AI model — not your full history." },
  ];

  return (
    <div>
      <div style={{ textAlign: "center", marginBottom: "28px" }}>
        <div style={{ width: "52px", height: "52px", borderRadius: "16px", background: "rgba(91,142,244,0.12)", display: "flex", alignItems: "center", justifyContent: "center", margin: "0 auto 16px" }}>
          <Shield size={24} color="var(--accent-primary)" />
        </div>
        <h2 style={{ fontSize: "22px", fontWeight: 700, color: "var(--text-primary)", marginBottom: "8px" }}>Privacy First</h2>
        <p style={{ fontSize: "14px", color: "var(--text-secondary)" }}>Here's how Noddy handles your data.</p>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: "12px", marginBottom: "36px" }}>
        {items.map((item, i) => (
          <motion.div
            key={i}
            initial={{ opacity: 0, x: -16 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: i * 0.1 }}
            style={{
              display: "flex",
              gap: "14px",
              padding: "14px 16px",
              background: "var(--bg-secondary)",
              borderRadius: "10px",
              border: "1px solid var(--border-medium)",
              alignItems: "flex-start",
            }}
          >
            <div style={{ color: "var(--accent-primary)", marginTop: "2px", flexShrink: 0 }}>{item.icon}</div>
            <div>
              <p style={{ fontSize: "14px", fontWeight: 600, color: "var(--text-primary)", marginBottom: "2px" }}>{item.title}</p>
              <p style={{ fontSize: "13px", color: "var(--text-secondary)", lineHeight: 1.5 }}>{item.desc}</p>
            </div>
          </motion.div>
        ))}
      </div>

      <NavButtons onBack={onBack} onNext={onNext} nextLabel="Understood" />
    </div>
  );
}

function PermissionsStep({
  permissions,
  onToggle,
  onNext,
  onBack,
}: {
  permissions: UserPermissions;
  onToggle: (key: keyof UserPermissions, value: boolean) => void;
  onNext: () => void;
  onBack: () => void;
}) {
  return (
    <div>
      <div style={{ textAlign: "center", marginBottom: "24px" }}>
        <div style={{ width: "52px", height: "52px", borderRadius: "16px", background: "rgba(91,142,244,0.12)", display: "flex", alignItems: "center", justifyContent: "center", margin: "0 auto 16px" }}>
          <Shield size={24} color="var(--accent-primary)" />
        </div>
        <h2 style={{ fontSize: "22px", fontWeight: 700, color: "var(--text-primary)", marginBottom: "6px" }}>Permissions</h2>
        <p style={{ fontSize: "13px", color: "var(--text-secondary)" }}>Choose what Noddy is allowed to do. You can change these at any time in Settings.</p>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: "10px", marginBottom: "32px" }}>
        {PERMISSION_ITEMS.map((item, i) => (
          <motion.div
            key={item.key}
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: i * 0.06 }}
            style={{
              display: "flex",
              alignItems: "center",
              gap: "14px",
              padding: "12px 14px",
              background: "var(--bg-secondary)",
              borderRadius: "10px",
              border: "1px solid var(--border-medium)",
            }}
          >
            <div style={{ color: "var(--accent-primary)", flexShrink: 0 }}>{item.icon}</div>
            <div style={{ flex: 1, minWidth: 0 }}>
              <p style={{ fontSize: "14px", fontWeight: 600, color: "var(--text-primary)", marginBottom: "2px" }}>{item.label}</p>
              <p style={{ fontSize: "12px", color: "var(--text-secondary)", lineHeight: 1.4 }}>{item.description}</p>
            </div>
            <ToggleSwitch checked={permissions[item.key]} onChange={(v) => onToggle(item.key, v)} />
          </motion.div>
        ))}
      </div>

      <NavButtons onBack={onBack} onNext={onNext} nextLabel="Continue" />
    </div>
  );
}

function PluginsStep({
  plugins,
  togglingId,
  onToggle,
  onNext,
  onBack,
}: {
  plugins: Plugin[];
  togglingId: string | null;
  onToggle: (plugin: Plugin) => void;
  onNext: () => void;
  onBack: () => void;
}) {
  return (
    <div>
      <div style={{ textAlign: "center", marginBottom: "24px" }}>
        <div style={{ width: "52px", height: "52px", borderRadius: "16px", background: "rgba(91,142,244,0.12)", display: "flex", alignItems: "center", justifyContent: "center", margin: "0 auto 16px" }}>
          <Zap size={24} color="var(--accent-primary)" />
        </div>
        <h2 style={{ fontSize: "22px", fontWeight: 700, color: "var(--text-primary)", marginBottom: "6px" }}>Plugins</h2>
        <p style={{ fontSize: "13px", color: "var(--text-secondary)" }}>Enable integrations you use. More can be configured later in Integrations.</p>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: "10px", marginBottom: "32px", maxHeight: "280px", overflowY: "auto" }}>
        {plugins.length === 0 ? (
          <p style={{ textAlign: "center", color: "var(--text-secondary)", fontSize: "14px", padding: "20px 0" }}>No plugins available.</p>
        ) : (
          plugins.map((plugin, i) => (
            <motion.div
              key={plugin.id}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: i * 0.06 }}
              style={{
                display: "flex",
                alignItems: "center",
                gap: "14px",
                padding: "12px 14px",
                background: "var(--bg-secondary)",
                borderRadius: "10px",
                border: "1px solid var(--border-medium)",
              }}
            >
              <div style={{ flex: 1, minWidth: 0 }}>
                <p style={{ fontSize: "14px", fontWeight: 600, color: "var(--text-primary)", marginBottom: "2px" }}>{plugin.name}</p>
                <p style={{ fontSize: "12px", color: "var(--text-secondary)", lineHeight: 1.4 }}>{plugin.description}</p>
              </div>
              <ToggleSwitch
                checked={plugin.enabled}
                onChange={() => onToggle(plugin)}
              />
              {togglingId === plugin.id && (
                <div style={{ width: "16px", height: "16px", borderRadius: "50%", border: "2px solid var(--accent-primary)", borderTopColor: "transparent", animation: "spin 0.6s linear infinite", flexShrink: 0 }} />
              )}
            </motion.div>
          ))
        )}
      </div>

      <NavButtons onBack={onBack} onNext={onNext} nextLabel="Almost done" />
    </div>
  );
}

function CompleteStep({ onComplete }: { onComplete: () => void }) {
  return (
    <div style={{ textAlign: "center", padding: "8px 0" }}>
      <motion.div
        initial={{ scale: 0, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        transition={{ delay: 0.1, type: "spring", stiffness: 200 }}
        style={{
          width: "80px",
          height: "80px",
          borderRadius: "50%",
          background: "linear-gradient(135deg, #22c55e, #16a34a)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          margin: "0 auto 24px",
          boxShadow: "0 8px 32px rgba(34,197,94,0.3)",
        }}
      >
        <CheckCircle2 size={40} color="#fff" />
      </motion.div>

      <h2 style={{ fontSize: "26px", fontWeight: 700, color: "var(--text-primary)", marginBottom: "10px" }}>You're all set!</h2>
      <p style={{ fontSize: "14px", color: "var(--text-secondary)", lineHeight: 1.6, maxWidth: "360px", margin: "0 auto 40px" }}>
        Noddy is ready. Start a conversation in the Chat tab, or explore your memory and reminders. Settings can be changed any time.
      </p>

      <motion.button
        className="btn btn-primary"
        onClick={onComplete}
        whileHover={{ scale: 1.02 }}
        whileTap={{ scale: 0.98 }}
        style={{ padding: "12px 40px", fontSize: "15px" }}
      >
        Open Noddy
      </motion.button>
    </div>
  );
}

function NavButtons({
  onBack,
  onNext,
  nextLabel = "Next",
  loading = false,
}: {
  onBack: () => void;
  onNext: () => void;
  nextLabel?: string;
  loading?: boolean;
}) {
  return (
    <div style={{ display: "flex", gap: "12px", justifyContent: "space-between" }}>
      <motion.button
        className="btn btn-secondary"
        onClick={onBack}
        whileHover={{ scale: 1.02 }}
        whileTap={{ scale: 0.98 }}
        style={{ display: "flex", alignItems: "center", gap: "6px" }}
      >
        <ChevronLeft size={16} /> Back
      </motion.button>
      <motion.button
        className="btn btn-primary"
        onClick={onNext}
        disabled={loading}
        whileHover={{ scale: 1.02 }}
        whileTap={{ scale: 0.98 }}
        style={{ display: "flex", alignItems: "center", gap: "6px" }}
      >
        {nextLabel} <ChevronRight size={16} />
      </motion.button>
    </div>
  );
}

// ─── Main Wizard ──────────────────────────────────────────────────────────────

export function OnboardingWizard({ settings, accessToken, onComplete, onSkip }: OnboardingWizardProps) {
  const TOTAL_STEPS = 5;
  const [step, setStep] = useState(0);
  const [direction, setDirection] = useState(1);
  const [permissions, setPermissions] = useState<UserPermissions>({
    access_running_apps: false,
    launch_apps: false,
    plugin_access: true,
    network_access: true,
    background_suggestions: true,
  });
  const [plugins, setPlugins] = useState<Plugin[]>([]);
  const [togglingId, setTogglingId] = useState<string | null>(null);

  useEffect(() => {
    // Load current permissions
    invoke<UserPermissions>("get_user_permissions")
      .then(setPermissions)
      .catch(console.error);

    // Load plugins
    invoke<Plugin[]>("get_plugins", { accessToken })
      .then(setPlugins)
      .catch(console.error);
  }, [accessToken]);

  const goNext = useCallback(() => {
    setDirection(1);
    setStep((s) => Math.min(s + 1, TOTAL_STEPS - 1));
  }, []);

  const goBack = useCallback(() => {
    setDirection(-1);
    setStep((s) => Math.max(s - 1, 0));
  }, []);

  const handlePermissionToggle = useCallback(
    async (key: keyof UserPermissions, value: boolean) => {
      try {
        const updated = await invoke<UserPermissions>("update_user_permission", {
          permission: key,
          value,
        });
        setPermissions(updated);
      } catch (e) {
        console.error("Failed to update permission:", e);
      }
    },
    []
  );

  const handlePluginToggle = useCallback(
    async (plugin: Plugin) => {
      setTogglingId(plugin.id);
      try {
        if (plugin.enabled) {
          await invoke("disable_plugin", { pluginId: plugin.id, accessToken });
        } else {
          await invoke("enable_plugin", { pluginId: plugin.id, accessToken });
        }
        setPlugins((prev) =>
          prev.map((p) => (p.id === plugin.id ? { ...p, enabled: !p.enabled } : p))
        );
      } catch (e) {
        console.error("Failed to toggle plugin:", e);
      } finally {
        setTogglingId(null);
      }
    },
    [accessToken]
  );

  const handleComplete = useCallback(async () => {
    try {
      await invoke("update_settings", {
        newSettings: {
          ...settings,
          first_run: false,
          onboarding_completed: true,
        },
      });
    } catch (e) {
      console.error("Failed to save onboarding completion:", e);
    }
    onComplete();
  }, [settings, onComplete]);

  const variants = {
    enter: (d: number) => ({ x: d > 0 ? 60 : -60, opacity: 0 }),
    center: { x: 0, opacity: 1 },
    exit: (d: number) => ({ x: d > 0 ? -60 : 60, opacity: 0 }),
  };

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.75)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 1000,
        backdropFilter: "blur(6px)",
      }}
    >
      <motion.div
        initial={{ opacity: 0, scale: 0.95, y: 20 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        style={{
          width: "100%",
          maxWidth: "480px",
          background: "var(--bg-primary)",
          border: "1px solid var(--border-medium)",
          borderRadius: "20px",
          padding: "36px 40px",
          position: "relative",
          boxShadow: "0 24px 64px rgba(0,0,0,0.6)",
          maxHeight: "90vh",
          overflowY: "auto",
        }}
      >
        {step !== 0 && step !== TOTAL_STEPS - 1 && (
          <button
            onClick={onSkip}
            style={{
              position: "absolute",
              top: "16px",
              right: "20px",
              background: "none",
              border: "none",
              color: "var(--text-muted)",
              fontSize: "13px",
              cursor: "pointer",
              padding: "4px 8px",
            }}
          >
            Skip setup
          </button>
        )}

        <ProgressDots total={TOTAL_STEPS} current={step} />

        <AnimatePresence mode="wait" custom={direction}>
          <motion.div
            key={step}
            custom={direction}
            variants={variants}
            initial="enter"
            animate="center"
            exit="exit"
            transition={{ duration: 0.25, ease: "easeInOut" }}
          >
            {step === 0 && <WelcomeStep onNext={goNext} />}
            {step === 1 && <PrivacyStep onNext={goNext} onBack={goBack} />}
            {step === 2 && (
              <PermissionsStep
                permissions={permissions}
                onToggle={handlePermissionToggle}
                onNext={goNext}
                onBack={goBack}
              />
            )}
            {step === 3 && (
              <PluginsStep
                plugins={plugins}
                togglingId={togglingId}
                onToggle={handlePluginToggle}
                onNext={goNext}
                onBack={goBack}
              />
            )}
            {step === 4 && <CompleteStep onComplete={handleComplete} />}
          </motion.div>
        </AnimatePresence>
      </motion.div>
    </div>
  );
}
