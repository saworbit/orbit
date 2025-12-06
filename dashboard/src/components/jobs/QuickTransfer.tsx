import { useState } from "react";
import {
  ArrowRight,
  HardDrive,
  Play,
  Copy,
  RefreshCw,
  CheckCircle2,
} from "lucide-react";
import { api } from "../../lib/api";

export function QuickTransfer() {
  const [source, setSource] = useState("");
  const [dest, setDest] = useState("");
  const [mode, setMode] = useState<"copy" | "sync">("copy");
  const [status, setStatus] = useState<
    "idle" | "loading" | "success" | "error"
  >("idle");
  const [errorMsg, setErrorMsg] = useState("");

  const handleLaunch = async () => {
    if (!source || !dest) return;
    setStatus("loading");
    try {
      await api.post("/create_job", {
        source,
        destination: dest,
        verify: mode === "sync",
      });
      setStatus("success");
      // Reset after 3 seconds
      setTimeout(() => {
        setStatus("idle");
        setSource("");
        setDest("");
      }, 3000);
    } catch (error) {
      console.error(error);
      setStatus("error");
      setErrorMsg(
        error instanceof Error ? error.message : "Failed to launch job"
      );
    }
  };

  return (
    <div className="bg-card border rounded-xl shadow-sm p-6 max-w-5xl mx-auto">
      {/* Header with Mode Toggle */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center mb-8 gap-4">
        <div>
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <Play size={20} className="text-primary" />
            New Transfer Task
          </h2>
          <p className="text-sm text-muted-foreground">
            Configure a simple point-to-point data movement
          </p>
        </div>

        <div className="bg-muted p-1 rounded-lg flex self-end sm:self-auto">
          <button
            onClick={() => setMode("copy")}
            className={`flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-md transition-all ${
              mode === "copy"
                ? "bg-background text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            <Copy size={14} /> Copy
          </button>
          <button
            onClick={() => setMode("sync")}
            className={`flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-md transition-all ${
              mode === "sync"
                ? "bg-background text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            <RefreshCw size={14} /> Sync
          </button>
        </div>
      </div>

      {/* Main Flow Area */}
      <div className="grid grid-cols-1 lg:grid-cols-[1fr,auto,1fr] gap-6 items-center">
        {/* Source */}
        <div className="space-y-3">
          <label className="text-sm font-medium text-muted-foreground flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-blue-500"></span> Source
            Origin
          </label>
          <div
            className={`border-2 rounded-xl overflow-hidden transition-colors ${source ? "border-blue-500/50 bg-blue-500/5" : "border-dashed border-border bg-muted/20"}`}
          >
            <div className="h-[280px] flex flex-col">
              <div className="p-3 border-b bg-background/50 flex items-center gap-2">
                <HardDrive size={14} className="text-muted-foreground" />
                <input
                  value={source}
                  readOnly
                  placeholder="Select a path..."
                  className="bg-transparent text-sm w-full outline-none font-mono text-muted-foreground"
                />
              </div>
              <div className="flex-1 overflow-hidden relative">
                <div className="text-slate-500 p-4">File browser coming soon...</div>
              </div>
            </div>
          </div>
        </div>

        {/* Visual Connector */}
        <div className="flex lg:flex-col items-center justify-center text-muted-foreground/30 gap-2">
          <div className="h-px w-full lg:w-px lg:h-12 bg-current"></div>
          <div
            className={`p-3 rounded-full border-2 ${mode === "sync" ? "border-orange-500 text-orange-500 bg-orange-500/10" : "border-blue-500 text-blue-500 bg-blue-500/10"}`}
          >
            <ArrowRight size={24} />
          </div>
          <div className="h-px w-full lg:w-px lg:h-12 bg-current"></div>
        </div>

        {/* Destination */}
        <div className="space-y-3">
          <label className="text-sm font-medium text-muted-foreground flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-orange-500"></span>{" "}
            Destination Target
          </label>
          <div
            className={`border-2 rounded-xl overflow-hidden transition-colors ${dest ? "border-orange-500/50 bg-orange-500/5" : "border-dashed border-border bg-muted/20"}`}
          >
            <div className="h-[280px] flex flex-col">
              <div className="p-3 border-b bg-background/50 flex items-center gap-2">
                <HardDrive size={14} className="text-muted-foreground" />
                <input
                  value={dest}
                  readOnly
                  placeholder="Select a path..."
                  className="bg-transparent text-sm w-full outline-none font-mono text-muted-foreground"
                />
              </div>
              <div className="flex-1 overflow-hidden relative">
                <div className="text-slate-500 p-4">File browser coming soon...</div>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Action Footer */}
      <div className="mt-8 pt-6 border-t flex flex-col sm:flex-row justify-between items-center gap-4">
        <div className="text-sm">
          {status === "error" && (
            <span className="text-red-500 font-medium">Error: {errorMsg}</span>
          )}
          {status === "success" && (
            <span className="text-green-500 font-medium flex items-center gap-2">
              <CheckCircle2 size={16} /> Job initiated successfully!
            </span>
          )}
        </div>

        <button
          onClick={handleLaunch}
          disabled={
            !source || !dest || status === "loading" || status === "success"
          }
          className={`
            px-8 py-2.5 rounded-lg font-semibold text-white transition-all
            ${
              !source || !dest
                ? "bg-muted text-muted-foreground cursor-not-allowed"
                : status === "success"
                  ? "bg-green-600 hover:bg-green-700"
                  : "bg-primary hover:bg-primary/90 shadow-lg shadow-primary/20 hover:shadow-primary/40 translate-y-0 active:translate-y-0.5"
            }
          `}
        >
          {status === "loading"
            ? "Initializing..."
            : status === "success"
              ? "Launched"
              : "Start Transfer"}
        </button>
      </div>
    </div>
  );
}
