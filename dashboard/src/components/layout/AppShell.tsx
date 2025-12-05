import { useState } from "react";
import type { ReactNode } from "react";
import {
  LayoutDashboard,
  Network,
  Zap,
  Settings,
  Menu,
  Activity,
  X,
} from "lucide-react";
import { ModeToggle } from "../mode-toggle";

interface AppShellProps {
  children: ReactNode;
  currentPage: string;
  onNavigate: (page: string) => void;
}

export function AppShell({ children, currentPage, onNavigate }: AppShellProps) {
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);

  const navItems = [
    { id: "dashboard", label: "Overview", icon: LayoutDashboard },
    { id: "jobs", label: "Job Manager", icon: Network },
    { id: "pipelines", label: "Pipelines", icon: Zap },
    { id: "admin", label: "Administration", icon: Settings },
  ];

  const handleMobileNavigate = (page: string) => {
    onNavigate(page);
    setIsMobileMenuOpen(false); // Close menu after selection
  };

  return (
    <div className="flex min-h-screen w-full bg-background text-foreground font-sans overflow-hidden">
      {/* --- DESKTOP SIDEBAR (Visible on md+) --- */}
      <aside className="fixed inset-y-0 left-0 z-50 hidden w-64 flex-col border-r border-border bg-card md:flex shadow-sm">
        <SidebarContent
          navItems={navItems}
          currentPage={currentPage}
          onNavigate={onNavigate}
        />
      </aside>

      {/* --- MOBILE DRAWER OVERLAY (Visible when isOpen) --- */}
      {isMobileMenuOpen && (
        <div className="fixed inset-0 z-50 flex md:hidden">
          {/* Backdrop */}
          <div
            className="fixed inset-0 bg-black/50 backdrop-blur-sm animate-in fade-in duration-200"
            onClick={() => setIsMobileMenuOpen(false)}
          />

          {/* Drawer Panel */}
          <div className="relative w-64 h-full bg-card border-r border-border shadow-xl animate-in slide-in-from-left duration-300 flex flex-col">
            <button
              onClick={() => setIsMobileMenuOpen(false)}
              className="absolute top-4 right-4 p-2 text-muted-foreground hover:text-foreground"
            >
              <X size={20} />
            </button>
            <SidebarContent
              navItems={navItems}
              currentPage={currentPage}
              onNavigate={handleMobileNavigate}
            />
          </div>
        </div>
      )}

      {/* --- MAIN CONTENT AREA --- */}
      <main className="flex-1 flex flex-col min-h-screen transition-all duration-300 md:pl-64">
        {/* Mobile Header Bar */}
        <header className="sticky top-0 z-40 flex items-center justify-between border-b border-border bg-card/80 px-6 py-4 backdrop-blur-md md:hidden">
          <div className="flex items-center gap-2">
            <span className="text-xl">🛸</span>
            <span className="font-bold tracking-tight">Orbit</span>
          </div>
          <button
            onClick={() => setIsMobileMenuOpen(true)}
            className="p-2 text-foreground hover:bg-muted rounded-md"
          >
            <Menu size={24} />
          </button>
        </header>

        {/* Content Scroll Area */}
        <div className="flex-1 overflow-y-auto bg-muted/5 p-4 md:p-8 relative">
          {/* Subtle Grid Background */}
          <div className="absolute inset-0 bg-[linear-gradient(to_right,#80808012_1px,transparent_1px),linear-gradient(to_bottom,#80808012_1px,transparent_1px)] bg-[size:24px_24px] pointer-events-none" />

          {/* Pre-Alpha Warning Banner */}
          <div className="relative z-10 mb-6 rounded-lg bg-gradient-to-r from-yellow-500/20 via-orange-500/20 to-red-500/20 border border-yellow-500/50 dark:border-yellow-600/50">
            <div className="px-4 py-3">
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

          <div className="relative mx-auto max-w-7xl space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
            {children}
          </div>
        </div>
      </main>
    </div>
  );
}

// Reusable Sidebar Content for consistency between Desktop & Mobile
function SidebarContent({
  navItems,
  currentPage,
  onNavigate,
}: {
  navItems: Array<{
    id: string;
    label: string;
    icon: React.ComponentType<{ className?: string }>;
  }>;
  currentPage: string;
  onNavigate: (page: string) => void;
}) {
  return (
    <>
      {/* Header */}
      <div className="flex h-16 items-center border-b border-border px-6 shrink-0">
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary text-primary-foreground shadow-sm">
            <span className="text-lg">🛸</span>
          </div>
          <div className="flex flex-col">
            <span className="font-bold leading-none tracking-tight">Orbit</span>
            <span className="text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">
              Control Plane
            </span>
          </div>
        </div>
      </div>

      {/* Navigation */}
      <div className="flex-1 overflow-y-auto py-6 px-4">
        <div className="mb-2 px-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground/70">
          Platform
        </div>
        <nav className="grid gap-1">
          {navItems.map((item) => (
            <button
              key={item.id}
              onClick={() => onNavigate(item.id)}
              className={`flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors ${
                currentPage === item.id
                  ? "bg-primary/10 text-primary"
                  : "text-muted-foreground hover:bg-accent hover:text-accent-foreground"
              }`}
            >
              <item.icon className="h-4 w-4" />
              {item.label}
            </button>
          ))}
        </nav>
      </div>

      {/* Footer */}
      <div className="border-t border-border p-4 mt-auto">
        <div className="flex items-center gap-3 rounded-lg border bg-background p-3 shadow-sm">
          <div className="flex h-8 w-8 items-center justify-center rounded-full bg-gradient-to-br from-blue-500 to-purple-600 text-[10px] font-bold text-white shadow-inner">
            OP
          </div>
          <div className="flex-1 overflow-hidden">
            <p className="truncate text-sm font-medium">Operator</p>
            <div className="flex items-center gap-1.5 text-xs text-green-600 dark:text-green-400 font-medium">
              <Activity className="h-3 w-3" />
              <span>Online</span>
            </div>
          </div>
          <ModeToggle />
        </div>
      </div>
    </>
  );
}
