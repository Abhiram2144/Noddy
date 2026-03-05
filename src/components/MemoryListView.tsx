import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import { Search, Trash2, Calendar, X } from "lucide-react";
import { useAuth } from "../auth/AuthContext";

type FilterTag = "all" | "work" | "personal" | "ideas" | "reminders";

interface Memory {
  id: string;
  content: string;
  timestamp: string;
  importance?: number;
  source?: string;
  tags?: string[];
}

interface MemoryListViewProps {
  onSelectMemory?: (memory: Memory) => void;
}

const DEBOUNCE_DELAY = 300;
const ANIMATION_NORMAL = 300;

const FILTER_PILLS: { label: string; value: FilterTag }[] = [
  { label: "All", value: "all" },
  { label: "Work", value: "work" },
  { label: "Personal", value: "personal" },
  { label: "Ideas", value: "ideas" },
  { label: "Reminders", value: "reminders" },
];

export function MemoryListView({ onSelectMemory }: MemoryListViewProps) {
  const { getAccessToken } = useAuth();
  const [memories, setMemories] = useState<Memory[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [activeFilter, setActiveFilter] = useState<FilterTag>("all");
  const [isLoading, setIsLoading] = useState(true);
  const [selectedMemory, setSelectedMemory] = useState<Memory | null>(null);
  const [detailPanelOpen, setDetailPanelOpen] = useState(false);
  const [debouncedSearch, setDebouncedSearch] = useState("");

  // Debounced search
  useEffect(() => {
    const timer = setTimeout(() => setDebouncedSearch(searchQuery), DEBOUNCE_DELAY);
    return () => clearTimeout(timer);
  }, [searchQuery]);

  // Fetch memories on mount
  useEffect(() => {
    fetchMemories();
  }, []);

  const fetchMemories = async () => {
    setIsLoading(true);
    try {
      const accessToken = await getAccessToken();
      const data = await invoke<any>("get_memories", { limit: 100, accessToken });
      if (Array.isArray(data)) {
        const formattedMemories = data.map((m: any) => ({
          id: m.id || Math.random().toString(),
          content: m.content || "",
          timestamp: m.timestamp || "Unknown",
          importance: m.importance || 0.5,
          source: m.source || "user_input",
          tags: m.tags ? (Array.isArray(m.tags) ? m.tags : []) : [],
        }));
        setMemories(formattedMemories);
      }
    } catch (error) {
      console.error("Failed to fetch memories:", error);
    } finally {
      setIsLoading(false);
    }
  };

  // Filter memories by search and category
  const filteredMemories = useMemo(() => {
    return memories.filter((memory) => {
      // Search filter
      const matchesSearch = memory.content
        .toLowerCase()
        .includes(debouncedSearch.toLowerCase());

      // Category filter
      let matchesCategory = true;
      if (activeFilter !== "all") {
        const tags = memory.tags || [];
        matchesCategory = tags.some((tag) =>
          tag.toLowerCase().includes(activeFilter)
        );
      }

      return matchesSearch && matchesCategory;
    });
  }, [memories, debouncedSearch, activeFilter]);

  const handleSelectMemory = (memory: Memory) => {
    setSelectedMemory(memory);
    setDetailPanelOpen(true);
    onSelectMemory?.(memory);
  };

  const handleDeleteMemory = (id: string) => {
    setMemories((prev) => prev.filter((m) => m.id !== id));
    if (selectedMemory?.id === id) {
      setDetailPanelOpen(false);
      setSelectedMemory(null);
    }
  };

  const containerVariants = {
    hidden: { opacity: 0 },
    show: {
      opacity: 1,
      transition: {
        staggerChildren: ANIMATION_NORMAL / 1000,
        delayChildren: 0.1,
      },
    },
  };

  const itemVariants = {
    hidden: { opacity: 0, translateY: 20 },
    show: {
      opacity: 1,
      translateY: 0,
      transition: { duration: ANIMATION_NORMAL / 1000 },
    },
    exit: { opacity: 0, translateY: -10, transition: { duration: 150 / 1000 } },
  };

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
        <h1 className="panel-title">Memory Bank</h1>
        <p className="panel-subtitle">Your stored knowledge and memories</p>
      </div>

      {/* Search Bar */}
      <motion.div
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: ANIMATION_NORMAL / 1000, delay: 100 / 1000 }}
        style={{ padding: "0 32px 20px 32px" }}
      >
        <div className="relative">
          <Search className="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-[#6c5ce7]/70" />
          <input
            type="text"
            placeholder="Search memories..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="search-input"
            style={{ paddingLeft: "48px", width: "100%" }}
          />
        </div>
      </motion.div>

      {/* Filter Pills */}
      <motion.div
        initial={{ opacity: 0, x: -20 }}
        animate={{ opacity: 1, x: 0 }}
        transition={{ duration: 250 / 1000, delay: 150 / 1000 }}
        style={{ padding: "0 32px 20px 32px", display: "flex", gap: "12px", overflowX: "auto" }}
      >
        {FILTER_PILLS.map((pill) => (
          <motion.button
            key={pill.value}
            onClick={() => setActiveFilter(pill.value)}
            whileHover={{ scale: 1.03 }}
            whileTap={{ scale: 0.97 }}
            className={activeFilter === pill.value ? "btn btn-primary" : "btn btn-secondary"}
            style={{ 
              whiteSpace: "nowrap",
              minWidth: "auto",
              ...(activeFilter === pill.value && {
                background: "linear-gradient(135deg, #6c5ce7 0%, #8b7bea 100%)",
                boxShadow: "0 4px 12px rgba(108, 92, 231, 0.3)",
              })
            }}
          >
            {pill.label}
          </motion.button>
        ))}
      </motion.div>

      {/* Memory Grid */}
      <div style={{ flex: 1, overflowY: "auto", padding: "0 32px 32px 32px" }}>
        {isLoading ? (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="empty-state"
          >
            <div style={{ fontSize: "48px", marginBottom: "16px", animation: "pulse 1.5s infinite" }}>⏳</div>
            <p>Loading memories...</p>
          </motion.div>
        ) : filteredMemories.length === 0 ? (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="empty-state"
          >
            <p>No memories found</p>
          </motion.div>
        ) : (
          <motion.div
            variants={containerVariants}
            initial="hidden"
            animate="show"
            style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(300px, 1fr))", gap: "20px" }}
          >
            <AnimatePresence mode="popLayout">
              {filteredMemories.map((memory) => (
                <motion.div
                  key={memory.id}
                  variants={itemVariants}
                  layout
                  onClick={() => handleSelectMemory(memory)}
                  className="card"
                  whileHover={{ y: -4 }}
                  style={{ cursor: "pointer", position: "relative" }}
                >
                  {/* Memory Text */}
                  <p style={{ fontSize: "14px", color: "var(--text-primary)", marginBottom: "16px", lineHeight: "1.6", display: "-webkit-box", WebkitLineClamp: 3, WebkitBoxOrient: "vertical", overflow: "hidden" }}>
                    {memory.content}
                  </p>

                  {/* Tags */}
                  {memory.tags && memory.tags.length > 0 && (
                    <div style={{ display: "flex", gap: "8px", flexWrap: "wrap", marginBottom: "16px" }}>
                      {memory.tags.slice(0, 2).map((tag) => (
                        <span
                          key={tag}
                          className="badge badge-primary"
                          style={{ fontSize: "11px" }}
                        >
                          {tag}
                        </span>
                      ))}
                    </div>
                  )}

                  {/* Footer */}
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", paddingTop: "12px", borderTop: "1px solid var(--bg-elevated)" }}>
                    <div style={{ display: "flex", alignItems: "center", gap: "8px", fontSize: "12px", color: "var(--text-secondary)" }}>
                      <Calendar style={{ width: "14px", height: "14px" }} />
                      <span>{memory.timestamp}</span>
                    </div>
                    <motion.button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleSelectMemory(memory);
                      }}
                      whileHover={{ scale: 1.04 }}
                      whileTap={{ scale: 0.97 }}
                      className="btn btn-secondary"
                      style={{ padding: "6px 10px", fontSize: "11px" }}
                    >
                      Details
                    </motion.button>
                    <motion.button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDeleteMemory(memory.id);
                      }}
                      whileHover={{ scale: 1.15 }}
                      whileTap={{ scale: 0.9 }}
                      style={{ padding: "8px", borderRadius: "8px", opacity: 0, transition: "opacity 0.2s", background: "transparent", border: "none", cursor: "pointer" }}
                      onMouseEnter={(e) => e.currentTarget.style.opacity = "1"}
                      onMouseLeave={(e) => e.currentTarget.style.opacity = "0"}
                    >
                      <Trash2 style={{ width: "16px", height: "16px", color: "var(--error)" }} />
                    </motion.button>
                  </div>
                </motion.div>
              ))}
            </AnimatePresence>
          </motion.div>
        )}
      </div>

      {/* Detail Panel */}
      <AnimatePresence>
        {detailPanelOpen && selectedMemory && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: ANIMATION_NORMAL / 1000 }}
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
              zIndex: 5000,
              pointerEvents: "auto",
            }}
            onClick={() => setDetailPanelOpen(false)}
          >
            <motion.div
              initial={{ scale: 0.9, y: 20 }}
              animate={{ scale: 1, y: 0 }}
              exit={{ scale: 0.9, y: 20 }}
              transition={{ duration: 350 / 1000, ease: "easeOut" }}
              style={{
                background: "var(--bg-tertiary)",
                borderRadius: "16px",
                padding: "32px",
                width: "90%",
                maxWidth: "600px",
                border: "1px solid var(--bg-elevated)",
                maxHeight: "80vh",
                overflowY: "auto",
              }}
              onClick={(e) => e.stopPropagation()}
            >
              {/* Panel Header */}
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "24px" }}>
                <h2 style={{ fontSize: "24px", fontWeight: 600, color: "var(--text-primary)" }}>Memory Details</h2>
                <motion.button
                  onClick={() => setDetailPanelOpen(false)}
                  whileHover={{ scale: 1.1, rotate: 90 }}
                  whileTap={{ scale: 0.9 }}
                  style={{ padding: "8px", background: "transparent", border: "none", cursor: "pointer", color: "var(--text-secondary)", display: "flex", alignItems: "center", justifyContent: "center", borderRadius: "8px" }}
                >
                  <X style={{ width: "20px", height: "20px" }} />
                </motion.button>
              </div>

              {/* Panel Content */}
              <div style={{ display: "flex", flexDirection: "column", gap: "24px" }}>
                {/* Full Text */}
                <div>
                  <p style={{ fontSize: "12px", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.5px", marginBottom: "12px", fontWeight: 500 }}>Content</p>
                  <p style={{ fontSize: "15px", color: "var(--text-primary)", lineHeight: "1.6" }}>{selectedMemory.content}</p>
                </div>

                {/* Tags */}
                {selectedMemory.tags && selectedMemory.tags.length > 0 && (
                  <div>
                    <p style={{ fontSize: "12px", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.5px", marginBottom: "12px", fontWeight: 500 }}>Tags</p>
                    <div style={{ display: "flex", gap: "8px", flexWrap: "wrap" }}>
                      {selectedMemory.tags.map((tag) => (
                        <span
                          key={tag}
                          className="badge badge-primary"
                        >
                          {tag}
                        </span>
                      ))}
                    </div>
                  </div>
                )}

                {/* Timestamp */}
                <div>
                  <p style={{ fontSize: "12px", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.5px", marginBottom: "8px", fontWeight: 500 }}>Created</p>
                  <p style={{ fontSize: "14px", color: "var(--text-primary)" }}>{selectedMemory.timestamp}</p>
                </div>

                {/* Importance */}
                <div>
                  <p style={{ fontSize: "12px", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.5px", marginBottom: "12px", fontWeight: 500 }}>Importance</p>
                  <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
                    <div style={{ flex: 1, background: "var(--bg-elevated)", borderRadius: "999px", height: "8px", overflow: "hidden" }}>
                      <motion.div
                        initial={{ width: 0 }}
                        animate={{ width: `${(selectedMemory.importance || 0.5) * 100}%` }}
                        transition={{ duration: 0.6, ease: "easeOut" }}
                        style={{ background: "linear-gradient(90deg, #6c5ce7, #a78bfa)", height: "100%", borderRadius: "999px" }}
                      />
                    </div>
                    <span style={{ fontSize: "14px", color: "var(--text-secondary)", minWidth: "3rem", textAlign: "right", fontWeight: 500 }}>
                      {((selectedMemory.importance || 0.5) * 100).toFixed(0)}%
                    </span>
                  </div>
                </div>

                {/* Delete Button */}
                <motion.button
                  onClick={() => {
                    handleDeleteMemory(selectedMemory.id);
                    setDetailPanelOpen(false);
                  }}
                  whileHover={{ scale: 1.02 }}
                  whileTap={{ scale: 0.98 }}
                  className="btn btn-secondary"
                  style={{ width: "100%", marginTop: "8px", color: "var(--error)", borderColor: "var(--error)" }}
                >
                  Delete Memory
                </motion.button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}
