import { useState } from 'react';
import { Plus } from 'lucide-react';
import { SimpleTransfer } from '../transfers/SimpleTransfer';
import { AdvancedTransfer } from '../transfers/AdvancedTransfer';

export function Transfers() {
  const [mode, setMode] = useState<'simple' | 'advanced'>('simple');

  return (
    <div className="p-6">
      <div className="flex items-center justify-between mb-6">
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

        <button className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 flex items-center gap-2">
          <Plus className="w-5 h-5" />
          New Transfer
        </button>
      </div>

      {mode === 'simple' ? <SimpleTransfer /> : <AdvancedTransfer />}
    </div>
  );
}
