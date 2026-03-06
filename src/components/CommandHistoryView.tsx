import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import { Clock, CheckCircle2, XCircle } from "lucide-react";
import { useAuth } from "../auth/AuthContext";

type FilterStatus = "all" | "success" | "failed";

interface CommandRecord {
  id: string;
  command: string;
  intent: string;
  timestamp: string;
  success: boolean;
  duration: number;
  status: string;
  error_message?: string;
}

const FILTER_PILLS: { label: string; value: FilterStatus }[] = [
  { label: "All", value: "all" },
  { label: "Success", value: "success" },
  { label: "Failed", value: "failed" },
];

export function CommandHistoryView() {
  const { getAccessToken } = useAuth();
  const [records, setRecords] = useState<CommandRecord[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [activeFilter, setActiveFilter] = useState<FilterStatus>("all");

  useEffect(() => {
    fetchHistory();
  }, []);

  const fetchHistory = async () => {
    setIsLoading(true);
    try {
      const accessToken = await getAccessToken();
      const data = await invoke<any[]>("get_command_history", {
        limit: 100,
        accessToken,
      });
      if (Array.isArray(data)) {
        setRecords(
          data.map((cmd) => ({
            id: cmd.id || Math.random().toString(),
            command: cmd.command || "Unknown command",
            intent: cmd.intent || "unknown",
            timestamp: cmd.timestamp || "Unknown",
            success: cmd.success !== false,
            duration: cmd.duration || 0,
            status: cmd.status || "completed",
            error_message: cmd.error_message ?? undefined,
          }))
        );
      }
    } catch (error) {
      console.error("Failed to fetch command history:", error);
    } finally {
      setIsLoading(false);
    }
  };

  const filteredRecords = useMemo(() => {
    if (activeFilter === "success") return records.filter((r) => r.success);
    if (activeFilter === "failed") return records.filter((r) => !r.success);
    return records;
  }, [records, activeFilter]);

  return (
    <motion.div
      className="panel-container"
      initial={{ opacity: 0, x: 40 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: -40 }}
      transition={{ duration: 0.4 }}
    >
      {/* Header */}
      <div className="panel-header">
        <h1 className="panel-title">Command History</h1>
        <p className="panel-subtitle">Recent command execution logs</p>
      </div>

      {/* Filter row */}
      <motion.div
        initial={{ opacity: 0, x: -20 }}
        animate={{ opacity: 1, x: 0 }}
        transition={{ duration: 0.25, delay: 0.1 }}
        style={{
          padding: "0 32px 20px 32px",
          display: "flex",
          gap: "12px",
          alignItems: "center",
        }}
      >
        {FILTER_PILLS.map((pill) => (
          <motion.button
            key={pill.value}
            onClick={() => setActiveFilter(pill.value)}
            whileHover={{ scale: 1.03 }}
            whileTap={{ scale: 0.97 }}
            className={
              activeFilter === pill.value ? "btn btn-primary" : "btn btn-secondary"
            }
            style={{
              whiteSpace: "nowrap",
              minWidth: "auto",
              ...(activeFilter === pill.value && {
                background: "linear-gradient(135deg, #6c5ce7 0%, #8b7bea 100%)",
                boxShadow: "0 4px 12px rgba(108, 92, 231, 0.3)",
              }),
            }}
          >
            {pill.label}
          </motion.button>
        ))}
        <div style={{ flex: 1 }} />
        <motion.button
          onClick={fetchHistory}
          whileHover={{ scale: 1.03 }}
          whileTap={{ scale: 0.97 }}
          className="btn btn-secondary"
          style={{ minWidth: "auto" }}
        >
          Refresh
        </motion.button>
      </motion.div>

      {/* Record count */}
      {!isLoading && (
        <div
          style={{
            padding: "0 32px 14px 32px",
            fontSize: "13px",
            color: "var(--text-secondary)",
          }}
        >
          Showing {filteredRecords.length} command
          {filteredRecords.length !== 1 ? "s" : ""}
        </div>
      )}

      {/* Records list */}
      <div style={{ flex: 1, overflowY: "auto", padding: "0 32px 32px 32px" }}>
        {isLoading ? (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="empty-state"
          >
            <div
              style={{
                fontSize: "48px",
                marginBottom: "16px",
                animation: "pulse 1.5s infinite",
              }}
            >
              ⏳
            </div>
            <p>Loading history...</p>
          </motion.div>
        ) : filteredRecords.length === 0 ? (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="empty-state"
          >
            <Clock
              style={{
                width: "48px",
                height: "48px",
                marginBottom: "16px",
                opacity: 0.4,
              }}
            />
            <p>
              {activeFilter === "all"
                ? "No commands executed yet"
                : `No ${activeFilter} commands found`}
            </p>
          </motion.div>
        ) : (
          <AnimatePresence mode="popLayout">
            <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
              {filteredRecords.map((record, index) => (
                <motion.div
                  key={record.id}
                  className="list-item"
                  initial={{ opacity: 0, y: 16 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -8 }}
                  transition={{ delay: Math.min(index * 0.03, 0.3) }}
                  whileHover={{ x: 4 }}
                >
                  <div
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "flex-start",
                    }}
                  >
                    {/* Left: command details */}
                    <div style={{ flex: 1, minWidth: 0 }}>
                      {/* Command text in mono block */}
                      <div
                        style={{
                          fontSize: "13px",
                          fontFamily: "monospace",
                          color: "var(--accent-primary)",
                          marginBottom: "8px",
                          background: "var(--bg-primary)",
                          padding: "8px 12px",
                          borderRadius: "6px",
                          wordBreak: "break-word",
                          whiteSpace: "pre-wrap",
                        }}
                      >
                        {record.command}
                      </div>

                      {/* Meta row */}
                      <div
                        style={{
                          fontSize: "12px",
                          color: "var(--text-secondary)",
                          display: "flex",
                          alignItems: "center",
                          gap: "16px",
                          flexWrap: "wrap",
                        }}
                      >
                        <span>
                          Intent:{" "}
                          <strong style={{ color: "var(--text-primary)" }}>
                            {record.intent}
                          </strong>
                        </span>
                        <span>
                          Duration:{" "}
                          <strong style={{ color: "var(--text-primary)" }}>
                            {record.duration}ms
                          </strong>
                        </span>
                        <span
                          style={{
                            display: "flex",
                            alignItems: "center",
                            gap: "4px",
                          }}
                        >
                          <Clock style={{ width: "12px", height: "12px" }} />
                          {record.timestamp}
                        </span>
                      </div>

                      {/* Error message (failed commands only) */}
                      {record.error_message && (
                        <div
                          style={{
                            marginTop: "8px",
                            fontSize: "12px",
                            color: "var(--error)",
                            background: "rgba(229,62,62,0.08)",
                            padding: "6px 10px",
                            borderRadius: "4px",
                          }}
                        >
                          {record.error_message}
                        </div>
                      )}
                    </div>

                    {/* Right: success / failed badge */}
                    <div style={{ marginLeft: "12px", flexShrink: 0 }}>
                      {record.success ? (
                        <span
                          className="badge badge-success"
                          style={{ display: "flex", alignItems: "center", gap: "4px" }}
                        >
                          <CheckCircle2 style={{ width: "12px", height: "12px" }} />
                          Success
                        </span>
                      ) : (
                        <span
                          className="badge badge-error"
                          style={{ display: "flex", alignItems: "center", gap: "4px" }}
                        >
                          <XCircle style={{ width: "12px", height: "12px" }} />
                          Failed
                        </span>
                      )}
                    </div>
                  </div>
                </motion.div>
              ))}
            </div>
          </AnimatePresence>
        )}
      </div>
    </motion.div>
  );
}
