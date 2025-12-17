import {
  useCallback,
  useState,
  useRef,
  useEffect,
  type DragEvent,
} from "react";
import {
  ReactFlow,
  type Node,
  type Edge,
  Controls,
  Background,
  BackgroundVariant,
  useNodesState,
  useEdgesState,
  addEdge,
  type Connection,
  type NodeTypes,
  MiniMap,
  Panel,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { Save, CheckCircle, Loader2 } from "lucide-react";
import {
  usePipeline,
  useSavePipeline,
  type BackendNode,
  type BackendEdge,
} from "../../hooks/usePipelines";

// Custom node component for pipeline nodes
function PipelineNode({
  data,
}: {
  data: Record<string, unknown> & { label: string; type: string };
}) {
  const nodeColors: Record<string, string> = {
    source: "bg-blue-500/10 border-blue-500 text-blue-700 dark:text-blue-300",
    destination:
      "bg-green-500/10 border-green-500 text-green-700 dark:text-green-300",
    transform:
      "bg-purple-500/10 border-purple-500 text-purple-700 dark:text-purple-300",
    filter:
      "bg-yellow-500/10 border-yellow-500 text-yellow-700 dark:text-yellow-300",
    merge:
      "bg-orange-500/10 border-orange-500 text-orange-700 dark:text-orange-300",
    split: "bg-pink-500/10 border-pink-500 text-pink-700 dark:text-pink-300",
    conditional: "bg-red-500/10 border-red-500 text-red-700 dark:text-red-300",
  };

  const colorClass = nodeColors[data.type] || "bg-gray-500/10 border-gray-500";

  return (
    <div
      className={`px-4 py-3 shadow-lg rounded-lg border-2 min-w-[150px] ${colorClass}`}
    >
      <div className="flex flex-col gap-1">
        <div className="text-xs font-semibold uppercase tracking-wide opacity-60">
          {data.type}
        </div>
        <div className="font-medium">{data.label}</div>
      </div>
    </div>
  );
}

const nodeTypes: NodeTypes = {
  pipelineNode: PipelineNode,
};

// Node palette data
const nodeTemplates = [
  {
    type: "source",
    label: "Source",
    description: "Data source (file, S3, SMB)",
  },
  {
    type: "destination",
    label: "Destination",
    description: "Transfer target",
  },
  { type: "transform", label: "Transform", description: "Compress/encrypt" },
  { type: "filter", label: "Filter", description: "Pattern matching" },
  { type: "merge", label: "Merge", description: "Combine streams" },
  { type: "split", label: "Split", description: "Distribute data" },
  {
    type: "conditional",
    label: "Conditional",
    description: "Route by condition",
  },
];

export function PipelineEditor({ pipelineId }: { pipelineId: string }) {
  const { data: pipeline, isLoading } = usePipeline(pipelineId);
  const savePipeline = useSavePipeline();

  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [isSaving, setIsSaving] = useState(false);
  const reactFlowWrapper = useRef<HTMLDivElement>(null);

  // Load pipeline data into React Flow state
  useEffect(() => {
    if (pipeline) {
      // Map backend nodes to React Flow nodes
      const flowNodes: Node[] = pipeline.nodes.map((node) => ({
        id: node.id,
        type: "pipelineNode",
        position: node.position || { x: 0, y: 0 },
        data: {
          label: node.name || node.node_type,
          type: node.node_type,
        },
      }));

      // Map backend edges to React Flow edges
      const flowEdges: Edge[] = pipeline.edges.map((edge) => ({
        id: edge.id,
        source: edge.source_node_id,
        target: edge.target_node_id,
        type: "smoothstep",
        animated: true,
      }));

      setNodes(flowNodes);
      setEdges(flowEdges);
    }
  }, [pipeline, setNodes, setEdges]);

  const onConnect = useCallback(
    (connection: Connection) => setEdges((eds) => addEdge(connection, eds)),
    [setEdges]
  );

  const onDragOver = useCallback((event: DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
  }, []);

  const onDrop = useCallback(
    (event: DragEvent) => {
      event.preventDefault();

      const type = event.dataTransfer.getData("application/reactflow");
      if (!type || !reactFlowWrapper.current) return;

      const reactFlowBounds = reactFlowWrapper.current.getBoundingClientRect();
      const position = {
        x: event.clientX - reactFlowBounds.left,
        y: event.clientY - reactFlowBounds.top,
      };

      const newNode: Node = {
        id: `${type}-${Date.now()}`,
        type: "pipelineNode",
        position,
        data: { label: `New ${type}`, type },
      };

      setNodes((nds) => nds.concat(newNode));
    },
    [setNodes]
  );

  const onDragStart = (event: DragEvent, nodeType: string) => {
    event.dataTransfer.setData("application/reactflow", nodeType);
    event.dataTransfer.effectAllowed = "move";
  };

  const handleSave = async () => {
    setIsSaving(true);
    try {
      // Map React Flow nodes back to backend format
      const backendNodes: BackendNode[] = nodes.map((node) => ({
        id: node.id,
        node_type: (node.data as { type: string }).type,
        name: (node.data as { label: string }).label,
        position: node.position,
        config: {}, // Add config if needed
      }));

      // Map React Flow edges back to backend format
      const backendEdges: BackendEdge[] = edges.map((edge) => ({
        id: edge.id,
        source_node_id: edge.source,
        target_node_id: edge.target,
        label: typeof edge.label === "string" ? edge.label : null,
      }));

      await savePipeline.mutateAsync({
        id: pipelineId,
        nodes: backendNodes as unknown as Node[],
        edges: backendEdges as unknown as Edge[],
      });
    } finally {
      setIsSaving(false);
    }
  };

  const handleValidate = () => {
    const errors: string[] = [];
    const sourceNodes = nodes.filter(
      (n) => (n.data as { type: string }).type === "source"
    );
    const destNodes = nodes.filter(
      (n) => (n.data as { type: string }).type === "destination"
    );

    if (sourceNodes.length === 0) errors.push("Pipeline must have a Source");
    if (destNodes.length === 0) errors.push("Pipeline must have a Destination");

    if (errors.length > 0) {
      alert("Validation Errors:\n" + errors.join("\n"));
    } else {
      alert("✅ Pipeline is valid!");
    }
  };

  if (isLoading) {
    return (
      <div className="w-full h-[600px] flex items-center justify-center">
        <div className="flex flex-col items-center gap-3">
          <Loader2 className="w-8 h-8 animate-spin text-primary" />
          <p className="text-muted-foreground">Loading pipeline...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex gap-4 h-[calc(100vh-200px)]">
      {/* Node Palette */}
      <div className="w-64 bg-card border rounded-xl p-4 space-y-3 overflow-y-auto">
        <h3 className="font-semibold text-sm uppercase tracking-wide text-muted-foreground">
          Node Palette
        </h3>
        {nodeTemplates.map((template) => (
          <div
            key={template.type}
            draggable
            onDragStart={(e) => onDragStart(e, template.type)}
            className="bg-background border rounded-lg p-3 cursor-move hover:border-primary hover:shadow-md transition-all"
          >
            <div className="font-medium text-sm">{template.label}</div>
            <div className="text-xs text-muted-foreground mt-1">
              {template.description}
            </div>
          </div>
        ))}

        <div className="pt-4 mt-4 border-t space-y-2">
          <div className="text-xs text-muted-foreground">
            <strong>Quick Guide:</strong>
            <ul className="list-disc list-inside mt-2 space-y-1">
              <li>Drag nodes onto canvas</li>
              <li>Click handles to connect</li>
              <li>Double-click to configure</li>
              <li>Delete key to remove</li>
            </ul>
          </div>
        </div>
      </div>

      {/* React Flow Canvas */}
      <div className="flex-1 bg-card border rounded-xl overflow-hidden">
        <div ref={reactFlowWrapper} className="w-full h-full">
          <ReactFlow
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            onDrop={onDrop}
            onDragOver={onDragOver}
            nodeTypes={nodeTypes}
            fitView
            attributionPosition="bottom-left"
          >
            <Background variant={BackgroundVariant.Dots} gap={12} size={1} />
            <Controls />
            <MiniMap />

            {/* Action Panel */}
            <Panel position="top-right" className="flex gap-2">
              <button
                onClick={handleValidate}
                className="flex items-center gap-2 px-3 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 shadow-lg text-sm font-medium"
              >
                <CheckCircle size={16} />
                Validate
              </button>
              <button
                onClick={handleSave}
                disabled={isSaving}
                className="flex items-center gap-2 px-3 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 shadow-lg text-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isSaving ? (
                  <Loader2 size={16} className="animate-spin" />
                ) : (
                  <Save size={16} />
                )}
                {isSaving ? "Saving..." : "Save"}
              </button>
            </Panel>

            {/* Status Panel */}
            <Panel position="top-left">
              <div className="bg-background/95 backdrop-blur border rounded-lg px-3 py-2 shadow-lg">
                <div className="flex items-center gap-3 text-sm">
                  <div className="flex items-center gap-1">
                    <div className="w-2 h-2 bg-blue-500 rounded-full"></div>
                    <span className="text-muted-foreground">
                      {nodes.length} nodes
                    </span>
                  </div>
                  <div className="flex items-center gap-1">
                    <div className="w-2 h-2 bg-green-500 rounded-full"></div>
                    <span className="text-muted-foreground">
                      {edges.length} connections
                    </span>
                  </div>
                </div>
              </div>
            </Panel>
          </ReactFlow>
        </div>
      </div>
    </div>
  );
}
