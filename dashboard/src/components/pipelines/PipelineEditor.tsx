import { useCallback, useMemo } from "react";
import {
  ReactFlow,
  addEdge,
  Background,
  Controls,
  useNodesState,
  useEdgesState,
  ReactFlowProvider,
} from "@xyflow/react";
import type { Connection, Node } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { Database, Zap, Cloud } from "lucide-react";

const initialNodes: Node[] = [
  {
    id: "1",
    type: "input",
    data: { label: "Source: Local Storage" },
    position: { x: 250, y: 25 },
  },
  {
    id: "2",
    type: "output",
    data: { label: "Destination: S3 Bucket" },
    position: { x: 250, y: 200 },
  },
];

function PipelineEditorInner() {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);

  // Memoize node and edge types to prevent React Flow warnings
  const nodeTypes = useMemo(() => ({}), []);
  const edgeTypes = useMemo(() => ({}), []);

  const onConnect = useCallback(
    (params: Connection) => setEdges((eds) => addEdge(params, eds)),
    [setEdges]
  );

  const addNode = (type: string) => {
    const id = Math.random().toString();
    const newNode: Node = {
      id,
      position: { x: Math.random() * 400 + 100, y: Math.random() * 300 + 50 },
      data: { label: `${type} Node` },
      type:
        type === "Source"
          ? "input"
          : type === "Destination"
            ? "output"
            : "default",
    };
    setNodes((nds) => nds.concat(newNode));
  };

  return (
    <div className="h-[600px] border border-border rounded-xl bg-card shadow-sm flex flex-col overflow-hidden">
      {/* Toolbar */}
      <div className="p-4 border-b border-border bg-muted/30 flex flex-wrap gap-2">
        <button
          onClick={() => addNode("Source")}
          className="flex items-center gap-2 px-4 py-2 bg-background border rounded-lg hover:bg-accent transition-colors text-sm font-medium"
        >
          <Database size={16} className="text-blue-500" />
          Add Source
        </button>
        <button
          onClick={() => addNode("Transform")}
          className="flex items-center gap-2 px-4 py-2 bg-background border rounded-lg hover:bg-accent transition-colors text-sm font-medium"
        >
          <Zap size={16} className="text-yellow-500" />
          Add Transform
        </button>
        <button
          onClick={() => addNode("Destination")}
          className="flex items-center gap-2 px-4 py-2 bg-background border rounded-lg hover:bg-accent transition-colors text-sm font-medium"
        >
          <Cloud size={16} className="text-green-500" />
          Add Destination
        </button>
        <div className="flex-1" />
        <div className="text-xs text-muted-foreground self-center">
          {nodes.length} nodes, {edges.length} connections
        </div>
      </div>

      {/* Canvas */}
      <div className="flex-1 bg-background/50">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          nodeTypes={nodeTypes}
          edgeTypes={edgeTypes}
          fitView
          className="bg-background/50"
        >
          <Background className="bg-muted/20" gap={16} />
          <Controls className="bg-card border-border" />
        </ReactFlow>
      </div>
    </div>
  );
}

export default function PipelineEditor() {
  return (
    <ReactFlowProvider>
      <PipelineEditorInner />
    </ReactFlowProvider>
  );
}
