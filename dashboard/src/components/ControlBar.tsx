import { useState } from "react";
import { Play, Pause, Square, Settings2, Copy } from "lucide-react";

export function ControlBar() {
  const [isRunning, setIsRunning] = useState(false);
  const [isPaused, setIsPaused] = useState(false);
  const [progress, setProgress] = useState(0);

  const handleStart = () => {
    setIsRunning(true);
    setIsPaused(false);
    // Simulate progress
    const interval = setInterval(() => {
      setProgress((prev) => {
        if (prev >= 100) {
          clearInterval(interval);
          setIsRunning(false);
          return 100;
        }
        return prev + 1;
      });
    }, 100);
  };

  const handlePause = () => {
    setIsPaused(!isPaused);
  };

  const handleStop = () => {
    setIsRunning(false);
    setIsPaused(false);
    setProgress(0);
  };

  return (
    <div className="bg-white rounded-xl shadow-sm border border-slate-200 p-6 mb-6">
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            {!isRunning ? (
              <button
                onClick={handleStart}
                className="px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg flex items-center gap-2"
              >
                <Play className="w-5 h-5" />
                Start Copy
              </button>
            ) : (
              <>
                <button
                  onClick={handlePause}
                  className="px-6 py-3 bg-amber-600 hover:bg-amber-700 text-white rounded-lg flex items-center gap-2"
                >
                  <Pause className="w-5 h-5" />
                  {isPaused ? "Resume" : "Pause"}
                </button>
                <button
                  onClick={handleStop}
                  className="px-6 py-3 bg-red-600 hover:bg-red-700 text-white rounded-lg flex items-center gap-2"
                >
                  <Square className="w-5 h-5" />
                  Stop
                </button>
              </>
            )}

            <button className="px-4 py-3 bg-slate-100 hover:bg-slate-200 rounded-lg border border-slate-300 flex items-center gap-2">
              <Settings2 className="w-5 h-5" />
              Options
            </button>
          </div>

          <div className="flex items-center gap-4 text-slate-600">
            <Copy className="w-5 h-5" />
            <div>
              <div className="text-sm">Files: 0 / 1,247</div>
              <div className="text-sm">
                Speed: {isRunning && !isPaused ? "45.2 MB/s" : "0 MB/s"}
              </div>
            </div>
          </div>
        </div>

        {/* Progress Bar */}
        <div className="space-y-2">
          <div className="flex justify-between text-sm">
            <span className="text-slate-600">Overall Progress</span>
            <span className="text-slate-900">{progress}%</span>
          </div>
          <div className="h-2 bg-slate-100 rounded-full overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-blue-500 to-purple-600 transition-all duration-300"
              style={{ width: `${progress}%` }}
            />
          </div>
          <div className="flex justify-between text-sm text-slate-500">
            <span>{isRunning ? "2.1 GB of 20.4 GB" : "0 GB of 20.4 GB"}</span>
            <span>{isRunning ? "Time remaining: 6m 32s" : "Not started"}</span>
          </div>
        </div>
      </div>
    </div>
  );
}
