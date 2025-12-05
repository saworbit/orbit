import { Filter, Download, Upload, MoreVertical } from 'lucide-react';

export function AdvancedTransfer() {
  const jobs = [
    {
      id: 1,
      name: 'Daily Backup to AWS S3',
      status: 'running',
      progress: 67,
      source: 'Local Drive',
      dest: 'AWS S3 Bucket',
      size: '20.4 GB',
      transferred: '13.7 GB',
      speed: '45.2 MB/s',
      eta: '2m 34s',
      protocol: 'S3',
    },
    {
      id: 2,
      name: 'Media Sync',
      status: 'paused',
      progress: 45,
      source: 'Cloud Storage',
      dest: 'Local Drive',
      size: '26.8 GB',
      transferred: '12.1 GB',
      speed: '0 MB/s',
      eta: '--',
      protocol: 'HTTPS',
    },
    {
      id: 3,
      name: 'Database Backup',
      status: 'completed',
      progress: 100,
      source: 'Database Server',
      dest: 'Backup Server',
      size: '890 MB',
      transferred: '890 MB',
      speed: '--',
      eta: 'Complete',
      protocol: 'SMB',
    },
    {
      id: 4,
      name: 'Project Files Transfer',
      status: 'error',
      progress: 23,
      source: 'Local Drive',
      dest: 'Remote Server',
      size: '5.2 GB',
      transferred: '1.2 GB',
      speed: '0 MB/s',
      eta: 'Failed',
      protocol: 'FTP',
    },
  ];

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'running':
        return 'bg-green-100 text-green-700 border-green-200';
      case 'paused':
        return 'bg-amber-100 text-amber-700 border-amber-200';
      case 'completed':
        return 'bg-blue-100 text-blue-700 border-blue-200';
      case 'error':
        return 'bg-red-100 text-red-700 border-red-200';
      default:
        return 'bg-slate-100 text-slate-700 border-slate-200';
    }
  };

  return (
    <div className="space-y-6">
      {/* Filters & Actions */}
      <div className="bg-white rounded-lg border border-slate-200 p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <button className="px-4 py-2 bg-slate-100 hover:bg-slate-200 rounded-lg flex items-center gap-2">
              <Filter className="w-4 h-4" />
              Filter
            </button>
            
            <select className="px-4 py-2 border border-slate-300 rounded-lg bg-white">
              <option>All Protocols</option>
              <option>S3</option>
              <option>SMB</option>
              <option>FTP</option>
              <option>HTTPS</option>
            </select>

            <select className="px-4 py-2 border border-slate-300 rounded-lg bg-white">
              <option>All Status</option>
              <option>Running</option>
              <option>Paused</option>
              <option>Completed</option>
              <option>Failed</option>
            </select>
          </div>

          <div className="flex items-center gap-2">
            <button className="px-4 py-2 bg-slate-100 hover:bg-slate-200 rounded-lg text-sm">
              Templates
            </button>
            <button className="px-4 py-2 bg-slate-100 hover:bg-slate-200 rounded-lg text-sm">
              Bulk Actions
            </button>
          </div>
        </div>
      </div>

      {/* Jobs List */}
      <div className="bg-white rounded-lg border border-slate-200 overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead className="bg-slate-50 border-b border-slate-200">
              <tr>
                <th className="px-4 py-3 text-left text-sm text-slate-600">
                  <input type="checkbox" className="rounded" />
                </th>
                <th className="px-4 py-3 text-left text-sm text-slate-600">Name</th>
                <th className="px-4 py-3 text-left text-sm text-slate-600">Status</th>
                <th className="px-4 py-3 text-left text-sm text-slate-600">Progress</th>
                <th className="px-4 py-3 text-left text-sm text-slate-600">Route</th>
                <th className="px-4 py-3 text-left text-sm text-slate-600">Size</th>
                <th className="px-4 py-3 text-left text-sm text-slate-600">Speed</th>
                <th className="px-4 py-3 text-left text-sm text-slate-600">ETA</th>
                <th className="px-4 py-3 text-left text-sm text-slate-600">Protocol</th>
                <th className="px-4 py-3 text-left text-sm text-slate-600"></th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-200">
              {jobs.map((job) => (
                <tr key={job.id} className="hover:bg-slate-50 cursor-pointer">
                  <td className="px-4 py-4">
                    <input type="checkbox" className="rounded" />
                  </td>
                  <td className="px-4 py-4">
                    <div className="text-slate-900">{job.name}</div>
                  </td>
                  <td className="px-4 py-4">
                    <span className={`px-2 py-1 rounded text-xs border capitalize ${getStatusColor(job.status)}`}>
                      {job.status}
                    </span>
                  </td>
                  <td className="px-4 py-4">
                    <div className="flex items-center gap-3">
                      <div className="flex-1 h-2 bg-slate-100 rounded-full overflow-hidden min-w-[100px]">
                        <div 
                          className={`h-full transition-all ${
                            job.status === 'running' ? 'bg-gradient-to-r from-blue-500 to-purple-600' :
                            job.status === 'completed' ? 'bg-green-500' :
                            job.status === 'error' ? 'bg-red-500' :
                            'bg-amber-500'
                          }`}
                          style={{ width: `${job.progress}%` }}
                        />
                      </div>
                      <span className="text-sm text-slate-600 min-w-[3ch]">{job.progress}%</span>
                    </div>
                  </td>
                  <td className="px-4 py-4">
                    <div className="flex items-center gap-2 text-sm">
                      <Upload className="w-3 h-3 text-slate-400" />
                      <span className="text-slate-600">{job.source}</span>
                      <span className="text-slate-400">→</span>
                      <Download className="w-3 h-3 text-slate-400" />
                      <span className="text-slate-600">{job.dest}</span>
                    </div>
                  </td>
                  <td className="px-4 py-4">
                    <div className="text-sm text-slate-600">{job.transferred} / {job.size}</div>
                  </td>
                  <td className="px-4 py-4">
                    <div className="text-sm text-slate-600">{job.speed}</div>
                  </td>
                  <td className="px-4 py-4">
                    <div className="text-sm text-slate-600">{job.eta}</div>
                  </td>
                  <td className="px-4 py-4">
                    <span className="px-2 py-1 bg-slate-100 text-slate-700 rounded text-xs">
                      {job.protocol}
                    </span>
                  </td>
                  <td className="px-4 py-4">
                    <button className="p-1 hover:bg-slate-200 rounded">
                      <MoreVertical className="w-4 h-4 text-slate-600" />
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Job Detail Panel (Slide-out) */}
      <div className="bg-white rounded-lg border border-slate-200 p-6">
        <h3 className="text-slate-700 mb-4">Job Details</h3>
        <p className="text-sm text-slate-500">Select a job to view detailed telemetry, logs, and controls</p>
      </div>
    </div>
  );
}
