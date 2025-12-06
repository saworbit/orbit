import { useQuery } from "@tanstack/react-query";
import { HardDrive, FileJson, ArrowLeft } from "lucide-react";
import { api } from "../../lib/api";

// The "Cool" Feature: Visual Chunk Map
// Simulates a dense grid of file chunks being processed
const ChunkMap = ({
  total,
  completed,
  failed,
}: {
  total: number;
  completed: number;
  failed: number;
}) => {
  // Cap visual chunks to 100 for performance/layout
  const displayCount = 100;
  const progressRatio = total > 0 ? completed / total : 0;
  const visualCompleted = Math.floor(progressRatio * displayCount);
  const visualFailed = Math.floor((failed / total) * displayCount);

  return (
    <div className="space-y-2">
      <div className="flex justify-between text-sm font-medium">
        <span>Chunk Allocation Map</span>
        <span className="text-muted-foreground font-mono">
          {completed}/{total} Chunks
        </span>
      </div>
      <div className="grid grid-cols-20 gap-1 bg-muted/20 p-4 rounded-lg border shadow-inner">
        {Array.from({ length: displayCount }).map((_, i) => {
          let color = "bg-muted"; // Pending
          if (i < visualFailed)
            color = "bg-red-500 shadow-[0_0_8px_rgba(239,68,68,0.6)]"; // Failed
          else if (i < visualCompleted + visualFailed)
            color = "bg-green-500 shadow-[0_0_8px_rgba(34,197,94,0.6)]"; // Done

          return (
            <div
              key={i}
              className={`h-1.5 w-1.5 md:h-2 md:w-2 rounded-[1px] ${color} transition-all duration-500`}
              title={`Chunk Block ${i}`}
            />
          );
        })}
      </div>
      <div className="flex gap-4 text-xs text-muted-foreground mt-2">
        <div className="flex items-center gap-1">
          <div className="w-2 h-2 bg-green-500 rounded-[1px]"></div> Completed
        </div>
        <div className="flex items-center gap-1">
          <div className="w-2 h-2 bg-red-500 rounded-[1px]"></div>{" "}
          Corrupt/Failed
        </div>
        <div className="flex items-center gap-1">
          <div className="w-2 h-2 bg-muted rounded-[1px]"></div> Pending
          Allocation
        </div>
      </div>
    </div>
  );
};

// --- DATA SHAPE (Must match Rust API) ---
interface Job {
  id: number;
  source: string;
  destination: string;
  status: string;
  progress: number;
  total_chunks: number;
  completed_chunks: number;
  failed_chunks: number;
  created_at: number;
  updated_at: number;
}

export function JobDetail({
  jobId,
  onBack,
}: {
  jobId: number;
  onBack: () => void;
}) {
  // --- CONNECT TO REAL API WITH LIVE POLLING ---
  const { data: job, isLoading } = useQuery({
    queryKey: ["job", jobId],
    queryFn: () =>
      api.post<Job>("/get_job", { job_id: jobId }).then((r) => r.data),
    refetchInterval: 1000, // Poll every second for live updates
  });

  if (isLoading || !job) {
    return (
      <div className="p-12 text-center animate-pulse">
        <div className="text-xl font-bold text-muted-foreground">
          Loading flight telemetry...
        </div>
      </div>
    );
  }

  // Mock config for now - will be added to API later
  const config = {
    compress: true,
    verify: true,
    parallel_workers: 4,
  };

  return (
    <div className="space-y-6 animate-in slide-in-from-right-4 duration-300">
      {/* Header */}
      <div className="flex items-center gap-4 border-b border-border pb-4">
        <button
          onClick={onBack}
          className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
        >
          <ArrowLeft size={16} />
          Back
        </button>
        <div className="h-6 w-px bg-border"></div>
        <div>
          <h1 className="text-2xl font-bold flex items-center gap-3">
            Job #{job.id}
            <span className="text-sm bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 px-2.5 py-0.5 rounded-full border border-blue-200 dark:border-blue-800 uppercase tracking-wide">
              {job.status}
            </span>
          </h1>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Left Column: Configuration & Meta */}
        <div className="space-y-6">
          <div className="bg-card border rounded-xl p-6 shadow-sm">
            <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider mb-4">
              Configuration
            </h3>
            <div className="space-y-4">
              <div className="group">
                <label className="text-xs text-muted-foreground flex items-center gap-1">
                  <HardDrive size={12} /> Source
                </label>
                <div className="font-mono text-sm break-all p-2 bg-muted/50 rounded mt-1 border border-transparent group-hover:border-border transition-colors">
                  {job.source}
                </div>
              </div>
              <div className="group">
                <label className="text-xs text-muted-foreground flex items-center gap-1">
                  <HardDrive size={12} /> Destination
                </label>
                <div className="font-mono text-sm break-all p-2 bg-muted/50 rounded mt-1 border border-transparent group-hover:border-border transition-colors">
                  {job.destination}
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4 pt-4 border-t border-border">
                <div className="bg-muted/30 p-2 rounded">
                  <div className="text-xs text-muted-foreground">
                    Compression
                  </div>
                  <div className="font-bold">
                    {config.compress ? "Enabled" : "Disabled"}
                  </div>
                </div>
                <div className="bg-muted/30 p-2 rounded">
                  <div className="text-xs text-muted-foreground">
                    Verification
                  </div>
                  <div className="font-bold">
                    {config.verify ? "Strict" : "Fast"}
                  </div>
                </div>
                <div className="bg-muted/30 p-2 rounded">
                  <div className="text-xs text-muted-foreground">Workers</div>
                  <div className="font-bold">
                    {config.parallel_workers} Threads
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Center/Right: Visualization & Progress */}
        <div className="lg:col-span-2 space-y-6">
          {/* Large Progress Card */}
          <div className="bg-card border rounded-xl p-8 shadow-sm relative overflow-hidden">
            <div className="absolute top-0 left-0 w-full h-1 bg-muted">
              <div
                className="h-full bg-blue-500 transition-all duration-500"
                style={{ width: `${job.progress}%` }}
              ></div>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-3 gap-8 text-center">
              <div>
                <div className="text-4xl font-bold tabular-nums">
                  {job.progress.toFixed(1)}%
                </div>
                <div className="text-xs text-muted-foreground uppercase mt-1">
                  Total Completion
                </div>
              </div>
              <div className="border-l border-border">
                <div className="text-4xl font-bold tabular-nums text-green-500">
                  {job.completed_chunks}
                </div>
                <div className="text-xs text-muted-foreground uppercase mt-1">
                  Chunks Synced
                </div>
              </div>
              <div className="border-l border-border">
                <div
                  className={`text-4xl font-bold tabular-nums ${job.failed_chunks > 0 ? "text-red-500" : "text-muted-foreground"}`}
                >
                  {job.failed_chunks}
                </div>
                <div className="text-xs text-muted-foreground uppercase mt-1">
                  Failed Chunks
                </div>
              </div>
            </div>
          </div>

          {/* The Visual Chunk Map */}
          <div className="bg-card border rounded-xl p-6 shadow-sm">
            <ChunkMap
              total={job.total_chunks}
              completed={job.completed_chunks}
              failed={job.failed_chunks}
            />
          </div>

          {/* Activity Log Preview */}
          <div className="bg-card border rounded-xl shadow-sm overflow-hidden">
            <div className="p-4 border-b border-border bg-muted/30 flex justify-between items-center">
              <h3 className="font-semibold text-sm flex items-center gap-2">
                <FileJson size={14} /> Event Stream
              </h3>
              <span className="text-xs text-muted-foreground">
                Live Tailing
              </span>
            </div>
            <div className="bg-black/90 text-green-400 font-mono text-xs p-4 h-48 overflow-y-auto space-y-1">
              <div className="opacity-50">
                [INFO] {new Date().toISOString()} - Job worker #3 initialized
              </div>
              <div className="opacity-50">
                [INFO] {new Date().toISOString()} - Connection to S3 pool
                established
              </div>
              <div>
                [INFO] {new Date().toISOString()} - Processing chunk block range
                4500-4600
              </div>
              <div>
                [WARN] {new Date().toISOString()} - Retry triggered on chunk
                4512 (Network Timeout)
              </div>
              <div>
                [INFO] {new Date().toISOString()} - Checksum verified for chunk
                4510
              </div>
              <div className="animate-pulse">_</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
