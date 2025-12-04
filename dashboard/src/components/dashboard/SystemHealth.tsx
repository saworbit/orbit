import { useQuery } from "@tanstack/react-query";
import { api } from "../../lib/api";
import { Activity, HardDrive, Cpu, Zap } from "lucide-react";

// Simple SVG Sparkline component
const Sparkline = ({
  color = "currentColor",
  data = [40, 30, 45, 50, 45, 60, 55, 70, 65, 80],
}: {
  color?: string;
  data?: number[];
}) => {
  const max = Math.max(...data, 100);
  const points = data
    .map((d, i) => {
      const x = (i / (data.length - 1)) * 100;
      const y = 100 - (d / max) * 100;
      return `${x},${y}`;
    })
    .join(" ");

  return (
    <svg
      viewBox="0 0 100 100"
      className="w-full h-full overflow-visible"
      preserveAspectRatio="none"
    >
      <polyline
        fill="none"
        stroke={color}
        strokeWidth="3"
        points={points}
        vectorEffect="non-scaling-stroke"
      />
      <path
        d={`M 0 100 L 0 ${100 - (data[0] / max) * 100} L ${points
          .split(" ")
          .map((p) => "L " + p)
          .join(" ")} L 100 100 Z`}
        fill={color}
        fillOpacity="0.1"
        stroke="none"
      />
    </svg>
  );
};

export default function SystemHealth() {
  const { data: health } = useQuery({
    queryKey: ["system-health"],
    queryFn: () => api.get<any>("/stats/health").then((r) => r.data),
    refetchInterval: 2000,
  });

  const stats = health || {
    active_jobs: 0,
    total_bandwidth_mbps: 0.0,
    system_load: 0.0,
    storage_health: "Optimal",
  };

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      <HealthCard
        label="Active Jobs"
        value={stats.active_jobs}
        unit="tasks"
        icon={Zap}
        color="text-blue-500"
        bg="bg-blue-500/10"
        trend={[2, 3, 2, 4, 3, 5, stats.active_jobs]} // Mock trend + real value
      />
      <HealthCard
        label="Network I/O"
        value={stats.total_bandwidth_mbps.toFixed(1)}
        unit="MB/s"
        icon={Activity}
        color="text-green-500"
        bg="bg-green-500/10"
        trend={[10, 45, 30, 60, 55, 80, stats.total_bandwidth_mbps]}
      />
      <HealthCard
        label="System Load"
        value={(stats.system_load * 100).toFixed(0)}
        unit="%"
        icon={Cpu}
        color="text-orange-500"
        bg="bg-orange-500/10"
        trend={[20, 22, 25, 40, 35, 45, stats.system_load * 100]}
      />
      <HealthCard
        label="Storage Health"
        value={stats.storage_health}
        unit="status"
        icon={HardDrive}
        color="text-purple-500"
        bg="bg-purple-500/10"
        staticVisual
      />
    </div>
  );
}

const HealthCard = ({
  label,
  value,
  unit,
  icon: Icon,
  color,
  bg,
  trend,
  staticVisual,
}: any) => (
  <div className="bg-card border rounded-xl p-5 shadow-sm relative overflow-hidden group">
    <div className="flex justify-between items-start mb-4">
      <div>
        <p className="text-sm font-medium text-muted-foreground">{label}</p>
        <div className="flex items-baseline gap-1 mt-1">
          <span className="text-2xl font-bold">{value}</span>
          <span className="text-xs text-muted-foreground font-medium uppercase">
            {unit}
          </span>
        </div>
      </div>
      <div className={`p-2.5 rounded-lg ${bg} ${color}`}>
        <Icon size={20} />
      </div>
    </div>

    <div className="h-12 w-full mt-2 opacity-50 group-hover:opacity-100 transition-opacity">
      {staticVisual ? (
        <div className="h-full w-full bg-muted/30 rounded flex items-center justify-center text-xs text-muted-foreground">
          System Optimal
        </div>
      ) : (
        <Sparkline color="currentColor" data={trend} />
      )}
    </div>
  </div>
);
