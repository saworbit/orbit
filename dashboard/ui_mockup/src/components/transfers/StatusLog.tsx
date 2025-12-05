import { useState } from 'react';
import { CheckCircle2, AlertCircle, Info, XCircle } from 'lucide-react';

export function StatusLog() {
  const [logs] = useState([
    { type: 'info', message: 'Orbit initialized successfully', time: '10:24:15' },
    { type: 'success', message: 'Connected to source location', time: '10:24:16' },
    { type: 'success', message: 'Connected to destination location', time: '10:24:17' },
    { type: 'info', message: 'Scanning source directory...', time: '10:24:18' },
    { type: 'warning', message: 'Some files may require administrator privileges', time: '10:24:20' },
  ]);

  const getIcon = (type: string) => {
    switch (type) {
      case 'success':
        return <CheckCircle2 className="w-4 h-4 text-green-600" />;
      case 'error':
        return <XCircle className="w-4 h-4 text-red-600" />;
      case 'warning':
        return <AlertCircle className="w-4 h-4 text-amber-600" />;
      default:
        return <Info className="w-4 h-4 text-blue-600" />;
    }
  };

  return (
    <div className="bg-white rounded-xl shadow-sm border border-slate-200 p-6">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-slate-700">Status Log</h3>
        <button className="text-sm text-slate-600 hover:text-slate-900">
          Clear Log
        </button>
      </div>
      
      <div className="bg-slate-900 rounded-lg p-4 font-mono text-sm max-h-64 overflow-y-auto">
        {logs.map((log, index) => (
          <div key={index} className="flex items-start gap-3 mb-2 last:mb-0">
            <span className="text-slate-500">{log.time}</span>
            <div className="mt-0.5">{getIcon(log.type)}</div>
            <span className="text-slate-300 flex-1">{log.message}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
