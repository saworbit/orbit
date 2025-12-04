import { useState, useMemo } from "react";
import {
  useJobs,
  useRunJob,
  useCancelJob,
  useDeleteJob,
} from "../../hooks/useJobs";
import {
  Play,
  X,
  Trash2,
  Search,
  Filter,
  RefreshCcw,
  Network,
} from "lucide-react";
import SystemHealth from "../dashboard/SystemHealth";

interface JobListProps {
  compact?: boolean;
  onSelectJob?: (jobId: number) => void;
}

export default function JobList({
  compact = false,
  onSelectJob,
}: JobListProps) {
  const { data: jobs, isLoading, refetch } = useJobs();
  const runJob = useRunJob();
  const cancelJob = useCancelJob();
  const deleteJob = useDeleteJob();

  const [filter, setFilter] = useState("all");
  const [search, setSearch] = useState("");

  const filteredJobs = useMemo(() => {
    if (!jobs) return [];
    return jobs.filter((job) => {
      const matchesStatus =
        filter === "all" || job.status.toLowerCase() === filter;
      const matchesSearch =
        job.source.toLowerCase().includes(search.toLowerCase()) ||
        job.destination.toLowerCase().includes(search.toLowerCase()) ||
        job.id.toString().includes(search);
      return matchesStatus && matchesSearch;
    });
  }, [jobs, filter, search]);

  if (isLoading) {
    return (
      <div className="space-y-6">
        {!compact && <SystemHealth />}
        <div className="w-full h-48 flex items-center justify-center text-muted-foreground animate-pulse">
          Loading job data...
        </div>
      </div>
    );
  }

  // Helper for status badges
  const getStatusBadge = (status: string) => {
    const styles: Record<string, string> = {
      pending:
        "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400 border-yellow-200 dark:border-yellow-800",
      running:
        "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400 border-blue-200 dark:border-blue-800",
      completed:
        "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400 border-green-200 dark:border-green-800",
      failed:
        "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400 border-red-200 dark:border-red-800",
      cancelled:
        "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-400 border-gray-200 dark:border-gray-700",
    };
    return (
      <span
        className={`px-2.5 py-0.5 rounded-full text-xs font-bold border capitalize ${styles[status.toLowerCase()] || styles.pending}`}
      >
        {status}
      </span>
    );
  };

  return (
    <div className="space-y-4">
      {!compact && <SystemHealth />}

      {!compact && (
        <>
          <div className="flex flex-col sm:flex-row gap-4 justify-between items-start sm:items-center">
            <div>
              <h2 className="text-2xl font-bold">Transfer Jobs</h2>
              <p className="text-sm text-muted-foreground">
                Monitor and manage your data transfer operations
              </p>
            </div>
          </div>

          <div className="flex flex-col sm:flex-row gap-4 justify-between items-center bg-card p-4 rounded-lg border shadow-sm">
            <div className="relative w-full sm:w-72">
              <Search
                className="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
                size={16}
              />
              <input
                type="text"
                placeholder="Search jobs..."
                className="w-full pl-9 pr-4 py-2 text-sm bg-background border rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
            </div>

            <div className="flex items-center gap-2 w-full sm:w-auto">
              <div className="flex items-center gap-2 px-3 py-2 bg-background border rounded-md text-sm">
                <Filter size={16} className="text-muted-foreground" />
                <select
                  value={filter}
                  onChange={(e) => setFilter(e.target.value)}
                  className="bg-transparent border-none outline-none text-foreground cursor-pointer"
                >
                  <option value="all">All Statuses</option>
                  <option value="running">Running</option>
                  <option value="pending">Pending</option>
                  <option value="completed">Completed</option>
                  <option value="failed">Failed</option>
                </select>
              </div>
              <button
                onClick={() => refetch()}
                className="p-2 hover:bg-accent rounded-md text-muted-foreground hover:text-foreground transition-colors"
                title="Refresh List"
              >
                <RefreshCcw size={18} />
              </button>
            </div>
          </div>
        </>
      )}

      <div className="bg-card border rounded-lg shadow-sm overflow-hidden">
        {filteredJobs.length === 0 ? (
          <div className="p-12 text-center">
            <div className="w-16 h-16 bg-muted rounded-full flex items-center justify-center mx-auto mb-4">
              <Network className="text-muted-foreground" size={32} />
            </div>
            <h3 className="text-lg font-medium">No jobs found</h3>
            <p className="text-muted-foreground">
              {search || filter !== "all"
                ? "Try adjusting your filters"
                : "Create a new job to get started"}
            </p>
          </div>
        ) : (
          <div className="divide-y divide-border">
            {filteredJobs.slice(0, compact ? 5 : undefined).map((job) => (
              <div
                key={job.id}
                className={`p-4 hover:bg-accent/50 transition-colors ${onSelectJob && !compact ? "cursor-pointer" : ""}`}
                onClick={() => onSelectJob && !compact && onSelectJob(job.id)}
              >
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-3">
                    <span className="font-mono text-xs text-muted-foreground">
                      #{job.id}
                    </span>
                    <h4 className="font-medium text-sm sm:text-base truncate max-w-[200px] sm:max-w-md">
                      {job.source.split("/").pop() || job.source}
                      <span className="mx-2 text-muted-foreground">→</span>
                      {job.destination.split("/").pop() || job.destination}
                    </h4>
                  </div>
                  {getStatusBadge(job.status)}
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs text-muted-foreground mb-3">
                  <div className="flex gap-1 truncate" title={job.source}>
                    <span className="font-semibold">Src:</span> {job.source}
                  </div>
                  <div className="flex gap-1 truncate" title={job.destination}>
                    <span className="font-semibold">Dst:</span>{" "}
                    {job.destination}
                  </div>
                </div>

                {job.status === "running" && (
                  <div className="space-y-1.5 mb-3">
                    <div className="flex justify-between text-xs">
                      <span>Progress</span>
                      <span className="font-medium text-foreground">
                        {Math.round(job.progress)}%
                      </span>
                    </div>
                    <div className="h-2 bg-secondary rounded-full overflow-hidden">
                      <div
                        className="h-full bg-blue-600 transition-all duration-500"
                        style={{ width: `${job.progress}%` }}
                      />
                    </div>
                    <div className="flex justify-between text-[10px] text-muted-foreground">
                      <span>
                        {job.completed_chunks} / {job.total_chunks} chunks
                      </span>
                      <span>
                        {(
                          (job.completed_chunks / (job.total_chunks || 1)) *
                          100
                        ).toFixed(1)}
                        % complete
                      </span>
                    </div>
                  </div>
                )}

                <div className="flex justify-end gap-2 mt-2">
                  {job.status === "pending" && (
                    <ActionBtn
                      onClick={() => runJob.mutate(job.id)}
                      icon={Play}
                      label="Run"
                      color="text-green-500 hover:bg-green-500/10"
                    />
                  )}
                  {job.status === "running" && (
                    <ActionBtn
                      onClick={() => cancelJob.mutate(job.id)}
                      icon={X}
                      label="Cancel"
                      color="text-orange-500 hover:bg-orange-500/10"
                    />
                  )}
                  <ActionBtn
                    onClick={() => deleteJob.mutate(job.id)}
                    icon={Trash2}
                    label="Delete"
                    color="text-red-500 hover:bg-red-500/10"
                  />
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

const ActionBtn = ({ onClick, icon: Icon, label, color }: any) => (
  <button
    onClick={(e) => {
      e.stopPropagation();
      onClick();
    }}
    className={`flex items-center gap-1 px-3 py-1.5 rounded text-xs font-medium transition-colors ${color}`}
  >
    <Icon size={14} />
    {label}
  </button>
);
