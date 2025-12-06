import { Server, Database, Cloud, HardDrive } from 'lucide-react';

export function NetworkMap() {
  const connections = [
    { id: 1, source: 'Local Drive', dest: 'AWS S3', protocol: 'S3', speed: '45.2 MB/s', status: 'active' },
    { id: 2, source: 'Database', dest: 'Backup Server', protocol: 'SMB', speed: '28.1 MB/s', status: 'active' },
    { id: 3, source: 'Cloud Storage', dest: 'Local Drive', protocol: 'HTTPS', speed: '12.5 MB/s', status: 'paused' },
  ];

  return (
    <div className="bg-white rounded-lg border border-slate-200 p-6">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-slate-900">Network Topology</h2>
        <div className="flex items-center gap-2">
          <button className="px-3 py-1 text-sm bg-slate-100 hover:bg-slate-200 rounded">
            Map View
          </button>
          <button className="px-3 py-1 text-sm hover:bg-slate-100 rounded">
            List View
          </button>
        </div>
      </div>

      {/* Visual Node Graph */}
      <div className="relative h-80 bg-slate-50 rounded-lg border border-slate-200 overflow-hidden">
        {/* Grid Pattern */}
        <div className="absolute inset-0" style={{
          backgroundImage: `linear-gradient(rgba(148, 163, 184, 0.1) 1px, transparent 1px), linear-gradient(90deg, rgba(148, 163, 184, 0.1) 1px, transparent 1px)`,
          backgroundSize: '20px 20px'
        }} />

        {/* Nodes */}
        <div className="absolute top-20 left-20">
          <div className="bg-white border-2 border-blue-500 rounded-lg p-4 shadow-lg">
            <HardDrive className="w-8 h-8 text-blue-600 mb-2" />
            <p className="text-sm text-slate-700">Local Drive</p>
            <p className="text-xs text-slate-500">Source</p>
          </div>
        </div>

        <div className="absolute top-20 right-20">
          <div className="bg-white border-2 border-purple-500 rounded-lg p-4 shadow-lg">
            <Cloud className="w-8 h-8 text-purple-600 mb-2" />
            <p className="text-sm text-slate-700">AWS S3</p>
            <p className="text-xs text-slate-500">Destination</p>
          </div>
        </div>

        <div className="absolute bottom-20 left-1/2 -translate-x-1/2">
          <div className="bg-white border-2 border-green-500 rounded-lg p-4 shadow-lg">
            <Server className="w-8 h-8 text-green-600 mb-2" />
            <p className="text-sm text-slate-700">Backup Server</p>
            <p className="text-xs text-slate-500">Active</p>
          </div>
        </div>

        {/* Connection Lines */}
        <svg className="absolute inset-0 pointer-events-none">
          <defs>
            <marker id="arrowhead" markerWidth="10" markerHeight="10" refX="9" refY="3" orient="auto">
              <polygon points="0 0, 10 3, 0 6" fill="#3b82f6" />
            </marker>
          </defs>
          <line x1="140" y1="80" x2="calc(100% - 140)" y2="80" stroke="#3b82f6" strokeWidth="2" markerEnd="url(#arrowhead)" strokeDasharray="5,5">
            <animate attributeName="stroke-dashoffset" from="0" to="10" dur="0.5s" repeatCount="indefinite" />
          </line>
        </svg>

        {/* Transfer Stats Overlay */}
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-white/95 backdrop-blur border border-slate-300 rounded-lg p-3 shadow-lg">
          <p className="text-xs text-slate-600">Active Transfer</p>
          <p className="text-sm text-slate-900">45.2 MB/s</p>
          <p className="text-xs text-slate-500">Protocol: S3</p>
        </div>
      </div>

      {/* Connection List */}
      <div className="mt-4 space-y-2">
        {connections.map((conn) => (
          <div key={conn.id} className="flex items-center justify-between p-3 bg-slate-50 rounded-lg">
            <div className="flex items-center gap-3">
              <div className={`w-2 h-2 rounded-full ${conn.status === 'active' ? 'bg-green-500' : 'bg-amber-500'}`} />
              <span className="text-sm text-slate-700">{conn.source}</span>
              <span className="text-slate-400">→</span>
              <span className="text-sm text-slate-700">{conn.dest}</span>
            </div>
            <div className="flex items-center gap-4">
              <span className="text-xs text-slate-500 bg-white px-2 py-1 rounded">{conn.protocol}</span>
              <span className="text-sm text-slate-600">{conn.speed}</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
