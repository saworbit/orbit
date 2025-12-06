import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "../../lib/api";
import {
  Folder,
  File as FileIcon,
  HardDrive,
  ArrowUp,
  CheckCircle2,
  Loader2,
} from "lucide-react";
import { Button } from "../ui/button";
import { cn } from "../../lib/utils";

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

interface FileBrowserProps {
  onSelect: (path: string) => void;
  selectedPath?: string;
}

export function FileBrowser({ onSelect, selectedPath }: FileBrowserProps) {
  const [currentPath, setCurrentPath] = useState<string>(".");

  // Fetch files from the API
  const {
    data: files,
    isLoading,
    error,
  } = useQuery({
    queryKey: ["files", currentPath],
    queryFn: async () => {
      const res = await api.get<FileEntry[]>(
        `/files/list?path=${encodeURIComponent(currentPath)}`
      );
      return res.data;
    },
  });

  // Navigate into a folder
  const handleNavigate = (path: string) => {
    setCurrentPath(path);
  };

  // Go up one level
  const handleUp = () => {
    if (currentPath === "." || currentPath === "/") return;
    // Simple parent logic for Windows/Unix paths
    const parent = currentPath.split(/[/\\]/).slice(0, -1).join("/") || "/";
    setCurrentPath(parent);
  };

  return (
    <div className="flex flex-col h-full border rounded-lg bg-background overflow-hidden shadow-sm">
      {/* 1. Header & Breadcrumbs */}
      <div className="bg-muted/50 p-2 border-b flex items-center gap-2 text-sm">
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={handleUp}
        >
          <ArrowUp size={14} />
        </Button>
        <div className="flex items-center gap-1 font-mono text-muted-foreground flex-1 truncate">
          <HardDrive size={14} />
          <span title={currentPath}>{currentPath}</span>
        </div>
      </div>

      {/* 2. File List */}
      <div className="flex-1 overflow-y-auto p-1 bg-white dark:bg-slate-950">
        {isLoading && (
          <div className="flex items-center justify-center h-20 text-muted-foreground gap-2">
            <Loader2 className="animate-spin" size={16} /> Loading...
          </div>
        )}

        {error && (
          <div className="p-4 text-red-500 text-sm text-center">
            Failed to list files. Is the backend running?
          </div>
        )}

        <div className="space-y-0.5">
          {!isLoading &&
            files?.map((file) => {
              const isSelected = selectedPath === file.path;
              return (
                <div
                  key={file.path}
                  className={cn(
                    "flex items-center gap-2 p-2 rounded-md cursor-pointer text-sm transition-colors",
                    isSelected
                      ? "bg-blue-100 dark:bg-blue-900/50 text-blue-900 dark:text-blue-100"
                      : "hover:bg-slate-100 dark:hover:bg-slate-800"
                  )}
                  onClick={() => {
                    if (file.is_dir) {
                      handleNavigate(file.path);
                    } else {
                      onSelect(file.path);
                    }
                  }}
                >
                  {file.is_dir ? (
                    <Folder
                      size={16}
                      className="text-blue-500 fill-blue-500/20"
                    />
                  ) : (
                    <FileIcon size={16} className="text-slate-400" />
                  )}

                  <span className="flex-1 truncate">{file.name}</span>

                  {/* Visual indicator for directories to drill down */}
                  {file.is_dir && (
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6 ml-auto"
                      onClick={(e) => {
                        e.stopPropagation(); // Don't trigger navigation twice
                        onSelect(file.path); // Select the FOLDER itself
                      }}
                    >
                      <CheckCircle2
                        size={14}
                        className={
                          isSelected ? "text-blue-600" : "text-slate-300"
                        }
                      />
                    </Button>
                  )}

                  {/* Selection checkmark for files */}
                  {!file.is_dir && isSelected && (
                    <CheckCircle2 size={14} className="text-blue-600 ml-auto" />
                  )}
                </div>
              );
            })}
        </div>
      </div>

      {/* 3. Footer Actions */}
      <div className="p-2 border-t bg-muted/50 flex justify-between items-center text-xs text-muted-foreground">
        <span>{files?.length || 0} items</span>
        <Button
          variant="secondary"
          size="sm"
          className="h-7 text-xs"
          onClick={() => onSelect(currentPath)}
        >
          Select Current Folder
        </Button>
      </div>
    </div>
  );
}
