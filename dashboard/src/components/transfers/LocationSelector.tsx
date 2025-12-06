import { useState } from 'react';
import { FolderOpen, HardDrive, Cloud, Database, ChevronRight } from 'lucide-react';

interface LocationSelectorProps {
  title: string;
  type: 'source' | 'destination';
}

export function LocationSelector({ title }: LocationSelectorProps) {
  const [selectedLocation, setSelectedLocation] = useState<string>('');
  const [selectedPath, setSelectedPath] = useState<string>('');

  const locations = [
    { id: 'local', name: 'Local Drive', icon: HardDrive },
    { id: 'cloud', name: 'Cloud Storage', icon: Cloud },
    { id: 'database', name: 'Database', icon: Database },
  ];

  const sampleFiles = [
    { name: 'Documents', type: 'folder', size: '2.4 GB' },
    { name: 'Images', type: 'folder', size: '5.1 GB' },
    { name: 'Videos', type: 'folder', size: '12.8 GB' },
    { name: 'Projects', type: 'folder', size: '890 MB' },
  ];

  return (
    <div className="space-y-4">
      <h3 className="text-slate-700">{title}</h3>
      
      <div className="space-y-2">
        <label className="text-sm text-slate-600">Location Type</label>
        <select 
          className="w-full px-4 py-2 border border-slate-300 rounded-lg bg-white text-slate-900"
          value={selectedLocation}
          onChange={(e) => setSelectedLocation(e.target.value)}
        >
          <option value="">Select location type...</option>
          {locations.map(loc => (
            <option key={loc.id} value={loc.id}>{loc.name}</option>
          ))}
        </select>
      </div>

      <div className="space-y-2">
        <label className="text-sm text-slate-600">Path</label>
        <div className="flex gap-2">
          <input 
            type="text"
            placeholder="/Users/documents"
            className="flex-1 px-4 py-2 border border-slate-300 rounded-lg"
            value={selectedPath}
            onChange={(e) => setSelectedPath(e.target.value)}
          />
          <button className="px-4 py-2 bg-slate-100 hover:bg-slate-200 rounded-lg border border-slate-300 flex items-center gap-2">
            <FolderOpen className="w-4 h-4" />
            Browse
          </button>
        </div>
      </div>

      {selectedLocation && (
        <div className="mt-4 border border-slate-200 rounded-lg overflow-hidden">
          <div className="bg-slate-50 px-4 py-2 text-sm text-slate-600 border-b border-slate-200">
            Files & Folders
          </div>
          <div className="max-h-48 overflow-y-auto">
            {sampleFiles.map((file, index) => (
              <div 
                key={index}
                className="px-4 py-3 hover:bg-slate-50 flex items-center justify-between border-b border-slate-100 last:border-b-0 cursor-pointer"
              >
                <div className="flex items-center gap-3">
                  <FolderOpen className="w-4 h-4 text-slate-400" />
                  <span className="text-slate-700">{file.name}</span>
                </div>
                <div className="flex items-center gap-3">
                  <span className="text-sm text-slate-500">{file.size}</span>
                  <ChevronRight className="w-4 h-4 text-slate-400" />
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
