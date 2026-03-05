import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import { Search, RotateCcw, ZoomIn, ZoomOut, X } from "lucide-react";
import ForceGraph3D from "react-force-graph-3d";

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
  const [graphData, setGraphData] = useState<GraphData>({ nodes: [], edges: [] });
  const [isLoading, setIsLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);
  const [hoveredNodeId, setHoveredNodeId] = useState<string | null>(null);
  const [detailPanelOpen, setDetailPanelOpen] = useState(false);
  const [zoom, setZoom] = useState(1);
  const graphRef = useRef<any>(null);

  useEffect(() => {
    fetchGraphData();
  }, []);

  // Handle zoom with mouse wheel
  useEffect(() => {
    const handleWheel = (e: WheelEvent) => {
      if (graphRef.current) {
        const direction = e.deltaY > 0 ? -0.1 : 0.1;
        setZoom((z) => Math.max(0.2, Math.min(3, z + direction)));
      }
    };

    window.addEventListener("wheel", handleWheel, { passive: true });
    return () => window.removeEventListener("wheel", handleWheel);
  }, []);

  const fetchGraphData = async () => {
    setIsLoading(true);
    try {
      const data = await invoke<any>("get_memory_graph", { limit: 200 });
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
    <div className="h-full flex flex-col bg-[#0b0b0b] text-[#eaeaea]">
      {/* Controls Bar */}
      <motion.div
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: ANIMATION_NORMAL / 1000 }}
        className="px-8 py-4 border-b border-[#2a2a2a] flex items-center gap-4"
      >
        <div className="flex-1 relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[#6c5ce7]" />
          <input
            type="text"
            placeholder="Search nodes..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full pl-10 pr-4 py-2.5 bg-[#161616] border border-[#2a2a2a] rounded-lg text-[#eaeaea] placeholder-[#aaaaaa] focus:outline-none focus:border-[#6c5ce7] transition-all duration-150"
          />
        </div>

        <motion.button
          onClick={handleResetCamera}
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.95 }}
          className="p-2.5 bg-[#161616] border border-[#2a2a2a] rounded-lg text-[#6c5ce7] hover:border-[#6c5ce7] transition-colors"
          title="Reset camera"
        >
          <RotateCcw className="w-4 h-4" />
        </motion.button>

        <motion.button
          onClick={() => setZoom((z) => Math.min(z + 0.2, 3))}
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.95 }}
          className="p-2.5 bg-[#161616] border border-[#2a2a2a] rounded-lg text-[#6c5ce7] hover:border-[#6c5ce7] transition-colors"
          title="Zoom in"
        >
          <ZoomIn className="w-4 h-4" />
        </motion.button>

        <motion.button
          onClick={() => setZoom((z) => Math.max(z - 0.2, 0.2))}
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.95 }}
          className="p-2.5 bg-[#161616] border border-[#2a2a2a] rounded-lg text-[#6c5ce7] hover:border-[#6c5ce7] transition-colors"
          title="Zoom out"
        >
          <ZoomOut className="w-4 h-4" />
        </motion.button>
      </motion.div>

      {/* Graph Container */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: ANIMATION_NORMAL / 1000, delay: 100 / 1000 }}
        className="flex-1 relative bg-[#0b0b0b] overflow-hidden"
      >
        {isLoading ? (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="absolute inset-0 flex items-center justify-center"
          >
            <div className="flex flex-col items-center gap-3">
              <motion.div
                animate={{ rotate: 360 }}
                transition={{ duration: 2, repeat: Infinity, ease: "linear" }}
                className="w-8 h-8 border-2 border-[#6c5ce7] border-t-transparent rounded-full"
              />
              <p className="text-[#aaaaaa] text-sm">Building knowledge graph...</p>
            </div>
          </motion.div>
        ) : filteredNodes.length === 0 ? (
          <div className="absolute inset-0 flex items-center justify-center text-[#aaaaaa]">
            <p className="text-sm">No memories in graph</p>
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
            backgroundColor="#0b0b0b"
            width={typeof window !== "undefined" ? window.innerWidth : 800}
            height={typeof window !== "undefined" ? window.innerHeight : 600}
            d3VelocityDecay={0.3}
            numDimensions={3}
            nodeVisibility={(n: any) => filteredNodes.some((fn) => fn.id === n.id)}
            warmupTicks={100}
            cooldownTicks={300}
          />
        )}

        {/* Zoom Indicator */}
        {!isLoading && filteredNodes.length > 0 && (
          <div className="absolute top-4 left-8 bg-[#161616]/80 border border-[#2a2a2a] rounded-lg px-3 py-2 text-xs text-[#aaaaaa]">
            Zoom: {(zoom * 100).toFixed(0)}%
          </div>
        )}

        {/* Legend */}
        {!isLoading && filteredNodes.length > 0 && (
          <motion.div
            initial={{ opacity: 0, x: -20 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: 200 / 1000 }}
            className="absolute bottom-8 left-8 bg-[#161616]/80 border border-[#2a2a2a] rounded-lg p-4 text-xs space-y-2"
          >
            <p className="text-[#aaaaaa] font-semibold mb-3">Tag Colors</p>
            <div className="space-y-1.5">
              {Object.entries(TAG_COLORS).map(([tag, color]) => (
                <div key={tag} className="flex items-center gap-2">
                  <div className="w-2 h-2 rounded-full" style={{ backgroundColor: color }} />
                  <span className="text-[#aaaaaa] capitalize">{tag}</span>
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
            className="absolute bottom-8 right-8 bg-[#161616]/80 border border-[#2a2a2a] rounded-lg p-4 text-xs space-y-2"
          >
            <div className="text-[#aaaaaa]">
              <span className="font-semibold text-[#6c5ce7]">{filteredNodes.length}</span> nodes
            </div>
            <div className="text-[#aaaaaa]">
              <span className="font-semibold text-[#6c5ce7]">{filteredEdges.length}</span> connections
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
            className="absolute inset-0 bg-black/50 z-40"
            onClick={() => setDetailPanelOpen(false)}
          >
            <motion.div
              initial={{ y: 20, opacity: 0 }}
              animate={{ y: 0, opacity: 1 }}
              exit={{ y: 20, opacity: 0 }}
              transition={{ duration: 300 / 1000 }}
              className="absolute bottom-8 left-8 w-96 bg-[#161616] border border-[#2a2a2a] rounded-lg p-6 z-50"
              onClick={(e) => e.stopPropagation()}
            >
              {/* Header */}
              <div className="flex items-start justify-between mb-4">
                <h3 className="text-lg font-semibold text-[#eaeaea] flex-1">Node Details</h3>
                <motion.button
                  onClick={() => setDetailPanelOpen(false)}
                  whileHover={{ scale: 1.1 }}
                  whileTap={{ scale: 0.95 }}
                  className="p-1 text-[#aaaaaa] hover:text-[#eaeaea]"
                >
                  <X className="w-4 h-4" />
                </motion.button>
              </div>

              {/* Content */}
              <div className="space-y-4">
                {/* Label */}
                <div>
                  <p className="text-[#aaaaaa] text-xs uppercase mb-1">Label</p>
                  <p className="text-[#eaeaea] text-sm line-clamp-3">{selectedNode.label}</p>
                </div>

                {/* ID */}
                <div>
                  <p className="text-[#aaaaaa] text-xs uppercase mb-1">ID</p>
                  <code className="text-[#6c5ce7] text-xs font-mono">
                    {selectedNode.id.slice(0, 16)}...
                  </code>
                </div>

                {/* Importance */}
                <div>
                  <p className="text-[#aaaaaa] text-xs uppercase mb-2">Importance</p>
                  <div className="flex items-center gap-2">
                    <div className="flex-1 bg-[#2a2a2a] rounded-full h-2 overflow-hidden">
                      <div
                        className="bg-[#6c5ce7] h-full rounded-full"
                        style={{ width: `${(selectedNode.importance || 0.5) * 100}%` }}
                      />
                    </div>
                    <span className="text-[#aaaaaa] text-xs">
                      {((selectedNode.importance || 0.5) * 100).toFixed(0)}%
                    </span>
                  </div>
                </div>

                {/* Tag */}
                {selectedNode.tag && (
                  <div>
                    <p className="text-[#aaaaaa] text-xs uppercase mb-2">Tag</p>
                    <span
                      className="inline-block px-3 py-1 rounded text-white text-xs"
                      style={{ backgroundColor: TAG_COLORS[selectedNode.tag] }}
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
    </div>
  );
}
