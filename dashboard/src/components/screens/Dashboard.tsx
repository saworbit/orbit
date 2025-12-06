import { KPICards } from "../dashboard/KPICards";
import { NetworkMap } from "../dashboard/NetworkMap";
import { ActivityFeed } from "../dashboard/ActivityFeed";

export function Dashboard() {
  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-slate-900">Command Center</h1>
        <div className="flex items-center gap-2">
          <button className="px-4 py-2 bg-white border border-slate-300 rounded-lg hover:bg-slate-50">
            Export Manifest
          </button>
          <button className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700">
            New Transfer
          </button>
        </div>
      </div>

      <KPICards />
      <NetworkMap />
      <ActivityFeed />
    </div>
  );
}
