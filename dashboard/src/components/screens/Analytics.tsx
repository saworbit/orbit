import { BarChart3, TrendingUp, Download } from "lucide-react";
import { useJobs } from "../../hooks/useJobs";

export function Analytics() {
  const { data: jobs } = useJobs();

  // Calculate basic stats
  const totalJobs = jobs?.length || 0;
  const completedJobs =
    jobs?.filter((j) => j.status === "completed").length || 0;
  const failedJobs = jobs?.filter((j) => j.status === "failed").length || 0;
  const successRate =
    totalJobs > 0 ? ((completedJobs / totalJobs) * 100).toFixed(1) : "0";

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-slate-900">Analytics & Insights</h1>
        <div className="flex items-center gap-2">
          <button className="px-4 py-2 bg-white border border-slate-300 rounded-lg hover:bg-slate-50 flex items-center gap-2">
            <Download className="w-4 h-4" />
            Export Report
          </button>
        </div>
      </div>

      {/* Summary Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="bg-white rounded-lg border border-slate-200 p-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-slate-600">Total Jobs</span>
            <BarChart3 className="w-4 h-4 text-blue-600" />
          </div>
          <div className="text-2xl font-bold text-slate-900">{totalJobs}</div>
          <div className="text-xs text-slate-500 mt-1">All time</div>
        </div>

        <div className="bg-white rounded-lg border border-slate-200 p-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-slate-600">Completed</span>
            <TrendingUp className="w-4 h-4 text-green-600" />
          </div>
          <div className="text-2xl font-bold text-slate-900">
            {completedJobs}
          </div>
          <div className="text-xs text-green-600 mt-1">
            +{successRate}% success rate
          </div>
        </div>

        <div className="bg-white rounded-lg border border-slate-200 p-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-slate-600">Failed</span>
            <BarChart3 className="w-4 h-4 text-red-600" />
          </div>
          <div className="text-2xl font-bold text-slate-900">{failedJobs}</div>
          <div className="text-xs text-slate-500 mt-1">Requires attention</div>
        </div>

        <div className="bg-white rounded-lg border border-slate-200 p-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-slate-600">Success Rate</span>
            <TrendingUp className="w-4 h-4 text-blue-600" />
          </div>
          <div className="text-2xl font-bold text-slate-900">
            {successRate}%
          </div>
          <div className="text-xs text-slate-500 mt-1">Overall performance</div>
        </div>
      </div>

      {/* Charts Placeholder */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="bg-white rounded-lg border border-slate-200 p-6">
          <h3 className="text-lg font-medium text-slate-900 mb-4">
            Job Completion Over Time
          </h3>
          <div className="h-64 flex items-center justify-center border-2 border-dashed border-slate-200 rounded-lg">
            <div className="text-center">
              <BarChart3 className="w-12 h-12 text-slate-400 mx-auto mb-2" />
              <p className="text-sm text-slate-500">
                Chart coming soon (Recharts)
              </p>
            </div>
          </div>
        </div>

        <div className="bg-white rounded-lg border border-slate-200 p-6">
          <h3 className="text-lg font-medium text-slate-900 mb-4">
            Throughput Trends
          </h3>
          <div className="h-64 flex items-center justify-center border-2 border-dashed border-slate-200 rounded-lg">
            <div className="text-center">
              <TrendingUp className="w-12 h-12 text-slate-400 mx-auto mb-2" />
              <p className="text-sm text-slate-500">
                Chart coming soon (Recharts)
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
