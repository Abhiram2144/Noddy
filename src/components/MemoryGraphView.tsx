import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { motion, AnimatePresence } from "framer-motion";
import { Search, RotateCcw, ZoomIn, ZoomOut, X } from "lucide-react";
import ForceGraph3D from "react-force-graph-3d";
import { useAuth } from "../auth/AuthContext";

interface GraphNode {
  id: string;
  label: string;
  content: string;
  importance: number;
  cluster_id: string;
  connection_count: number;
  access_count: number;
  recentness: number;
  x?: number;
  y?: number;
  z?: number;
}

interface GraphEdge {
  source: string | GraphNode;
  target: string | GraphNode;
  weight: number;
  relationship: string;
}

interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

interface MemoryGraphViewProps {
  onSelectMemory?: (memoryId: string) => void;
}

type MemoryCategory = "work" | "personal" | "ideas" | "reminders" | "general";

const CATEGORY_COLORS: Record<MemoryCategory, string> = {
  work: "#38bdf8",
  personal: "#fb7185",
  ideas: "#f59e0b",
  reminders: "#34d399",
  general: "#a78bfa",
};

const CATEGORY_LABELS: Record<MemoryCategory, string> = {
  work: "Work",
  personal: "Personal",
  ideas: "Ideas",
  reminders: "Reminders",
  general: "General",
};

const CATEGORY_KEYWORDS: Record<Exclude<MemoryCategory, "general">, string[]> = {
  work: ["meeting", "project", "client", "deadline", "task", "sprint", "jira", "work", "office"],
  personal: ["family", "mom", "dad", "friend", "home", "birthday", "personal", "health"],
  ideas: ["idea", "brainstorm", "concept", "plan", "build", "design", "future", "startup"],
  reminders: ["remind", "reminder", "tomorrow", "today", "schedule", "appointment", "call", "follow up"],
};

const ANIMATION_NORMAL = 300;

export function MemoryGraphView({ onSelectMemory }: MemoryGraphViewProps) {
  const { getAccessToken } = useAuth();
  const [graphData, setGraphData] = useState<GraphData>({ nodes: [], edges: [] });
  const [isLoading, setIsLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);
  const [detailPanelOpen, setDetailPanelOpen] = useState(false);
  const [zoom, setZoom] = useState(1);
  const graphRef = useRef<any>(null);
  const graphContainerRef = useRef<HTMLDivElement | null>(null);
  const [graphSize, setGraphSize] = useState({ width: 900, height: 560 });

  useEffect(() => {
    fetchGraphData();
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setup = async () => {
      unlisten = await listen("memory_saved", async () => {
        await fetchGraphData();
      });
    };

    setup();
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  // Keep graph canvas bounded to panel size instead of full window.
  useEffect(() => {
    const element = graphContainerRef.current;
    if (!element) return;

    const updateSize = () => {
      const rect = element.getBoundingClientRect();
      setGraphSize({
        width: Math.max(320, Math.floor(rect.width)),
        height: Math.max(360, Math.floor(rect.height)),
      });
    };

    updateSize();
    const observer = new ResizeObserver(updateSize);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const zoomCamera = (factor: number) => {
    if (!graphRef.current) return;
    const current = graphRef.current.cameraPosition();
    const nextZ = Math.max(70, Math.min(3500, current.z * factor));
    graphRef.current.cameraPosition({ x: current.x, y: current.y, z: nextZ }, undefined, 300);
    setZoom((z) => Math.max(0.2, Math.min(3, factor < 1 ? z + 0.15 : z - 0.15)));
  };

  const fetchGraphData = async () => {
    setIsLoading(true);
    try {
      const accessToken = await getAccessToken();
      const data = await invoke<any>("get_graph_data", { limit: 200, accessToken });
      if (data && data.nodes && data.edges) {
        setGraphData({
          nodes: data.nodes || [],
          edges: data.edges || [],
        });
        setSelectedNode((current) =>
          current ? (data.nodes || []).find((node: GraphNode) => node.id === current.id) ?? current : current
        );
      }
    } catch (error) {
      console.error("Failed to fetch graph data:", error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleNodeClick = async (node: GraphNode) => {
    setSelectedNode(node);
    setDetailPanelOpen(true);
    onSelectMemory?.(node.id);

    try {
      const accessToken = await getAccessToken();
      await invoke("track_memory_access", { memoryId: node.id, accessToken });
      await fetchGraphData();
    } catch (error) {
      console.error("Failed to track memory access:", error);
    }
  };

  const handleResetCamera = () => {
    if (graphRef.current) {
      graphRef.current.zoomToFit(400);
    }
  };

  const filteredNodes = searchQuery.trim()
    ? graphData.nodes.filter((n) =>
        n.label.toLowerCase().includes(searchQuery.toLowerCase()) ||
        n.content.toLowerCase().includes(searchQuery.toLowerCase())
      )
    : graphData.nodes;

  const filteredEdges = searchQuery.trim()
    ? graphData.edges.filter(
        (e) =>
          filteredNodes.some((n) => n.id === (typeof e.source === "string" ? e.source : e.source.id)) &&
          filteredNodes.some((n) => n.id === (typeof e.target === "string" ? e.target : e.target.id))
      )
    : graphData.edges;

  // Spread nodes apart and keep visible links between them.
  useEffect(() => {
    const graph = graphRef.current;
    if (!graph || isLoading) return;

    const linkForce = graph.d3Force("link");
    if (linkForce) {
      linkForce.distance(90);
      linkForce.strength(0.8);
    }

    const chargeForce = graph.d3Force("charge");
    if (chargeForce) {
      chargeForce.strength(-120);
    }

    const collisionForce = graph.d3Force("collision");
    if (collisionForce) {
      collisionForce.radius(16);
    }

    graph.d3ReheatSimulation();
  }, [isLoading, filteredNodes.length, filteredEdges.length]);

  const displayNodes: GraphNode[] = (() => {
    if (filteredNodes.length === 0) return [];

    // For small/medium graphs, seed positions so nodes don't overlap at origin.
    if (filteredNodes.length <= 30) {
      const radius = Math.max(120, filteredNodes.length * 20);
      return filteredNodes.map((node, index) => {
        const angle = (index / filteredNodes.length) * Math.PI * 2;
        return {
          ...node,
          x: Math.cos(angle) * radius,
          y: Math.sin(angle) * radius,
          z: ((index % 6) - 2.5) * 20,
        };
      });
    }

    if (filteredEdges.length > 0) return filteredNodes;

    // When there are no edges, seed a ring layout so nodes don't stack visually.
    const radius = Math.max(180, filteredNodes.length * 18);
    return filteredNodes.map((node, index) => {
      const angle = (index / filteredNodes.length) * Math.PI * 2;
      return {
        ...node,
        x: Math.cos(angle) * radius,
        y: Math.sin(angle) * radius,
        z: ((index % 4) - 1.5) * 30,
      };
    });
  })();

  useEffect(() => {
    if (!graphRef.current || isLoading || filteredNodes.length === 0) return;
    const timer = window.setTimeout(() => {
      graphRef.current.zoomToFit(400, 20);
      setZoom(1);
    }, 120);
    return () => window.clearTimeout(timer);
  }, [isLoading, filteredNodes.length]);

  const detectCategory = (node: GraphNode): MemoryCategory => {
    const text = `${node.label} ${node.content}`.toLowerCase();

    for (const keyword of CATEGORY_KEYWORDS.reminders) {
      if (text.includes(keyword)) return "reminders";
    }
    for (const keyword of CATEGORY_KEYWORDS.work) {
      if (text.includes(keyword)) return "work";
    }
    for (const keyword of CATEGORY_KEYWORDS.personal) {
      if (text.includes(keyword)) return "personal";
    }
    for (const keyword of CATEGORY_KEYWORDS.ideas) {
      if (text.includes(keyword)) return "ideas";
    }

    return "general";
  };

  const categoryForNode = (node: GraphNode): MemoryCategory => detectCategory(node);
  const categoryCount = new Set(graphData.nodes.map(categoryForNode)).size;

  const colorForCategory = (category: MemoryCategory) => CATEGORY_COLORS[category];
  const labelForCategory = (category: MemoryCategory) => CATEGORY_LABELS[category];

  const nodeColor = (node: GraphNode) => {
    if (node.id === selectedNode?.id) return "#ffffff";
    return colorForCategory(categoryForNode(node));
  };

  const nodeSize = (node: GraphNode) => {
    const baseSize = 8 + (node.importance || 0.5) * 10;
    return node.id === selectedNode?.id ? baseSize * 1.2 : baseSize;
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
        <h1 className="panel-title">Knowledge Graph</h1>
        <p className="panel-subtitle">Visualize category-based relationships and importance across your memories</p>
      </div>

      {/* Controls Bar */}
      <motion.div
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: ANIMATION_NORMAL / 1000 }}
        style={{ padding: "0 32px 20px 32px", display: "flex", alignItems: "center", gap: "16px" }}
      >
        <div style={{ flex: 1, position: "relative" }}>
          <Search style={{ position: "absolute", left: "16px", top: "50%", transform: "translateY(-50%)", width: "20px", height: "20px", color: "#6c5ce7", opacity: 0.7 }} />
          <input
            type="text"
            placeholder="Search nodes..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="search-input"
            style={{ paddingLeft: "48px", width: "100%" }}
          />
        </div>

        <motion.button
          onClick={handleResetCamera}
          whileHover={{ scale: 1.05, rotate: -180 }}
          whileTap={{ scale: 0.95 }}
          className="btn btn-secondary"
          title="Reset camera"
          style={{ padding: "12px" }}
        >
          <RotateCcw style={{ width: "20px", height: "20px" }} />
        </motion.button>

        <motion.button
          onClick={() => zoomCamera(0.85)}
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.95 }}
          className="btn btn-secondary"
          title="Zoom in"
          style={{ padding: "12px" }}
        >
          <ZoomIn style={{ width: "20px", height: "20px" }} />
        </motion.button>

        <motion.button
          onClick={() => zoomCamera(1.15)}
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.95 }}
          className="btn btn-secondary"
          title="Zoom out"
          style={{ padding: "12px" }}
        >
          <ZoomOut style={{ width: "20px", height: "20px" }} />
        </motion.button>
      </motion.div>

      {/* Graph Container */}
      <motion.div
        ref={graphContainerRef}
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: ANIMATION_NORMAL / 1000, delay: 100 / 1000 }}
        onWheel={(e) => {
          e.preventDefault();
          zoomCamera(e.deltaY > 0 ? 1.12 : 0.88);
        }}
        style={{
          flex: 1,
          position: "relative",
          background: "var(--bg-secondary)",
          overflow: "hidden",
          height: "min(68vh, 760px)",
          borderRadius: "16px",
          border: "1px solid var(--border-subtle)",
        }}
      >
        {isLoading ? (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="empty-state"
          >
            <div style={{ fontSize: "48px", marginBottom: "16px", animation: "pulse 1.5s infinite" }}>⏳</div>
            <p>Building knowledge graph...</p>
          </motion.div>
        ) : filteredNodes.length === 0 ? (
          <div className="empty-state">
            <p>No memories in graph</p>
          </div>
        ) : (
          <ForceGraph3D
            ref={graphRef}
            graphData={{
              nodes: displayNodes,
              links: filteredEdges.map((e) => ({
                source: typeof e.source === "string" ? e.source : e.source.id,
                target: typeof e.target === "string" ? e.target : e.target.id,
                value: e.weight,
                relationship: e.relationship,
              })),
            }}
            nodeLabel={(n: any) => `${n.label}\nImportance: ${Math.round((n.importance || 0) * 100)}%\nCategory: ${labelForCategory(categoryForNode(n as GraphNode))}`}
            nodeColor={nodeColor}
            nodeVal={nodeSize}
            nodeOpacity={1}
            nodeRelSize={11}
            nodeResolution={24}
            linkColor={(link: any) =>
              link.relationship === "keyword_similarity" ? "#cbd5e1" : "#94a3b8"
            }
            linkOpacity={0.98}
            linkWidth={(link: any) => 2.2 + (link.value || 0) * 2.6}
            onNodeClick={handleNodeClick}
            backgroundColor="#05070d"
            width={graphSize.width}
            height={graphSize.height}
            d3VelocityDecay={0.25}
            numDimensions={3}
            nodeVisibility={(n: any) => filteredNodes.some((fn) => fn.id === n.id)}
            warmupTicks={100}
            cooldownTicks={300}
            onEngineStop={() => {
              if (graphRef.current) {
                graphRef.current.zoomToFit(360, 30);
                setZoom(1);
              }
            }}
            enableNodeDrag
          />
        )}

        {/* Zoom Indicator */}
        {!isLoading && filteredNodes.length > 0 && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 300 / 1000 }}
            style={{
              position: "absolute",
              top: "24px",
              left: "32px",
              background: "var(--bg-tertiary)",
              border: "1px solid var(--bg-elevated)",
              borderRadius: "12px",
              padding: "12px 16px",
              fontSize: "13px",
              color: "var(--text-secondary)",
            }}
          >
            Zoom: {(zoom * 100).toFixed(0)}%
          </motion.div>
        )}

        {!isLoading && filteredNodes.length > 0 && (
          <div style={{ position: "absolute", top: "24px", right: "32px", fontSize: "12px", color: "var(--text-secondary)" }}>
            Click a node to open details
          </div>
        )}

        {/* Legend */}
        {!isLoading && filteredNodes.length > 0 && (
          <motion.div
            initial={{ opacity: 0, x: -20 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: 200 / 1000 }}
            className="card"
            style={{
              position: "absolute",
              bottom: "32px",
              left: "32px",
              padding: "20px",
              minWidth: "180px",
            }}
          >
            <p style={{ fontSize: "12px", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.5px", marginBottom: "16px", fontWeight: 500 }}>Category Colors</p>
            <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
              {Array.from(new Set(filteredNodes.map(categoryForNode))).map((category) => {
                const color = colorForCategory(category);
                return (
                <div key={category} style={{ display: "flex", alignItems: "center", gap: "12px" }}>
                  <div style={{ width: "12px", height: "12px", borderRadius: "50%", backgroundColor: color, boxShadow: `0 0 8px ${color}40` }} />
                  <span style={{ fontSize: "13px", color: "var(--text-primary)", textTransform: "capitalize" }}>{labelForCategory(category)}</span>
                </div>
              );})}
            </div>
          </motion.div>
        )}

        {/* Stats */}
        {!isLoading && filteredNodes.length > 0 && (
          <motion.div
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: 200 / 1000 }}
            className="card"
            style={{
              position: "absolute",
              bottom: "32px",
              right: "32px",
              padding: "20px",
              minWidth: "160px",
            }}
          >
            <div style={{ fontSize: "13px", color: "var(--text-secondary)", marginBottom: "8px" }}>
              <span style={{ fontSize: "20px", fontWeight: 600, color: "#a78bfa" }}>{filteredNodes.length}</span> nodes
            </div>
            <div style={{ fontSize: "13px", color: "var(--text-secondary)", marginBottom: "8px" }}>
              <span style={{ fontSize: "20px", fontWeight: 600, color: "#a78bfa" }}>{filteredEdges.length}</span> connections
            </div>
            <div style={{ fontSize: "13px", color: "var(--text-secondary)" }}>
              <span style={{ fontSize: "20px", fontWeight: 600, color: "#a78bfa" }}>{categoryCount}</span> categories
            </div>
          </motion.div>
        )}
      </motion.div>

      {/* Node Detail Panel */}
      <AnimatePresence>
        {detailPanelOpen && selectedNode && (
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
              transition={{ duration: 300 / 1000, ease: "easeOut" }}
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
              {/* Header */}
              <div style={{ display: "flex", alignItems: "start", justifyContent: "space-between", marginBottom: "24px" }}>
                <h3 style={{ fontSize: "24px", fontWeight: 600, color: "var(--text-primary)", flex: 1 }}>Node Details</h3>
                <motion.button
                  onClick={() => setDetailPanelOpen(false)}
                  whileHover={{ scale: 1.1, rotate: 90 }}
                  whileTap={{ scale: 0.9 }}
                  style={{ padding: "8px", background: "transparent", border: "none", cursor: "pointer", color: "var(--text-secondary)", display: "flex", alignItems: "center", justifyContent: "center", borderRadius: "8px" }}
                >
                  <X style={{ width: "20px", height: "20px" }} />
                </motion.button>
              </div>

              {/* Content */}
              <div style={{ display: "flex", flexDirection: "column", gap: "24px" }}>
                {/* Memory */}
                <div>
                  <p style={{ fontSize: "12px", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.5px", marginBottom: "8px", fontWeight: 500 }}>Memory</p>
                  <p style={{ fontSize: "15px", color: "var(--text-primary)", lineHeight: "1.6" }}>{selectedNode.content}</p>
                </div>

                {/* ID */}
                <div>
                  <p style={{ fontSize: "12px", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.5px", marginBottom: "8px", fontWeight: 500 }}>ID</p>
                  <code style={{ fontSize: "13px", color: "#a78bfa", fontFamily: "monospace", background: "rgba(108, 92, 231, 0.1)", padding: "6px 12px", borderRadius: "8px", display: "inline-block" }}>
                    {selectedNode.id.slice(0, 16)}...
                  </code>
                </div>

                {/* Importance */}
                <div>
                  <p style={{ fontSize: "12px", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.5px", marginBottom: "12px", fontWeight: 500 }}>Importance</p>
                  <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
                    <div style={{ flex: 1, background: "var(--bg-elevated)", borderRadius: "999px", height: "8px", overflow: "hidden" }}>
                      <motion.div
                        initial={{ width: 0 }}
                        animate={{ width: `${(selectedNode.importance || 0.5) * 100}%` }}
                        transition={{ duration: 0.6, ease: "easeOut" }}
                        style={{ background: "linear-gradient(90deg, #6c5ce7, #a78bfa)", height: "100%", borderRadius: "999px" }}
                      />
                    </div>
                    <span style={{ fontSize: "14px", color: "var(--text-secondary)", minWidth: "3rem", textAlign: "right", fontWeight: 500 }}>
                      {((selectedNode.importance || 0.5) * 100).toFixed(0)}%
                    </span>
                  </div>
                </div>

                <div>
                  <p style={{ fontSize: "12px", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.5px", marginBottom: "12px", fontWeight: 500 }}>Category</p>
                  <span
                    style={{
                      display: "inline-block",
                      padding: "8px 16px",
                      borderRadius: "8px",
                      color: "white",
                      fontSize: "14px",
                      fontWeight: 500,
                      backgroundColor: colorForCategory(categoryForNode(selectedNode)),
                      boxShadow: `0 4px 12px ${colorForCategory(categoryForNode(selectedNode))}40`
                    }}
                  >
                    {labelForCategory(categoryForNode(selectedNode))}
                  </span>
                </div>

                <div style={{ display: "grid", gridTemplateColumns: "repeat(3, minmax(0, 1fr))", gap: "12px" }}>
                  <div className="card" style={{ padding: "16px" }}>
                    <p style={{ fontSize: "12px", color: "var(--text-secondary)", marginBottom: "6px" }}>Connections</p>
                    <p style={{ fontSize: "20px", color: "var(--text-primary)", fontWeight: 600 }}>{selectedNode.connection_count}</p>
                  </div>
                  <div className="card" style={{ padding: "16px" }}>
                    <p style={{ fontSize: "12px", color: "var(--text-secondary)", marginBottom: "6px" }}>Accesses</p>
                    <p style={{ fontSize: "20px", color: "var(--text-primary)", fontWeight: 600 }}>{selectedNode.access_count}</p>
                  </div>
                  <div className="card" style={{ padding: "16px" }}>
                    <p style={{ fontSize: "12px", color: "var(--text-secondary)", marginBottom: "6px" }}>Recentness</p>
                    <p style={{ fontSize: "20px", color: "var(--text-primary)", fontWeight: 600 }}>{Math.round(selectedNode.recentness * 100)}%</p>
                  </div>
                </div>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}
