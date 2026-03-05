import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import { Search, RotateCcw, ZoomIn, ZoomOut, X } from "lucide-react";
import ForceGraph3D from "react-force-graph-3d";
import { useAuth } from "../auth/AuthContext";

interface GraphNode {
  id: string;
  label: string;
  importance: number;
  tag?: string;
}

interface GraphEdge {
  source: string | GraphNode;
  target: string | GraphNode;
  weight: number;
}

interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

interface MemoryGraphViewProps {
  onSelectMemory?: (memoryId: string) => void;
}

const TAG_COLORS: Record<string, string> = {
  work: "#3b82f6",
  personal: "#22c55e",
  idea: "#a855f7",
  reminder: "#f59e0b",
  default: "#6c5ce7",
};

const ANIMATION_NORMAL = 300;

export function MemoryGraphView({ onSelectMemory }: MemoryGraphViewProps) {
  const { getAccessToken } = useAuth();
  const [graphData, setGraphData] = useState<GraphData>({ nodes: [], edges: [] });
  const [isLoading, setIsLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);
  const [hoveredNodeId, setHoveredNodeId] = useState<string | null>(null);
  const [detailPanelOpen, setDetailPanelOpen] = useState(false);
  const [zoom, setZoom] = useState(1);
  const graphRef = useRef<any>(null);
  const graphContainerRef = useRef<HTMLDivElement | null>(null);
  const [graphSize, setGraphSize] = useState({ width: 900, height: 560 });

  useEffect(() => {
    fetchGraphData();
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
      const data = await invoke<any>("get_memory_graph", { limit: 200, accessToken });
      if (data && data.nodes && data.edges) {
        setGraphData({
          nodes: data.nodes || [],
          edges: data.edges || [],
        });
      }
    } catch (error) {
      console.error("Failed to fetch graph data:", error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleNodeClick = (node: any) => {
    setSelectedNode(node);
    setDetailPanelOpen(true);
    onSelectMemory?.(node.id);
  };

  const handleResetCamera = () => {
    if (graphRef.current) {
      graphRef.current.zoomToFit(400);
    }
  };

  const filteredNodes = searchQuery.trim()
    ? graphData.nodes.filter((n) =>
        n.label.toLowerCase().includes(searchQuery.toLowerCase())
      )
    : graphData.nodes;

  const filteredEdges = searchQuery.trim()
    ? graphData.edges.filter(
        (e) =>
          filteredNodes.some((n) => n.id === (typeof e.source === "string" ? e.source : e.source.id)) &&
          filteredNodes.some((n) => n.id === (typeof e.target === "string" ? e.target : e.target.id))
      )
    : graphData.edges;

  const nodeColor = (node: GraphNode) => {
    if (node.id === selectedNode?.id) return "#ddd6fe";
    if (node.id === hoveredNodeId) return "#b19cd9";
    return TAG_COLORS[node.tag || "default"];
  };

  const nodeSize = (node: GraphNode) => {
    const baseSize = 6 + (node.importance || 0.5) * 8;
    return node.id === selectedNode?.id || node.id === hoveredNodeId
      ? baseSize * 1.3
      : baseSize;
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
        <p className="panel-subtitle">Visualize connections between your memories</p>
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
              nodes: filteredNodes,
              links: filteredEdges.map((e) => ({
                source: typeof e.source === "string" ? e.source : e.source.id,
                target: typeof e.target === "string" ? e.target : e.target.id,
                value: e.weight,
              })),
            }}
            nodeLabel={(n: any) => n.label}
            nodeColor={nodeColor}
            nodeVal={nodeSize}
            linkColor={() => "#444444"}
            linkWidth={(link: any) => 0.5 + (link.value || 0) * 1.5}
            onNodeClick={handleNodeClick}
            onNodeHover={(node: any) => setHoveredNodeId(node ? node.id : null)}
            backgroundColor="var(--bg-secondary)"
            width={graphSize.width}
            height={graphSize.height}
            d3VelocityDecay={0.3}
            numDimensions={3}
            nodeVisibility={(n: any) => filteredNodes.some((fn) => fn.id === n.id)}
            warmupTicks={100}
            cooldownTicks={300}
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
            <p style={{ fontSize: "12px", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.5px", marginBottom: "16px", fontWeight: 500 }}>Tag Colors</p>
            <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
              {Object.entries(TAG_COLORS).map(([tag, color]) => (
                <div key={tag} style={{ display: "flex", alignItems: "center", gap: "12px" }}>
                  <div style={{ width: "12px", height: "12px", borderRadius: "50%", backgroundColor: color, boxShadow: `0 0 8px ${color}40` }} />
                  <span style={{ fontSize: "13px", color: "var(--text-primary)", textTransform: "capitalize" }}>{tag}</span>
                </div>
              ))}
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
            <div style={{ fontSize: "13px", color: "var(--text-secondary)" }}>
              <span style={{ fontSize: "20px", fontWeight: 600, color: "#a78bfa" }}>{filteredEdges.length}</span> connections
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
                {/* Label */}
                <div>
                  <p style={{ fontSize: "12px", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.5px", marginBottom: "8px", fontWeight: 500 }}>Label</p>
                  <p style={{ fontSize: "15px", color: "var(--text-primary)", lineHeight: "1.6", display: "-webkit-box", WebkitLineClamp: 3, WebkitBoxOrient: "vertical", overflow: "hidden" }}>{selectedNode.label}</p>
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

                {/* Tag */}
                {selectedNode.tag && (
                  <div>
                    <p style={{ fontSize: "12px", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.5px", marginBottom: "12px", fontWeight: 500 }}>Tag</p>
                    <span
                      style={{ 
                        display: "inline-block",
                        padding: "8px 16px",
                        borderRadius: "8px",
                        color: "white",
                        fontSize: "14px",
                        fontWeight: 500,
                        backgroundColor: TAG_COLORS[selectedNode.tag],
                        boxShadow: `0 4px 12px ${TAG_COLORS[selectedNode.tag]}40`
                      }}
                    >
                      {selectedNode.tag}
                    </span>
                  </div>
                )}
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}
