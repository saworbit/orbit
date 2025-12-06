import { Activity, Database, Clock, TrendingUp } from 'lucide-react';

export function KPICards() {
  const kpis = [
    {
      label: 'Active Jobs',
      value: '5',
      change: '+2 from yesterday',
      trend: 'up',
      icon: Activity,
      color: 'blue',
    },
    {
      label: 'Total Data Transferred',
      value: '2.5 TB',
      change: '+180 GB today',
      trend: 'up',
      icon: Database,
      color: 'purple',
    },
    {
      label: 'Average Speed',
      value: '45.2 MB/s',
      change: 'Optimal range',
      trend: 'stable',
      icon: TrendingUp,
      color: 'green',
    },
    {
      label: 'Uptime',
      value: '99.9%',
      change: '30 days',
      trend: 'stable',
      icon: Clock,
      color: 'amber',
    },
  ];

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      {kpis.map((kpi, index) => {
        const Icon = kpi.icon;
        const colorClasses = {
          blue: 'bg-blue-50 text-blue-600',
          purple: 'bg-purple-50 text-purple-600',
          green: 'bg-green-50 text-green-600',
          amber: 'bg-amber-50 text-amber-600',
        };

        return (
          <div key={index} className="bg-white rounded-lg border border-slate-200 p-6">
            <div className="flex items-start justify-between mb-4">
              <div className={`w-12 h-12 rounded-lg flex items-center justify-center ${colorClasses[kpi.color as keyof typeof colorClasses]}`}>
                <Icon className="w-6 h-6" />
              </div>
              {/* Sparkline placeholder */}
              <div className="h-8 w-20">
                <svg className="w-full h-full" viewBox="0 0 80 32">
                  <polyline
                    fill="none"
                    stroke={kpi.color === 'blue' ? '#3b82f6' : kpi.color === 'purple' ? '#9333ea' : kpi.color === 'green' ? '#22c55e' : '#f59e0b'}
                    strokeWidth="2"
                    points="0,20 20,15 40,10 60,12 80,8"
                  />
                </svg>
              </div>
            </div>
            
            <div className="space-y-1">
              <p className="text-sm text-slate-600">{kpi.label}</p>
              <p className="text-slate-900">{kpi.value}</p>
              <p className="text-xs text-slate-500">{kpi.change}</p>
            </div>
          </div>
        );
      })}
    </div>
  );
}
