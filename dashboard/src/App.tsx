import { useState } from 'react'
import JobWizard from './components/jobs/JobWizard'
import JobList from './components/jobs/JobList'
import PipelineEditor from './components/pipelines/PipelineEditor'
import { QuickTransfer } from './components/jobs/QuickTransfer'
import UserList from './components/admin/UserList'
import { ThemeProvider } from './components/theme-provider'
import { ModeToggle } from './components/mode-toggle'
import './App.css'

type Page = 'jobs' | 'create' | 'pipelines' | 'admin'
type PipelineView = 'quick' | 'advanced'

function App() {
  const [currentPage, setCurrentPage] = useState<Page>('jobs')
  const [pipelineView, setPipelineView] = useState<PipelineView>('quick')

  return (
    <ThemeProvider defaultTheme="dark" storageKey="orbit-theme">
      <div className="min-h-screen bg-background">
      {/* Navigation Bar */}
      <nav className="bg-card border-b shadow-sm">
        <div className="max-w-7xl mx-auto px-4">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center gap-2">
              <div className="text-2xl">🛸</div>
              <h1 className="text-xl font-bold text-foreground">Orbit Control Plane</h1>
              <span className="text-xs bg-purple-100 dark:bg-purple-900 text-purple-700 dark:text-purple-300 px-2 py-1 rounded font-semibold">
                v2.2.0-beta.1
              </span>
            </div>

            <div className="flex gap-1 items-center">
              <ModeToggle />
              <button
                onClick={() => setCurrentPage('jobs')}
                className={`px-4 py-2 rounded transition-colors ${
                  currentPage === 'jobs'
                    ? 'bg-primary text-primary-foreground'
                    : 'text-foreground hover:bg-accent'
                }`}
              >
                Jobs
              </button>
              <button
                onClick={() => setCurrentPage('create')}
                className={`px-4 py-2 rounded transition-colors ${
                  currentPage === 'create'
                    ? 'bg-primary text-primary-foreground'
                    : 'text-foreground hover:bg-accent'
                }`}
              >
                Create Job
              </button>
              <button
                onClick={() => setCurrentPage('pipelines')}
                className={`px-4 py-2 rounded transition-colors ${
                  currentPage === 'pipelines'
                    ? 'bg-primary text-primary-foreground'
                    : 'text-foreground hover:bg-accent'
                }`}
              >
                Pipelines
              </button>
              <button
                onClick={() => setCurrentPage('admin')}
                className={`px-4 py-2 rounded transition-colors ${
                  currentPage === 'admin'
                    ? 'bg-primary text-primary-foreground'
                    : 'text-foreground hover:bg-accent'
                }`}
              >
                Admin
              </button>
            </div>
          </div>
        </div>
      </nav>

      {/* Main Content */}
      <main className="py-6">
        {currentPage === 'jobs' && <JobList />}
        {currentPage === 'create' && <JobWizard />}
        {currentPage === 'pipelines' && (
          <div className="max-w-7xl mx-auto p-6">
            <div className="mb-6">
              <div className="flex gap-4 border-b border-border">
                <button
                  onClick={() => setPipelineView('quick')}
                  className={`px-4 py-2 font-medium transition-colors relative ${
                    pipelineView === 'quick'
                      ? 'text-primary'
                      : 'text-muted-foreground hover:text-foreground'
                  }`}
                >
                  Quick Transfer
                  {pipelineView === 'quick' && (
                    <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-primary" />
                  )}
                </button>
                <button
                  onClick={() => setPipelineView('advanced')}
                  className={`px-4 py-2 font-medium transition-colors relative ${
                    pipelineView === 'advanced'
                      ? 'text-primary'
                      : 'text-muted-foreground hover:text-foreground'
                  }`}
                >
                  Advanced Editor
                  {pipelineView === 'advanced' && (
                    <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-primary" />
                  )}
                </button>
              </div>
            </div>

            {pipelineView === 'quick' ? (
              <QuickTransfer />
            ) : (
              <div>
                <h2 className="text-2xl font-bold mb-6">Visual Pipeline Editor</h2>
                <PipelineEditor />
              </div>
            )}
          </div>
        )}
        {currentPage === 'admin' && (
          <div className="max-w-6xl mx-auto p-6">
            <UserList />
          </div>
        )}
      </main>
      </div>
    </ThemeProvider>
  )
}

export default App
