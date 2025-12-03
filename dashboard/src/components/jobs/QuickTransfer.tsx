import { useState } from "react";
import { ArrowRight, HardDrive, Play, Settings2 } from "lucide-react";
import { Button } from "../ui/button";
import { Card } from "../ui/card";
import { FileBrowser } from "../files/FileBrowser";
import { api } from "../../lib/api";

export function QuickTransfer() {
  const [source, setSource] = useState("");
  const [dest, setDest] = useState("");
  const [mode, setMode] = useState<"copy" | "sync">("copy");
  const [isLaunching, setIsLaunching] = useState(false);

  // "Intuitive": We hide the complexity of the graph API here
  const handleLaunch = async () => {
    if (!source || !dest) return;

    setIsLaunching(true);
    try {
      // Construct the pipeline automatically
      const payload = {
        nodes: [
          { id: "src", type: "source", data: { path: source } },
          { id: "dst", type: "destination", data: { path: dest } },
        ],
        edges: [{ id: "e1", source: "src", target: "dst" }],
        config: { mode }, // copy or sync
      };

      // Call your Rust API
      await api.post("/pipelines/execute", payload);

      // Show success message
      alert(
        `${mode === "copy" ? "Copy" : "Sync"} job started successfully!\nFrom: ${source}\nTo: ${dest}`
      );

      // Reset form
      setSource("");
      setDest("");
    } catch (error) {
      console.error("Failed to start transfer:", error);
      alert(
        `Failed to start transfer: ${error instanceof Error ? error.message : "Unknown error"}`
      );
    } finally {
      setIsLaunching(false);
    }
  };

  return (
    <Card className="p-6 w-full max-w-5xl mx-auto bg-card text-card-foreground shadow-lg">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-2xl font-bold flex items-center gap-2">
          <Play className="text-green-500 fill-green-500" size={24} />
          Quick Transfer
        </h2>
        <div className="flex bg-muted p-1 rounded-lg">
          <button
            onClick={() => setMode("copy")}
            className={`px-3 py-1 text-sm rounded-md transition-all ${
              mode === "copy"
                ? "bg-background shadow text-foreground"
                : "text-muted-foreground"
            }`}
          >
            Copy
          </button>
          <button
            onClick={() => setMode("sync")}
            className={`px-3 py-1 text-sm rounded-md transition-all ${
              mode === "sync"
                ? "bg-background shadow text-foreground"
                : "text-muted-foreground"
            }`}
          >
            Sync
          </button>
        </div>
      </div>

      <div className="grid grid-cols-[1fr,auto,1fr] gap-4 items-start">
        {/* Source Side */}
        <div className="space-y-2">
          <label className="text-sm font-medium text-muted-foreground">
            Source
          </label>
          <div className="border rounded-lg p-4 bg-muted/30 min-h-[280px] flex flex-col">
            <div className="flex items-center gap-2 mb-2 text-blue-500">
              <HardDrive size={16} />
              <span className="font-mono text-xs truncate">
                {source || "Select source..."}
              </span>
            </div>
            {/* Embed FileBrowser here */}
            <div className="flex-1 overflow-hidden">
              <FileBrowser onSelect={setSource} />
            </div>
          </div>
        </div>

        {/* The Arrow (Visual Direction) */}
        <div className="flex flex-col items-center justify-center text-muted-foreground pt-8">
          <ArrowRight size={32} />
        </div>

        {/* Destination Side */}
        <div className="space-y-2">
          <label className="text-sm font-medium text-muted-foreground">
            Destination
          </label>
          <div className="border rounded-lg p-4 bg-muted/30 min-h-[280px] flex flex-col">
            <div className="flex items-center gap-2 mb-2 text-orange-500">
              <HardDrive size={16} />
              <span className="font-mono text-xs truncate">
                {dest || "Select destination..."}
              </span>
            </div>
            <div className="flex-1 overflow-hidden">
              <FileBrowser onSelect={setDest} />
            </div>
          </div>
        </div>
      </div>

      {/* Footer Actions */}
      <div className="mt-8 flex justify-between items-center">
        <p className="text-sm text-muted-foreground">
          {mode === "copy"
            ? "Copy files from source to destination"
            : "Keep source and destination in sync"}
        </p>
        <div className="flex gap-3">
          <Button variant="outline" className="gap-2">
            <Settings2 size={16} />
            Advanced Options
          </Button>
          <Button
            size="lg"
            onClick={handleLaunch}
            disabled={!source || !dest || isLaunching}
            className="bg-green-600 hover:bg-green-700 text-white min-w-[150px]"
          >
            {isLaunching ? "Starting..." : "Start Transfer"}
          </Button>
        </div>
      </div>
    </Card>
  );
}
