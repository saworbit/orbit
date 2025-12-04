import { useState } from "react";
import type { ReactNode } from "react";
import { LayoutDashboard, Network, Zap, Settings, Menu, X } from "lucide-react";
import { ModeToggle } from "../mode-toggle";

interface AppShellProps {
  children: ReactNode;
  currentPage: string;
  onNavigate: (page: string) => void;
}

export function AppShell({ children, currentPage, onNavigate }: AppShellProps) {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  const navItems = [
    { id: "dashboard", label: "Overview", icon: LayoutDashboard },
    { id: "jobs", label: "Job Manager", icon: Network },
    { id: "pipelines", label: "Pipelines", icon: Zap },
    { id: "admin", label: "Administration", icon: Settings },
  ];

  const handleNavigate = (page: string) => {
    onNavigate(page);
    setMobileMenuOpen(false);
  };

  return (
    <div className="min-h-screen bg-background text-foreground flex flex-col md:flex-row">
      {/* Sidebar Navigation */}
      <aside
        className={`w-full md:w-64 bg-card border-r border-border flex-shrink-0 flex flex-col h-auto md:h-screen sticky top-0 z-50 transition-transform duration-300 ${
          mobileMenuOpen
            ? "translate-x-0"
            : "-translate-x-full md:translate-x-0"
        } fixed md:static`}
      >
        <div className="p-6 border-b border-border flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="relative">
              <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center shadow-lg shadow-primary/25">
                <span className="text-xl">🛸</span>
              </div>
              <span className="absolute -top-1 -right-1 flex h-2.5 w-2.5">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
                <span className="relative inline-flex rounded-full h-2.5 w-2.5 bg-green-500"></span>
              </span>
            </div>
            <div>
              <h1 className="font-bold text-lg leading-none tracking-tight">
                Orbit
              </h1>
              <span className="text-[10px] text-muted-foreground uppercase tracking-widest font-semibold">
                Control Plane
              </span>
            </div>
          </div>
          <button
            onClick={() => setMobileMenuOpen(false)}
            className="md:hidden p-2 text-muted-foreground hover:text-foreground"
          >
            <X size={20} />
          </button>
        </div>

        <div className="px-4 py-6">
          <div className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-2 px-2">
            Platform
          </div>
          <nav className="space-y-1">
            {navItems.map((item) => (
              <button
                key={item.id}
                onClick={() => handleNavigate(item.id)}
                className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-md text-sm font-medium transition-all duration-200 group ${
                  currentPage === item.id
                    ? "bg-primary/10 text-primary border-r-2 border-primary"
                    : "text-muted-foreground hover:bg-muted hover:text-foreground"
                }`}
              >
                <item.icon
                  size={18}
                  className={
                    currentPage === item.id
                      ? "animate-pulse"
                      : "group-hover:scale-110 transition-transform"
                  }
                />
                {item.label}
              </button>
            ))}
          </nav>
        </div>

        <div className="mt-auto p-4 border-t border-border space-y-4 bg-muted/10">
          <div className="bg-card border rounded-lg p-3 shadow-sm">
            <div className="flex items-center gap-3">
              <div className="w-8 h-8 rounded-full bg-gradient-to-tr from-blue-600 to-purple-600 flex items-center justify-center text-white text-xs font-bold ring-2 ring-background">
                OP
              </div>
              <div className="flex-1 overflow-hidden">
                <p className="text-sm font-medium truncate">Operator</p>
                <div className="flex items-center gap-1.5 text-xs text-green-500">
                  <svg
                    className="w-2.5 h-2.5"
                    fill="currentColor"
                    viewBox="0 0 8 8"
                  >
                    <circle cx="4" cy="4" r="3" />
                  </svg>
                  <span>System Online</span>
                </div>
              </div>
              <ModeToggle />
            </div>
          </div>
        </div>
      </aside>

      {/* Mobile Menu Backdrop */}
      {mobileMenuOpen && (
        <div
          className="fixed inset-0 bg-black/50 z-40 md:hidden"
          onClick={() => setMobileMenuOpen(false)}
        />
      )}

      {/* Main Content Area */}
      <main className="flex-1 bg-background/50 overflow-x-hidden">
        {/* Mobile Menu Button */}
        <div className="md:hidden sticky top-0 z-30 bg-card border-b border-border p-4 flex items-center justify-between">
          <button
            onClick={() => setMobileMenuOpen(true)}
            className="p-2 text-muted-foreground hover:text-foreground hover:bg-accent rounded-md"
          >
            <Menu size={24} />
          </button>
          <div className="flex items-center gap-2">
            <span className="text-xl">🛸</span>
            <span className="font-bold">Orbit</span>
          </div>
          <div className="w-10" /> {/* Spacer for centering */}
        </div>

        {/* Pre-Alpha Warning Banner */}
        <div className="bg-gradient-to-r from-yellow-500/20 via-orange-500/20 to-red-500/20 border-y border-yellow-500/50 dark:border-yellow-600/50">
          <div className="max-w-7xl mx-auto px-4 py-3">
            <div className="flex items-start gap-3">
              <div className="flex-shrink-0 mt-0.5">
                <svg
                  className="w-5 h-5 text-yellow-600 dark:text-yellow-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                  />
                </svg>
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-semibold text-yellow-900 dark:text-yellow-200">
                  ⚠️ PRE-ALPHA SOFTWARE - USE AT YOUR OWN RISK
                </p>
                <p className="text-xs text-yellow-800 dark:text-yellow-300 mt-1">
                  This dashboard is experimental and under active development.
                  APIs may change without notice.
                  <strong className="font-semibold">
                    {" "}
                    NOT recommended for production use.
                  </strong>{" "}
                  Test thoroughly with non-critical data.
                </p>
              </div>
            </div>
          </div>
        </div>

        <div className="max-w-7xl mx-auto p-4 md:p-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
          {children}
        </div>
      </main>
    </div>
  );
}
