import { useState, useEffect, useRef } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "../../lib/api";
import {
  Activity,
  Server,
  HardDrive,
  ArrowUpRight,
  ArrowDownRight,
  Zap,
} from "lucide-react";

interface SystemHealth {
  active_jobs: number;
  total_bandwidth_mbps: number;
  system_load: number;
  storage_health: string;
}

interface MetricCardProps {
  label: string;
  value: string | number;
  unit: string;
  icon: React.ReactNode;
  trend?: string;
  trendUp?: boolean;
  good?: boolean;
}

// --- Visual Components ---

const AreaChart = ({
  data,
  color,
  height = 60,
}: {
  data: number[];
  color: string;
  height?: number;
}) => {
  if (data.length < 2) return null;
  const max = Math.max(...data, 100) * 1.1; // 10% headroom
  const min = 0;
  const range = max - min;

  const points = data
    .map((d, i) => {
      const x = (i / (data.length - 1)) * 100;
      const y = 100 - ((d - min) / range) * 100;
      return `${x},${y}`;
    })
    .join(" ");

  return (
    <div
      className={`relative w-full overflow-hidden`}
      style={{ height: `${height}px` }}
    >
      <svg
        viewBox="0 0 100 100"
        preserveAspectRatio="none"
        className="w-full h-full"
      >
        <defs>
          <linearGradient
            id={`grad-${color}`}
            x1="0%"
            y1="0%"
            x2="0%"
            y2="100%"
          >
            <stop offset="0%" style={{ stopColor: color, stopOpacity: 0.2 }} />
            <stop offset="100%" style={{ stopColor: color, stopOpacity: 0 }} />
          </linearGradient>
        </defs>
        <path
          d={`M 0 100 L 0 ${100 - ((data[0] - min) / range) * 100} L ${points
            .split(" ")
            .map((p) => "L " + p)
            .join(" ")} L 100 100 Z`}
          fill={`url(#grad-${color})`}
        />
        <polyline
          fill="none"
          stroke={color}
          strokeWidth="2"
          vectorEffect="non-scaling-stroke"
          points={points}
        />
      </svg>
    </div>
  );
};

// --- Main Overview Component ---

export default function DashboardOverview() {
  // Client-side buffer for "Live" feel
  const [throughputHistory, setThroughputHistory] = useState<number[]>(
    new Array(30).fill(0)
  );
  const lastBandwidthRef = useRef<number | null>(null);

  const { data: health } = useQuery({
    queryKey: ["system-health"],
    queryFn: () => api.get<SystemHealth>("/stats/health").then((r) => r.data),
    refetchInterval: 1000, // Aggressive polling for "live" feel
  });

  // Simulate updating history when data arrives
  useEffect(() => {
    const currentBandwidth = health
      ? health.total_bandwidth_mbps
      : Math.random() * 500 + 200;

    // Only update if value has changed to avoid unnecessary re-renders
    // Use queueMicrotask to defer setState and avoid synchronous cascading renders
    if (lastBandwidthRef.current !== currentBandwidth) {
      lastBandwidthRef.current = currentBandwidth;
      queueMicrotask(() => {
        setThroughputHistory((prev) => [...prev.slice(1), currentBandwidth]);
      });
    }
  }, [health]);

  const stats = health || {
    active_jobs: 3,
    total_bandwidth_mbps: 0.0,
    system_load: 0.45,
    storage_health: "Healthy",
  };

  return (
    <div className="space-y-6">
      <div className="flex flex-col md:flex-row justify-between items-end gap-4 border-b border-border pb-6">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">Mission Control</h2>
          <p className="text-muted-foreground mt-1">
            Real-time telemetry and flight status.
          </p>
        </div>
        <div className="flex items-center gap-2 text-sm text-muted-foreground bg-muted/50 px-3 py-1 rounded-full">
          <span className="relative flex h-2 w-2">
            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
            <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500"></span>
          </span>
          Live Stream Active
        </div>
      </div>

      {/* Top Metrics Row */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <MetricCard
          label="Active Jobs"
          value={stats.active_jobs}
          unit="Running"
          icon={<Zap className="text-yellow-500" />}
          trend="+1"
          trendUp={true}
        />
        <MetricCard
          label="Throughput"
          value={stats.total_bandwidth_mbps.toFixed(0)}
          unit="MB/s"
          icon={<Activity className="text-blue-500" />}
          trend="+12%"
          trendUp={true}
        />
        <MetricCard
          label="System Load"
          value={(stats.system_load * 100).toFixed(0)}
          unit="%"
          icon={<Server className="text-purple-500" />}
          trend="Stable"
        />
        <MetricCard
          label="Storage Health"
          value={stats.storage_health}
          unit="Status"
          icon={<HardDrive className="text-green-500" />}
          trend="Optimal"
          good
        />
      </div>

      {/* Main Throughput Graph */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 bg-card border rounded-xl shadow-sm p-6 relative overflow-hidden">
          <div className="flex justify-between items-center mb-6">
            <div>
              <h3 className="text-lg font-semibold">Network Throughput</h3>
              <p className="text-sm text-muted-foreground">
                Inbound/Outbound traffic analysis
              </p>
            </div>
            <div className="text-2xl font-mono font-bold text-blue-500">
              {stats.total_bandwidth_mbps.toFixed(1)}{" "}
              <span className="text-sm text-muted-foreground">MB/s</span>
            </div>
          </div>

          <div className="h-[200px] w-full">
            <AreaChart data={throughputHistory} color="#3b82f6" height={200} />
          </div>

          <div className="grid grid-cols-3 gap-4 mt-6 pt-6 border-t border-border/50">
            <div className="text-center">
              <div className="text-xs text-muted-foreground uppercase tracking-wider">
                Peak
              </div>
              <div className="font-mono font-bold">892 MB/s</div>
            </div>
            <div className="text-center border-l border-border/50">
              <div className="text-xs text-muted-foreground uppercase tracking-wider">
                Average
              </div>
              <div className="font-mono font-bold">450 MB/s</div>
            </div>
            <div className="text-center border-l border-border/50">
              <div className="text-xs text-muted-foreground uppercase tracking-wider">
                Total Transferred
              </div>
              <div className="font-mono font-bold">4.2 TB</div>
            </div>
          </div>
        </div>

        {/* Side Panel: Storage Status */}
        <div className="bg-card border rounded-xl shadow-sm p-6 flex flex-col">
          <h3 className="text-lg font-semibold mb-4">Capacity Planning</h3>
          <div className="flex-1 flex items-center justify-center relative">
            {/* Simple CSS Donut Chart */}
            <div className="w-48 h-48 rounded-full border-[16px] border-muted relative flex items-center justify-center">
              <div className="absolute inset-0 rounded-full border-[16px] border-primary border-t-transparent border-l-transparent rotate-45"></div>
              <div className="text-center">
                <div className="text-3xl font-bold">72%</div>
                <div className="text-xs text-muted-foreground uppercase">
                  Used
                </div>
              </div>
            </div>
          </div>
          <div className="space-y-3 mt-6">
            <div className="flex justify-between text-sm">
              <span className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-primary"></span> Used
                Space
              </span>
              <span className="font-mono">84.2 TB</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-muted"></span>{" "}
                Available
              </span>
              <span className="font-mono">32.8 TB</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

const MetricCard = ({
  label,
  value,
  unit,
  icon,
  trend,
  trendUp,
  good,
}: MetricCardProps) => (
  <div className="bg-card border rounded-xl p-5 shadow-sm hover:border-primary/50 transition-colors group">
    <div className="flex justify-between items-start mb-2">
      <div className="p-2 bg-muted/50 rounded-lg group-hover:bg-muted transition-colors">
        {icon}
      </div>
      {trend && (
        <div
          className={`flex items-center text-xs font-bold ${good || trendUp ? "text-green-500" : "text-muted-foreground"}`}
        >
          {trendUp ? <ArrowUpRight size={14} /> : <ArrowDownRight size={14} />}
          {trend}
        </div>
      )}
    </div>
    <div>
      <div className="text-3xl font-bold tracking-tight text-foreground">
        {value}
      </div>
      <div className="text-xs font-medium text-muted-foreground uppercase tracking-wider mt-1">
        {label} <span className="opacity-50">/ {unit}</span>
      </div>
    </div>
  </div>
);
