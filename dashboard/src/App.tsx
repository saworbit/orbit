import { useState } from 'react'
import JobWizard from './components/jobs/JobWizard'
import JobList from './components/jobs/JobList'
import PipelineEditor from './components/pipelines/PipelineEditor'
import './App.css'

type Page = 'jobs' | 'create' | 'pipelines'

function App() {
  const [currentPage, setCurrentPage] = useState<Page>('jobs')

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Navigation Bar */}
      <nav className="bg-white border-b shadow-sm">
        <div className="max-w-7xl mx-auto px-4">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center gap-2">
              <div className="text-2xl">🛸</div>
              <h1 className="text-xl font-bold text-gray-900">Orbit Control Plane</h1>
              <span className="text-xs bg-purple-100 text-purple-700 px-2 py-1 rounded font-semibold">
                v2.2.0-alpha.2
              </span>
            </div>

            <div className="flex gap-1">
              <button
                onClick={() => setCurrentPage('jobs')}
                className={`px-4 py-2 rounded transition-colors ${
                  currentPage === 'jobs'
                    ? 'bg-blue-600 text-white'
                    : 'text-gray-700 hover:bg-gray-100'
                }`}
              >
                Jobs
              </button>
              <button
                onClick={() => setCurrentPage('create')}
                className={`px-4 py-2 rounded transition-colors ${
                  currentPage === 'create'
                    ? 'bg-blue-600 text-white'
                    : 'text-gray-700 hover:bg-gray-100'
                }`}
              >
                Create Job
              </button>
              <button
                onClick={() => setCurrentPage('pipelines')}
                className={`px-4 py-2 rounded transition-colors ${
                  currentPage === 'pipelines'
                    ? 'bg-blue-600 text-white'
                    : 'text-gray-700 hover:bg-gray-100'
                }`}
              >
                Pipelines
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
          <div className="max-w-6xl mx-auto p-6">
            <h2 className="text-2xl font-bold mb-6">Visual Pipeline Editor</h2>
            <PipelineEditor />
          </div>
        )}
      </main>
    </div>
  )
}

export default App
