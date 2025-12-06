import { FolderOpen, File, HardDrive } from 'lucide-react';

export function Files() {
  return (
    <div className="p-6">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-slate-900">File Browser</h1>
        <div className="flex items-center gap-2">
          <button className="px-4 py-2 bg-white border border-slate-300 rounded-lg hover:bg-slate-50 flex items-center gap-2">
            <HardDrive className="w-4 h-4" />
            Local
          </button>
          <button className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700">
            Browse Remote
          </button>
        </div>
      </div>

      {/* File Browser Placeholder */}
      <div className="bg-white rounded-lg border border-slate-200 overflow-hidden">
        {/* Breadcrumb */}
        <div className="border-b border-slate-200 px-4 py-3 bg-slate-50">
          <div className="flex items-center gap-2 text-sm text-slate-600">
            <HardDrive className="w-4 h-4" />
            <span>/</span>
            <span className="text-slate-900">home</span>
            <span>/</span>
            <span className="text-slate-900">user</span>
          </div>
        </div>

        {/* File List Placeholder */}
        <div className="p-12 text-center">
          <div className="w-16 h-16 bg-slate-100 rounded-full flex items-center justify-center mx-auto mb-4">
            <FolderOpen className="w-8 h-8 text-slate-400" />
          </div>
          <h3 className="text-lg font-medium text-slate-900 mb-2">File Browser Coming Soon</h3>
          <p className="text-slate-600 mb-4">
            This feature will allow you to browse and select files from local and remote filesystems.
          </p>
          <div className="inline-flex items-center gap-2 text-sm text-slate-500">
            <File className="w-4 h-4" />
            <span>API endpoint: /api/list_dir</span>
          </div>
        </div>
      </div>
    </div>
  );
}
