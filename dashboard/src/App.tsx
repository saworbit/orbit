import { useState } from "react";
import { AppShell } from "./components/layout/AppShell";
import { ThemeProvider } from "./components/theme-provider";
import JobList from "./components/jobs/JobList";
import { QuickTransfer } from "./components/jobs/QuickTransfer";
import PipelineEditor from "./components/pipelines/PipelineEditor";
import UserList from "./components/admin/UserList";
import SystemHealth from "./components/dashboard/SystemHealth";

// Combined Dashboard View
function DashboardOverview() {
  return (
    <div className="space-y-8">
      <div>
        <h2 className="text-3xl font-bold tracking-tight mb-2">System Overview</h2>
        <p className="text-muted-foreground">Real-time metrics and active transfer monitoring.</p>
      </div>
      <SystemHealth />
      <div className="pt-4 border-t border-border">
        <h3 className="text-xl font-semibold mb-4">Recent Activity</h3>
        <JobList compact />
      </div>
    </div>
  );
}

function App() {
  const [page, setPage] = useState("dashboard");

  return (
    <ThemeProvider defaultTheme="dark" storageKey="orbit-theme">
      <AppShell currentPage={page} onNavigate={setPage}>
        {page === "dashboard" && <DashboardOverview />}
        {page === "jobs" && <JobList />}
        {page === "pipelines" && (
          <div className="space-y-6">
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-3xl font-bold tracking-tight">Pipeline Studio</h2>
                <p className="text-muted-foreground">Design complex data workflows or initiate quick transfers.</p>
              </div>
            </div>

            <div className="grid gap-8">
              <QuickTransfer />
              <div className="space-y-4">
                <h3 className="text-xl font-semibold border-b pb-2">Visual Editor</h3>
                <PipelineEditor />
              </div>
            </div>
          </div>
        )}
        {page === "admin" && <UserList />}
      </AppShell>
    </ThemeProvider>
  );
}

export default App;
