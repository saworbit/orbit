import { useState } from 'react';
import { ArrowLeft } from 'lucide-react';
import { SimpleTransfer } from '../transfers/SimpleTransfer';
import { AdvancedTransfer } from '../transfers/AdvancedTransfer';
import JobList from '../jobs/JobList';
import { JobDetail } from '../jobs/JobDetail';

export function Transfers() {
  const [mode, setMode] = useState<'simple' | 'advanced'>('simple');
  const [selectedJobId, setSelectedJobId] = useState<number | null>(null);

  // If job is selected, show JobDetail with chunk map
  if (selectedJobId) {
    return (
      <div className="p-6">
        <button
          onClick={() => setSelectedJobId(null)}
          className="mb-4 flex items-center gap-2 text-slate-600 hover:text-slate-900"
        >
          <ArrowLeft className="w-5 h-5" />
          Back to Transfers
        </button>
        <JobDetail jobId={selectedJobId} onBack={() => setSelectedJobId(null)} />
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <h1 className="text-slate-900">Transfer Jobs</h1>

          {/* Mode Toggle */}
          <div className="flex items-center gap-1 bg-slate-100 rounded-lg p-1">
            <button
              onClick={() => setMode('simple')}
              className={`px-4 py-2 rounded-md text-sm transition-colors ${
                mode === 'simple'
                  ? 'bg-white text-slate-900 shadow-sm'
                  : 'text-slate-600 hover:text-slate-900'
              }`}
            >
              Simple
            </button>
            <button
              onClick={() => setMode('advanced')}
              className={`px-4 py-2 rounded-md text-sm transition-colors ${
                mode === 'advanced'
                  ? 'bg-white text-slate-900 shadow-sm'
                  : 'text-slate-600 hover:text-slate-900'
              }`}
            >
              Advanced
            </button>
          </div>
        </div>
      </div>

      {/* Transfer Panel */}
      {mode === 'simple' ? <SimpleTransfer /> : <AdvancedTransfer />}

      {/* Job List with chunk map integration */}
      <div className="mt-6">
        <JobList compact={false} onSelectJob={setSelectedJobId} />
      </div>
    </div>
  );
}
