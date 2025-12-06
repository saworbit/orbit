import {
  CheckCircle2,
  AlertCircle,
  Clock,
  XCircle,
  ChevronDown,
} from "lucide-react";
import { useState } from "react";
import { useJobs } from "../../hooks/useJobs";

export function ActivityFeed() {
  const [filter, setFilter] = useState("all");
  const { data: jobs, isLoading } = useJobs();

  // Convert jobs to activity feed items
  const activities =
    jobs
      ?.map((job) => {
        const getType = (status: string) => {
          if (status === "completed") return "success";
          if (status === "failed" || status === "cancelled") return "error";
          if (status === "running") return "progress";
          return "warning";
        };

        const getTitle = (job: (typeof jobs)[0]) => {
          if (job.status === "completed")
            return `Transfer to ${job.destination.split("/").pop()} completed`;
          if (job.status === "running")
            return `Transferring to ${job.destination.split("/").pop()}`;
          if (job.status === "failed")
            return `Transfer failed: ${job.destination.split("/").pop()}`;
          if (job.status === "cancelled")
            return `Transfer cancelled: ${job.destination.split("/").pop()}`;
          return `Job pending: ${job.destination.split("/").pop()}`;
        };

        const getDetails = (job: (typeof jobs)[0]) => {
          if (job.status === "running") {
            return `${job.progress}% complete • ${job.completed_chunks} of ${job.total_chunks} chunks`;
          }
          if (job.status === "completed") {
            return `${job.completed_chunks} chunks transferred successfully`;
          }
          if (job.failed_chunks > 0) {
            return `${job.failed_chunks} chunks failed • ${job.completed_chunks} completed`;
          }
          return `${job.total_chunks} chunks total`;
        };

        const getTimestamp = (created_at: number) => {
          const now = Date.now();
          const diff = Math.floor((now - created_at) / 1000); // seconds

          if (diff < 60) return `${diff}s ago`;
          if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
          if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
          return `${Math.floor(diff / 86400)}d ago`;
        };

        return {
          id: job.id,
          type: getType(job.status),
          title: getTitle(job),
          timestamp: getTimestamp(job.created_at),
          details: getDetails(job),
          expanded: false,
        };
      })
      .sort((a, b) => b.id - a.id) // Most recent first
      .slice(0, 10) || []; // Limit to 10 items

  // If no jobs, show placeholder
  if (!isLoading && activities.length === 0) {
    activities.push({
      id: 0,
      type: "warning",
      title: "No activity yet",
      timestamp: "Just now",
      details: "Create a job to see activity here",
      expanded: false,
    });
  }

  const getIcon = (type: string) => {
    switch (type) {
      case "success":
        return <CheckCircle2 className="w-5 h-5 text-green-600" />;
      case "error":
        return <XCircle className="w-5 h-5 text-red-600" />;
      case "warning":
        return <AlertCircle className="w-5 h-5 text-amber-600" />;
      case "progress":
        return <Clock className="w-5 h-5 text-blue-600" />;
      default:
        return <CheckCircle2 className="w-5 h-5 text-slate-400" />;
    }
  };

  return (
    <div className="bg-white rounded-lg border border-slate-200 p-6">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-slate-900">Recent Activity</h2>

        <div className="flex items-center gap-2">
          {["all", "completed", "failed"].map((f) => (
            <button
              key={f}
              onClick={() => setFilter(f)}
              className={`px-3 py-1 text-sm rounded capitalize ${
                filter === f
                  ? "bg-blue-100 text-blue-700"
                  : "hover:bg-slate-100 text-slate-600"
              }`}
            >
              {f}
            </button>
          ))}
        </div>
      </div>

      <div className="space-y-3">
        {activities
          .filter((activity) => {
            if (filter === "all") return true;
            if (filter === "completed") return activity.type === "success";
            if (filter === "failed") return activity.type === "error";
            return true;
          })
          .map((activity) => (
            <div
              key={activity.id}
              className="border border-slate-200 rounded-lg p-4 hover:border-slate-300 transition-colors"
            >
              <div className="flex items-start gap-3">
                <div className="mt-0.5">{getIcon(activity.type)}</div>

                <div className="flex-1 min-w-0">
                  <div className="flex items-start justify-between gap-2">
                    <p className="text-slate-900">{activity.title}</p>
                    <button className="text-slate-400 hover:text-slate-600">
                      <ChevronDown className="w-4 h-4" />
                    </button>
                  </div>

                  <p className="text-sm text-slate-600 mt-1">
                    {activity.details}
                  </p>
                  <p className="text-xs text-slate-500 mt-2">
                    {activity.timestamp}
                  </p>
                </div>
              </div>
            </div>
          ))}
      </div>

      <div className="mt-4 pt-4 border-t border-slate-200 text-center">
        <button className="text-sm text-blue-600 hover:text-blue-700">
          Load More Activity
        </button>
      </div>
    </div>
  );
}
