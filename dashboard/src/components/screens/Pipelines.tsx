import { useState } from "react";
import {
  Workflow,
  Plus,
  ArrowLeft,
  Trash2,
  Calendar,
  GitBranch,
} from "lucide-react";
import {
  usePipelines,
  useCreatePipeline,
  useDeletePipeline,
} from "../../hooks/usePipelines";
import { PipelineEditor } from "../pipelines/PipelineEditor";

export function Pipelines() {
  const { data: pipelines, isLoading } = usePipelines();
  const createPipeline = useCreatePipeline();
  const deletePipeline = useDeletePipeline();

  const [selectedPipelineId, setSelectedPipelineId] = useState<string | null>(
    null
  );
  const [isCreating, setIsCreating] = useState(false);
  const [newPipelineName, setNewPipelineName] = useState("");
  const [newPipelineDesc, setNewPipelineDesc] = useState("");

  const handleCreatePipeline = async () => {
    if (!newPipelineName.trim()) {
      alert("Pipeline name is required");
      return;
    }

    try {
      const pipelineId = await createPipeline.mutateAsync({
        name: newPipelineName,
        description: newPipelineDesc,
      });
      setIsCreating(false);
      setNewPipelineName("");
      setNewPipelineDesc("");
      setSelectedPipelineId(pipelineId);
    } catch (error) {
      alert("Failed to create pipeline");
    }
  };

  const handleDeletePipeline = async (pipelineId: string, name: string) => {
    if (confirm(`Delete pipeline "${name}"? This cannot be undone.`)) {
      await deletePipeline.mutateAsync(pipelineId);
      if (selectedPipelineId === pipelineId) {
        setSelectedPipelineId(null);
      }
    }
  };

  const getStatusBadge = (status: string) => {
    const styles: Record<string, string> = {
      draft:
        "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-400 border-gray-200 dark:border-gray-700",
      ready:
        "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400 border-blue-200 dark:border-blue-800",
      running:
        "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400 border-green-200 dark:border-green-800",
      completed:
        "bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-400 border-purple-200 dark:border-purple-800",
      failed:
        "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400 border-red-200 dark:border-red-800",
    };
    return (
      <span
        className={`px-2.5 py-0.5 rounded-full text-xs font-bold border capitalize ${styles[status.toLowerCase()] || styles.draft}`}
      >
        {status}
      </span>
    );
  };

  // If a pipeline is selected, show the editor
  if (selectedPipelineId) {
    return (
      <div className="p-6">
        <div className="flex items-center gap-4 mb-6">
          <button
            onClick={() => setSelectedPipelineId(null)}
            className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
          >
            <ArrowLeft size={16} />
            Back to Pipelines
          </button>
          <div className="h-6 w-px bg-border"></div>
          <h1 className="text-2xl font-bold">Pipeline Editor</h1>
        </div>
        <PipelineEditor pipelineId={selectedPipelineId} />
      </div>
    );
  }

  // Otherwise, show the pipeline list
  return (
    <div className="p-6">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-3xl font-bold tracking-tight flex items-center gap-3">
            <Workflow className="text-primary" size={32} />
            Pipeline Workflows
          </h1>
          <p className="text-muted-foreground mt-1">
            Create and manage data transfer workflows
          </p>
        </div>
        <button
          onClick={() => setIsCreating(!isCreating)}
          className="px-4 py-2.5 bg-primary text-primary-foreground rounded-lg font-medium hover:bg-primary/90 shadow-lg shadow-primary/20 transition-all flex items-center gap-2"
        >
          <Plus size={18} />
          New Pipeline
        </button>
      </div>

      {/* Create Pipeline Form */}
      {isCreating && (
        <div className="bg-card border rounded-xl p-6 mb-6 shadow-sm">
          <h3 className="font-semibold flex items-center gap-2 mb-4">
            <GitBranch size={18} className="text-primary" />
            Create New Pipeline
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="space-y-2">
              <label className="text-sm font-medium text-muted-foreground">
                Pipeline Name *
              </label>
              <input
                placeholder="e.g., Backup to S3"
                className="w-full px-3 py-2 bg-background border rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
                value={newPipelineName}
                onChange={(e) => setNewPipelineName(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium text-muted-foreground">
                Description (optional)
              </label>
              <input
                placeholder="Brief description of the workflow"
                className="w-full px-3 py-2 bg-background border rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
                value={newPipelineDesc}
                onChange={(e) => setNewPipelineDesc(e.target.value)}
              />
            </div>
          </div>
          <div className="flex justify-end gap-2 mt-4">
            <button
              onClick={() => {
                setIsCreating(false);
                setNewPipelineName("");
                setNewPipelineDesc("");
              }}
              className="px-4 py-2 border rounded-lg hover:bg-accent transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={handleCreatePipeline}
              disabled={!newPipelineName.trim() || createPipeline.isPending}
              className="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {createPipeline.isPending ? "Creating..." : "Create Pipeline"}
            </button>
          </div>
        </div>
      )}

      {/* Pipeline List */}
      {isLoading ? (
        <div className="bg-card border rounded-xl p-12 text-center">
          <div className="animate-pulse">
            <div className="w-16 h-16 bg-muted rounded-full mx-auto mb-4"></div>
            <div className="h-4 bg-muted rounded w-32 mx-auto"></div>
          </div>
        </div>
      ) : !pipelines || pipelines.length === 0 ? (
        <div className="bg-card border rounded-xl p-12 text-center">
          <div className="w-16 h-16 bg-muted rounded-full flex items-center justify-center mx-auto mb-4">
            <Workflow className="text-muted-foreground" size={32} />
          </div>
          <h3 className="text-lg font-medium mb-2">No Pipelines Yet</h3>
          <p className="text-muted-foreground mb-4">
            Create your first workflow to get started
          </p>
          <button
            onClick={() => setIsCreating(true)}
            className="inline-flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
          >
            <Plus size={16} />
            Create Pipeline
          </button>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {pipelines.map((pipeline) => (
            <div
              key={pipeline.id}
              className="bg-card border rounded-xl p-6 hover:shadow-lg transition-all cursor-pointer group"
              onClick={() => setSelectedPipelineId(pipeline.id)}
            >
              <div className="flex items-start justify-between mb-3">
                <div className="flex-1">
                  <h3 className="font-semibold text-lg group-hover:text-primary transition-colors">
                    {pipeline.name}
                  </h3>
                  {pipeline.description && (
                    <p className="text-sm text-muted-foreground mt-1">
                      {pipeline.description}
                    </p>
                  )}
                </div>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDeletePipeline(pipeline.id, pipeline.name);
                  }}
                  className="text-red-500 hover:bg-red-500/10 p-2 rounded transition-colors opacity-0 group-hover:opacity-100"
                >
                  <Trash2 size={16} />
                </button>
              </div>

              <div className="flex items-center justify-between mt-4 pt-4 border-t">
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <Calendar size={12} />
                  {new Date(pipeline.created_at * 1000).toLocaleDateString()}
                </div>
                {getStatusBadge(pipeline.status)}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Help Text */}
      <div className="mt-6 bg-blue-50 dark:bg-blue-950/20 border border-blue-200 dark:border-blue-900 rounded-lg p-4">
        <div className="flex items-start gap-3">
          <GitBranch
            className="text-blue-600 dark:text-blue-400 mt-0.5"
            size={20}
          />
          <div className="flex-1">
            <h4 className="font-semibold text-blue-900 dark:text-blue-100 mb-1">
              About Pipelines
            </h4>
            <p className="text-sm text-blue-700 dark:text-blue-300">
              Pipelines let you chain multiple transfer operations into
              workflows. Drag nodes onto the canvas, connect them, and configure
              each step. Perfect for complex multi-stage data migrations.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
