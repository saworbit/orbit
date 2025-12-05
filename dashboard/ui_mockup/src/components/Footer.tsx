import { HardDrive, Wifi, Pause, Play } from 'lucide-react';

export function Footer() {
  return (
    <footer className="h-10 bg-slate-900 border-t border-slate-700 flex items-center justify-between px-6 text-sm">
      {/* System Health */}
      <div className="flex items-center gap-6">
        <div className="flex items-center gap-2 text-slate-300">
          <HardDrive className="w-4 h-4" />
          <span>Disk: 842 GB free</span>
        </div>
        
        <div className="flex items-center gap-2 text-green-400">
          <Wifi className="w-4 h-4" />
          <span>Connected: 3 active</span>
        </div>
        
        <div className="text-slate-400">
          Uptime: 99.9%
        </div>
      </div>

      {/* Quick Actions */}
      <div className="flex items-center gap-2">
        <button className="px-3 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white flex items-center gap-2">
          <Pause className="w-3 h-3" />
          <span>Pause All</span>
        </button>
        <button className="px-3 py-1 rounded bg-blue-600 hover:bg-blue-700 text-white flex items-center gap-2">
          <Play className="w-3 h-3" />
          <span>Resume All</span>
        </button>
      </div>
    </footer>
  );
}
