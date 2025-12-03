import { useJobs, useRunJob, useCancelJob, useDeleteJob } from '../../hooks/useJobs';
import { Play, X, Trash2 } from 'lucide-react';

export default function JobList() {
  const { data: jobs, isLoading } = useJobs();
  const runJob = useRunJob();
  const cancelJob = useCancelJob();
  const deleteJob = useDeleteJob();

  if (isLoading) {
    return <div className="p-6 text-center">Loading jobs...</div>;
  }

  if (!jobs || jobs.length === 0) {
    return (
      <div className="p-6 text-center text-gray-500">
        No jobs yet. Create one to get started!
      </div>
    );
  }

  const getStatusColor = (status: string) => {
    switch (status.toLowerCase()) {
      case 'pending':
        return 'bg-yellow-100 text-yellow-800';
      case 'running':
        return 'bg-blue-100 text-blue-800';
      case 'completed':
        return 'bg-green-100 text-green-800';
      case 'failed':
        return 'bg-red-100 text-red-800';
      case 'cancelled':
        return 'bg-gray-100 text-gray-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  return (
    <div className="max-w-6xl mx-auto p-6">
      <h2 className="text-2xl font-bold mb-6">Transfer Jobs</h2>

      <div className="space-y-3">
        {jobs.map((job) => (
          <div
            key={job.id}
            className="border rounded-lg p-4 hover:shadow-md transition-shadow"
          >
            <div className="flex items-start justify-between">
              <div className="flex-1">
                <div className="flex items-center gap-3 mb-2">
                  <span className="font-mono text-sm text-gray-500">#{job.id}</span>
                  <span className={`px-2 py-1 rounded text-xs font-semibold ${getStatusColor(job.status)}`}>
                    {job.status.toUpperCase()}
                  </span>
                </div>

                <div className="space-y-1 text-sm">
                  <div className="flex gap-2">
                    <span className="text-gray-500 font-medium">Source:</span>
                    <span className="font-mono truncate">{job.source}</span>
                  </div>
                  <div className="flex gap-2">
                    <span className="text-gray-500 font-medium">Destination:</span>
                    <span className="font-mono truncate">{job.destination}</span>
                  </div>
                </div>

                {job.status === 'running' && (
                  <div className="mt-3">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="text-xs text-gray-600">Progress: {Math.round(job.progress)}%</span>
                      <span className="text-xs text-gray-400">
                        ({job.completed_chunks}/{job.total_chunks} chunks)
                      </span>
                    </div>
                    <div className="w-full bg-gray-200 rounded-full h-2">
                      <div
                        className="bg-blue-600 h-2 rounded-full transition-all"
                        style={{ width: `${job.progress}%` }}
                      ></div>
                    </div>
                  </div>
                )}
              </div>

              <div className="flex gap-2 ml-4">
                {job.status === 'pending' && (
                  <button
                    onClick={() => runJob.mutate(job.id)}
                    className="p-2 text-green-600 hover:bg-green-50 rounded"
                    title="Run job"
                  >
                    <Play size={18} />
                  </button>
                )}
                {job.status === 'running' && (
                  <button
                    onClick={() => cancelJob.mutate(job.id)}
                    className="p-2 text-orange-600 hover:bg-orange-50 rounded"
                    title="Cancel job"
                  >
                    <X size={18} />
                  </button>
                )}
                <button
                  onClick={() => deleteJob.mutate(job.id)}
                  className="p-2 text-red-600 hover:bg-red-50 rounded"
                  title="Delete job"
                >
                  <Trash2 size={18} />
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
