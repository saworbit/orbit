import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { api } from '../../lib/api';
import { Folder, File, HardDrive } from 'lucide-react';

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  modified: number;
}

interface FileBrowserProps {
  onSelect: (path: string) => void;
}

export function FileBrowser({ onSelect }: FileBrowserProps) {
  const [currentPath, setCurrentPath] = useState<string>('.');

  const { data: files, isLoading } = useQuery({
    queryKey: ['files', currentPath],
    queryFn: async () => {
      const res = await api.post<FileEntry[]>('/list_dir', { path: currentPath });
      return res.data;
    },
  });

  const handleNavigate = (entry: FileEntry) => {
    if (entry.is_dir) {
      setCurrentPath(entry.path);
    }
    onSelect(entry.path);
  };

  if (isLoading) return <div className="p-4">Loading file system...</div>;

  return (
    <div className="border border-border rounded-md bg-card">
      <div className="bg-muted p-2 border-b border-border font-mono text-xs flex items-center gap-2 text-muted-foreground">
        <HardDrive size={16} />
        <span className="truncate">{currentPath}</span>
      </div>
      <div className="h-64 overflow-y-auto p-2">
        {files?.map((file) => (
          <div
            key={file.path}
            className="flex items-center gap-2 p-1 hover:bg-accent cursor-pointer rounded transition-colors"
            onClick={() => handleNavigate(file)}
          >
            {file.is_dir ? (
              <Folder size={16} className="text-blue-500 dark:text-blue-400" />
            ) : (
              <File size={16} className="text-muted-foreground" />
            )}
            <span className="text-sm truncate text-foreground">{file.name}</span>
            {!file.is_dir && (
              <span className="text-xs text-muted-foreground ml-auto">
                {(file.size / 1024).toFixed(1)} KB
              </span>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
