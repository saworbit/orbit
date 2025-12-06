import { Activity, Database, Clock, TrendingUp } from 'lucide-react';
import { useJobs } from '../../hooks/useJobs';

export function KPICards() {
  const { data: jobs, isLoading } = useJobs();

  // Calculate real statistics from job data
  const activeJobs = jobs?.filter((j) => j.status === 'running' || j.status === 'pending').length || 0;
  const completedJobs = jobs?.filter((j) => j.status === 'completed').length || 0;

  // Calculate total transferred data (assuming 1 chunk = 1MB for visualization)
  const totalChunks = jobs?.reduce((sum, j) => sum + (j.completed_chunks || 0), 0) || 0;
  const totalGB = (totalChunks / 1024).toFixed(1);

  // Calculate average progress for active jobs
  const runningJobs = jobs?.filter((j) => j.status === 'running') || [];
  const avgProgress = runningJobs.length > 0
    ? (runningJobs.reduce((sum, j) => sum + (j.progress || 0), 0) / runningJobs.length).toFixed(1)
    : '0';

  const kpis = [
    {
      label: 'Active Jobs',
      value: isLoading ? '...' : `${activeJobs}`,
      change: completedJobs > 0 ? `${completedJobs} completed` : 'No completed jobs yet',
      trend: 'up',
      icon: Activity,
      color: 'blue',
    },
    {
      label: 'Total Data Transferred',
      value: isLoading ? '...' : `${totalGB} GB`,
      change: totalChunks > 0 ? `${totalChunks} chunks completed` : 'No data transferred',
      trend: 'up',
      icon: Database,
      color: 'purple',
    },
    {
      label: 'Average Progress',
      value: isLoading ? '...' : `${avgProgress}%`,
      change: runningJobs.length > 0 ? `${runningJobs.length} jobs running` : 'No active jobs',
      trend: 'stable',
      icon: TrendingUp,
      color: 'green',
    },
    {
      label: 'Total Jobs',
      value: isLoading ? '...' : `${jobs?.length || 0}`,
      change: 'All time',
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
