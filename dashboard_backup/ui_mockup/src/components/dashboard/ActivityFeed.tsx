import { CheckCircle2, AlertCircle, Clock, XCircle, ChevronDown } from 'lucide-react';
import { useState } from 'react';

export function ActivityFeed() {
  const [filter, setFilter] = useState('all');
  
  const activities = [
    {
      id: 1,
      type: 'success',
      title: 'Backup to AWS S3 completed',
      timestamp: '2 minutes ago',
      details: '2.4 GB transferred in 1m 23s',
      expanded: false,
    },
    {
      id: 2,
      type: 'progress',
      title: 'Media sync in progress',
      timestamp: '5 minutes ago',
      details: '45% complete • 12.1 GB of 26.8 GB',
      expanded: false,
    },
    {
      id: 3,
      type: 'warning',
      title: 'Network hiccup detected, resuming...',
      timestamp: '12 minutes ago',
      details: 'Retry attempt 2 of 5 • Exponential backoff applied',
      expanded: false,
    },
    {
      id: 4,
      type: 'error',
      title: 'Permission denied for /system/protected',
      timestamp: '28 minutes ago',
      details: 'Skipped 3 files • Partial transfer completed',
      expanded: false,
    },
    {
      id: 5,
      type: 'success',
      title: 'Database backup completed',
      timestamp: '1 hour ago',
      details: '890 MB transferred with compression (Zstd)',
      expanded: false,
    },
  ];

  const getIcon = (type: string) => {
    switch (type) {
      case 'success':
        return <CheckCircle2 className="w-5 h-5 text-green-600" />;
      case 'error':
        return <XCircle className="w-5 h-5 text-red-600" />;
      case 'warning':
        return <AlertCircle className="w-5 h-5 text-amber-600" />;
      case 'progress':
        return <Clock className="w-5 h-5 text-blue-600" />;
      default:
        return <CheckCircle2 className="w-5 h-5 text-slate-400" />;
    }
  };

  return (
    <div className="bg-white rounded-lg border border-slate-200 p-6">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-slate-900">Recent Activity</h2>
        
        <div className="flex items-center gap-2">
          {['all', 'completed', 'failed'].map((f) => (
            <button
              key={f}
              onClick={() => setFilter(f)}
              className={`px-3 py-1 text-sm rounded capitalize ${
                filter === f
                  ? 'bg-blue-100 text-blue-700'
                  : 'hover:bg-slate-100 text-slate-600'
              }`}
            >
              {f}
            </button>
          ))}
        </div>
      </div>

      <div className="space-y-3">
        {activities.map((activity) => (
          <div
            key={activity.id}
            className="border border-slate-200 rounded-lg p-4 hover:border-slate-300 transition-colors"
          >
            <div className="flex items-start gap-3">
              <div className="mt-0.5">{getIcon(activity.type)}</div>
              
              <div className="flex-1 min-w-0">
                <div className="flex items-start justify-between gap-2">
                  <p className="text-slate-900">{activity.title}</p>
                  <button className="text-slate-400 hover:text-slate-600">
                    <ChevronDown className="w-4 h-4" />
                  </button>
                </div>
                
                <p className="text-sm text-slate-600 mt-1">{activity.details}</p>
                <p className="text-xs text-slate-500 mt-2">{activity.timestamp}</p>

                {/* AI Suggestion */}
                {activity.id === 5 && (
                  <div className="mt-3 p-3 bg-blue-50 border border-blue-200 rounded-lg">
                    <p className="text-sm text-blue-900">💡 AI Suggestion: Optimize compression for images? (Zstd recommended)</p>
                    <div className="flex gap-2 mt-2">
                      <button className="px-3 py-1 text-xs bg-blue-600 text-white rounded hover:bg-blue-700">
                        Apply
                      </button>
                      <button className="px-3 py-1 text-xs border border-blue-300 text-blue-700 rounded hover:bg-blue-100">
                        Learn More
                      </button>
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>
        ))}
      </div>

      <div className="mt-4 pt-4 border-t border-slate-200 text-center">
        <button className="text-sm text-blue-600 hover:text-blue-700">
          Load More Activity
        </button>
      </div>
    </div>
  );
}
