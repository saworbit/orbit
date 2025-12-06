import { Workflow, Plus, Play } from 'lucide-react';

export function Pipelines() {
  return (
    <div className="p-6">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-slate-900">Pipeline Workflows</h1>
        <div className="flex items-center gap-2">
          <button className="px-4 py-2 bg-white border border-slate-300 rounded-lg hover:bg-slate-50 flex items-center gap-2">
            <Play className="w-4 h-4" />
            Run Selected
          </button>
          <button className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 flex items-center gap-2">
            <Plus className="w-4 h-4" />
            New Pipeline
          </button>
        </div>
      </div>

      {/* Pipeline Editor Placeholder */}
      <div className="bg-white rounded-lg border border-slate-200 overflow-hidden">
        <div className="p-12 text-center">
          <div className="w-16 h-16 bg-slate-100 rounded-full flex items-center justify-center mx-auto mb-4">
            <Workflow className="w-8 h-8 text-slate-400" />
          </div>
          <h3 className="text-lg font-medium text-slate-900 mb-2">Pipeline Editor Coming Soon</h3>
          <p className="text-slate-600 mb-4">
            Visual workflow editor for chaining multiple transfer jobs with React Flow.
          </p>
          <div className="inline-flex items-center gap-2 text-sm text-slate-500">
            <Workflow className="w-4 h-4" />
            <span>Powered by @xyflow/react</span>
          </div>
        </div>
      </div>
    </div>
  );
}
