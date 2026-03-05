import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import { Search, Trash2, Calendar, X } from "lucide-react";

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
      const data = await invoke<any>("get_memories", { limit: 100 });
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
    <div className="h-full flex flex-col bg-[#0d0d0d] text-[#eaeaea]">
      {/* Header */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: ANIMATION_NORMAL / 1000 }}
        className="px-8 py-6 bg-[#0d0d0d] border-b border-[#2a2a2a]"
      >
        <h1 className="text-3xl font-bold mb-1">Memory Bank</h1>
        <p className="text-[#aaaaaa] text-sm">Your stored knowledge and memories</p>
      </motion.div>

      {/* Search Bar */}
      <motion.div
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: ANIMATION_NORMAL / 1000, delay: 100 / 1000 }}
        className="px-8 py-4 border-b border-[#2a2a2a]"
      >
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[#6c5ce7]" />
          <input
            type="text"
            placeholder="Search memories..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full pl-10 pr-4 py-2.5 bg-[#161616] border border-[#2a2a2a] rounded-lg text-[#eaeaea] placeholder-[#aaaaaa] focus:outline-none focus:border-[#6c5ce7] transition-all duration-150"
          />
        </div>
      </motion.div>

      {/* Filter Pills */}
      <motion.div
        initial={{ opacity: 0, x: -20 }}
        animate={{ opacity: 1, x: 0 }}
        transition={{ duration: 250 / 1000, delay: 150 / 1000 }}
        className="px-8 py-4 flex gap-2 overflow-x-auto border-b border-[#2a2a2a]"
      >
        {FILTER_PILLS.map((pill) => (
          <motion.button
            key={pill.value}
            onClick={() => setActiveFilter(pill.value)}
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
            className={`px-4 py-2 rounded-full text-sm font-medium whitespace-nowrap transition-all duration-150 ${
              activeFilter === pill.value
                ? "bg-[#6c5ce7] text-white"
                : "bg-[#161616] text-[#aaaaaa] border border-[#2a2a2a] hover:border-[#6c5ce7] hover:text-[#eaeaea]"
            }`}
          >
            {pill.label}
          </motion.button>
        ))}
      </motion.div>

      {/* Memory Grid */}
      <div className="flex-1 overflow-y-auto px-8 py-6">
        {isLoading ? (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="flex items-center justify-center h-40"
          >
            <div className="flex flex-col items-center gap-3">
              <motion.div
                animate={{ rotate: 360 }}
                transition={{ duration: 2, repeat: Infinity, ease: "linear" }}
                className="w-8 h-8 border-2 border-[#6c5ce7] border-t-transparent rounded-full"
              />
              <p className="text-[#aaaaaa] text-sm">Loading memories...</p>
            </div>
          </motion.div>
        ) : filteredMemories.length === 0 ? (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="flex items-center justify-center h-40 text-[#aaaaaa]"
          >
            <p className="text-sm">No memories found</p>
          </motion.div>
        ) : (
          <motion.div
            variants={containerVariants}
            initial="hidden"
            animate="show"
            className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6"
          >
            <AnimatePresence mode="popLayout">
              {filteredMemories.map((memory) => (
                <motion.div
                  key={memory.id}
                  variants={itemVariants}
                  layout
                  onClick={() => handleSelectMemory(memory)}
                  className="bg-[#161616] border border-[#2a2a2a] rounded-lg p-4 cursor-pointer hover:border-[#6c5ce7] transition-all duration-150 group"
                  whileHover={{ scale: 1.04 }}
                >
                  {/* Memory Text */}
                  <p className="text-[#eaeaea] text-sm line-clamp-3 mb-3 leading-relaxed">
                    {memory.content}
                  </p>

                  {/* Tags */}
                  {memory.tags && memory.tags.length > 0 && (
                    <div className="flex gap-1 flex-wrap mb-3">
                      {memory.tags.slice(0, 2).map((tag) => (
                        <span
                          key={tag}
                          className="px-2 py-1 bg-[#6c5ce7]/20 text-[#6c5ce7] text-xs rounded"
                        >
                          {tag}
                        </span>
                      ))}
                    </div>
                  )}

                  {/* Footer */}
                  <div className="flex items-center justify-between pt-3 border-t border-[#2a2a2a]">
                    <div className="flex items-center gap-2 text-[#aaaaaa] text-xs">
                      <Calendar className="w-3 h-3" />
                      <span>{memory.timestamp}</span>
                    </div>
                    <motion.button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDeleteMemory(memory.id);
                      }}
                      whileHover={{ scale: 1.1 }}
                      whileTap={{ scale: 0.95 }}
                      className="p-1.5 rounded opacity-0 group-hover:opacity-100 transition-opacity"
                    >
                      <Trash2 className="w-3.5 h-3.5 text-red-400" />
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
            className="absolute inset-0 bg-black/50 z-40"
            onClick={() => setDetailPanelOpen(false)}
          >
            <motion.div
              initial={{ x: 420 }}
              animate={{ x: 0 }}
              exit={{ x: 420 }}
              transition={{ duration: 350 / 1000 }}
              className="fixed right-0 top-0 bottom-0 w-[420px] bg-[#161616] border-l border-[#2a2a2a] overflow-y-auto z-50"
              onClick={(e) => e.stopPropagation()}
            >
              {/* Panel Header */}
              <div className="px-6 py-4 border-b border-[#2a2a2a] flex items-center justify-between">
                <h2 className="text-lg font-semibold">Memory Details</h2>
                <motion.button
                  onClick={() => setDetailPanelOpen(false)}
                  whileHover={{ scale: 1.1 }}
                  whileTap={{ scale: 0.95 }}
                  className="p-1.5 text-[#aaaaaa] hover:text-[#eaeaea]"
                >
                  <X className="w-5 h-5" />
                </motion.button>
              </div>

              {/* Panel Content */}
              <div className="p-6 space-y-6">
                {/* Full Text */}
                <div>
                  <p className="text-[#aaaaaa] text-xs uppercase mb-2">Content</p>
                  <p className="text-[#eaeaea] leading-relaxed">{selectedMemory.content}</p>
                </div>

                {/* Tags */}
                {selectedMemory.tags && selectedMemory.tags.length > 0 && (
                  <div>
                    <p className="text-[#aaaaaa] text-xs uppercase mb-2">Tags</p>
                    <div className="flex gap-2 flex-wrap">
                      {selectedMemory.tags.map((tag) => (
                        <span
                          key={tag}
                          className="px-3 py-1 bg-[#6c5ce7]/20 text-[#6c5ce7] text-xs rounded"
                        >
                          {tag}
                        </span>
                      ))}
                    </div>
                  </div>
                )}

                {/* Timestamp */}
                <div>
                  <p className="text-[#aaaaaa] text-xs uppercase mb-1">Created</p>
                  <p className="text-[#eaeaea] text-sm">{selectedMemory.timestamp}</p>
                </div>

                {/* Importance */}
                <div>
                  <p className="text-[#aaaaaa] text-xs uppercase mb-2">Importance</p>
                  <div className="flex items-center gap-2">
                    <div className="flex-1 bg-[#2a2a2a] rounded-full h-2 overflow-hidden">
                      <div
                        className="bg-[#6c5ce7] h-full rounded-full"
                        style={{ width: `${(selectedMemory.importance || 0.5) * 100}%` }}
                      />
                    </div>
                    <span className="text-[#aaaaaa] text-sm">
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
                  className="w-full py-2.5 bg-red-500/20 border border-red-500/30 rounded-lg text-red-400 text-sm font-medium hover:bg-red-500/30 transition-colors"
                >
                  Delete Memory
                </motion.button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
