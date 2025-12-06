import { useState } from "react";
import { FileBrowser } from "../files/FileBrowser";
import { useCreateJob } from "../../hooks/useJobs";

export default function JobWizard() {
  const [source, setSource] = useState("");
  const [dest, setDest] = useState("");
  const createJob = useCreateJob();

  const handleSubmit = () => {
    createJob.mutate({ source, destination: dest });
  };

  return (
    <div className="max-w-6xl mx-auto p-6">
      <h2 className="text-2xl font-bold mb-6">Create New Transfer Job</h2>

      <div className="grid grid-cols-2 gap-6">
        <div className="space-y-4">
          <h3 className="font-semibold text-lg">1. Select Source</h3>
          <FileBrowser onSelect={setSource} />
          <div className="text-xs text-gray-500 p-2 bg-gray-50 rounded">
            <strong>Selected:</strong> {source || "(none)"}
          </div>
        </div>

        <div className="space-y-4">
          <h3 className="font-semibold text-lg">2. Select Destination</h3>
          <FileBrowser onSelect={setDest} />
          <div className="text-xs text-gray-500 p-2 bg-gray-50 rounded">
            <strong>Selected:</strong> {dest || "(none)"}
          </div>
        </div>
      </div>

      <div className="mt-6 pt-4 border-t flex justify-between items-center">
        <div className="text-sm text-gray-600">
          {createJob.isSuccess && (
            <span className="text-green-600 font-semibold">
              Job created successfully!
            </span>
          )}
          {createJob.isError && (
            <span className="text-red-600">
              Error:{" "}
              {createJob.error instanceof Error
                ? createJob.error.message
                : "Failed to create job"}
            </span>
          )}
        </div>

        <button
          onClick={handleSubmit}
          disabled={!source || !dest || createJob.isPending}
          className="px-6 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed"
        >
          {createJob.isPending ? "Launching..." : "Launch Orbit Job"}
        </button>
      </div>
    </div>
  );
}
