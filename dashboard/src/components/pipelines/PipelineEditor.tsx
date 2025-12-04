import { useCallback, useMemo } from "react";
import ReactFlow, {
  addEdge,
  Background,
  Controls,
  useNodesState,
  useEdgesState,
  ReactFlowProvider,
} from "@xyflow/react";
import type { Connection, Node } from "@xyflow/react";
import "@xyflow/react/dist/style.css";

const initialNodes: Node[] = [
  {
    id: "1",
    type: "input",
    data: { label: "Source: Local" },
    position: { x: 250, y: 5 },
  },
  {
    id: "2",
    type: "output",
    data: { label: "Dest: S3 Bucket" },
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
      position: { x: Math.random() * 400, y: Math.random() * 400 },
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
    <div className="h-[600px] border rounded-lg bg-slate-50 flex flex-col">
      <div className="p-4 border-b bg-white flex gap-2">
        <button
          onClick={() => addNode("Source")}
          className="px-4 py-2 border rounded hover:bg-gray-100"
        >
          + Source
        </button>
        <button
          onClick={() => addNode("Transform")}
          className="px-4 py-2 border rounded hover:bg-gray-100"
        >
          + Transform
        </button>
        <button
          onClick={() => addNode("Destination")}
          className="px-4 py-2 border rounded hover:bg-gray-100"
        >
          + Destination
        </button>
      </div>
      <div className="flex-1">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          nodeTypes={nodeTypes}
          edgeTypes={edgeTypes}
          fitView
        >
          <Background color="#aaa" gap={16} />
          <Controls />
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
