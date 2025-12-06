import { useState } from "react";
import { StatusLog } from "./StatusLog";
import { ArrowRight, Play } from "lucide-react";
import { useCreateJob } from "../../hooks/useJobs";

export function SimpleTransfer() {
  const [source, setSource] = useState("");
  const [destination, setDestination] = useState("");
  const [compress, setCompress] = useState(false);
  const [verify, setVerify] = useState(true);
  const [parallelWorkers, setParallelWorkers] = useState(4);

  const createJob = useCreateJob();

  const handleStartTransfer = () => {
    if (!source || !destination) {
      alert("Please select both source and destination paths");
      return;
    }

    createJob.mutate({
      source,
      destination,
      compress,
      verify,
      parallel_workers: parallelWorkers,
    });
  };

  return (
    <div className="space-y-6">
      {/* Transfer Panel */}
      <div className="bg-white rounded-xl shadow-sm border border-slate-200 p-6">
        <div className="grid grid-cols-1 lg:grid-cols-[1fr,auto,1fr] gap-6 items-center">
          {/* Source Selector */}
          <div className="space-y-4">
            <h3 className="text-slate-700">Source</h3>
            <div className="space-y-2">
              <label className="text-sm text-slate-600">Path</label>
              <input
                type="text"
                placeholder="/path/to/source"
                className="w-full px-4 py-2 border border-slate-300 rounded-lg"
                value={source}
                onChange={(e) => setSource(e.target.value)}
              />
            </div>
          </div>

          <div className="flex justify-center">
            <div className="w-12 h-12 bg-blue-50 rounded-full flex items-center justify-center">
              <ArrowRight className="w-6 h-6 text-blue-600" />
            </div>
          </div>

          {/* Destination Selector */}
          <div className="space-y-4">
            <h3 className="text-slate-700">Destination</h3>
            <div className="space-y-2">
              <label className="text-sm text-slate-600">Path</label>
              <input
                type="text"
                placeholder="/path/to/destination"
                className="w-full px-4 py-2 border border-slate-300 rounded-lg"
                value={destination}
                onChange={(e) => setDestination(e.target.value)}
              />
            </div>
          </div>
        </div>

        {/* Options */}
        <div className="mt-6 pt-6 border-t border-slate-200">
          <div className="flex items-center gap-6">
            <label className="flex items-center gap-2 text-sm text-slate-700">
              <input
                type="checkbox"
                checked={compress}
                onChange={(e) => setCompress(e.target.checked)}
                className="rounded border-slate-300"
              />
              Compress
            </label>
            <label className="flex items-center gap-2 text-sm text-slate-700">
              <input
                type="checkbox"
                checked={verify}
                onChange={(e) => setVerify(e.target.checked)}
                className="rounded border-slate-300"
              />
              Verify
            </label>
            <div className="flex items-center gap-2 text-sm text-slate-700">
              <label>Workers:</label>
              <input
                type="number"
                min="1"
                max="16"
                value={parallelWorkers}
                onChange={(e) => setParallelWorkers(parseInt(e.target.value))}
                className="w-16 px-2 py-1 border border-slate-300 rounded"
              />
            </div>
          </div>
        </div>
      </div>

      {/* Control Bar */}
      <div className="bg-white rounded-xl shadow-sm border border-slate-200 p-6">
        <div className="flex items-center justify-between">
          <button
            onClick={handleStartTransfer}
            disabled={createJob.isPending}
            className="px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <Play className="w-5 h-5" />
            {createJob.isPending ? "Creating..." : "Create Transfer Job"}
          </button>

          {createJob.isSuccess && (
            <div className="text-sm text-green-600">
              ✓ Job created successfully! Check the job list below.
            </div>
          )}

          {createJob.isError && (
            <div className="text-sm text-red-600">
              ✗ Failed to create job. Check the console for details.
            </div>
          )}
        </div>
      </div>

      <StatusLog />
    </div>
  );
}
