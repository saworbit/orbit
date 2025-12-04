import { useState } from "react";
import { AppShell } from "./components/layout/AppShell";
import { ThemeProvider } from "./components/theme-provider";
import DashboardOverview from "./components/dashboard/Overview";
import JobList from "./components/jobs/JobList";
import { JobDetail } from "./components/jobs/JobDetail";
import { QuickTransfer } from "./components/jobs/QuickTransfer";
import PipelineEditor from "./components/pipelines/PipelineEditor";
import UserList from "./components/admin/UserList";

function App() {
  const [page, setPage] = useState("dashboard");
  const [selectedJobId, setSelectedJobId] = useState<number | null>(null);

  const handleNavigate = (newPage: string) => {
    setPage(newPage);
    setSelectedJobId(null);
  };

  const handleJobSelect = (id: number) => {
    setSelectedJobId(id);
    setPage("jobs");
  };

  return (
    <ThemeProvider defaultTheme="dark" storageKey="orbit-theme">
      <AppShell currentPage={page} onNavigate={handleNavigate}>

        {page === "dashboard" && <DashboardOverview />}

        {page === "jobs" && (
          selectedJobId ? (
            <JobDetail jobId={selectedJobId} onBack={() => setSelectedJobId(null)} />
          ) : (
            <div className="space-y-6">
              <div className="flex justify-between items-center">
                <div>
                  <h2 className="text-2xl font-bold">Job Management</h2>
                  <p className="text-muted-foreground mt-1">Monitor and control data transfer operations</p>
                </div>
                <button
                  onClick={() => setPage("pipelines")}
                  className="bg-primary text-primary-foreground px-4 py-2 rounded-md text-sm font-medium hover:bg-primary/90 shadow-lg shadow-primary/20"
                >
                  + New Job
                </button>
              </div>
              {/* Pass selection handler to JobList */}
              <JobList onSelectJob={handleJobSelect} />
            </div>
          )
        )}

        {page === "pipelines" && (
          <div className="space-y-6">
            <div>
              <h2 className="text-2xl font-bold">Pipeline Studio</h2>
              <p className="text-muted-foreground mt-1">Design complex data workflows or initiate quick transfers</p>
            </div>
            <QuickTransfer />
            <div className="mt-8">
              <h3 className="text-lg font-semibold mb-4 border-b pb-2">Visual Workflow Editor</h3>
              <PipelineEditor />
            </div>
          </div>
        )}

        {page === "admin" && <UserList />}

      </AppShell>
    </ThemeProvider>
  );
}

export default App;
