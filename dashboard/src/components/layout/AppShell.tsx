import { useState } from "react";
import type { ReactNode } from "react";
import {
  LayoutDashboard,
  Network,
  Zap,
  Settings,
  LogOut,
  Menu,
  X
} from "lucide-react";
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
      <aside className={`w-full md:w-64 bg-card border-r border-border flex-shrink-0 flex flex-col h-auto md:h-screen sticky top-0 z-50 transition-transform duration-300 ${
        mobileMenuOpen ? 'translate-x-0' : '-translate-x-full md:translate-x-0'
      } fixed md:static`}>
        <div className="p-4 border-b border-border flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center animate-pulse">
              <span className="text-xl">🛸</span>
            </div>
            <div>
              <h1 className="font-bold text-lg leading-none">Orbit</h1>
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

        <nav className="flex-1 p-4 space-y-1 overflow-y-auto">
          {navItems.map((item) => (
            <button
              key={item.id}
              onClick={() => handleNavigate(item.id)}
              className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-md text-sm font-medium transition-all duration-200 ${
                currentPage === item.id
                  ? "bg-primary text-primary-foreground shadow-md"
                  : "text-muted-foreground hover:bg-accent hover:text-foreground"
              }`}
            >
              <item.icon size={18} />
              {item.label}
            </button>
          ))}
        </nav>

        <div className="p-4 border-t border-border space-y-4">
          <div className="flex items-center justify-between px-2">
            <span className="text-sm font-medium text-muted-foreground">Theme</span>
            <ModeToggle />
          </div>
          <div className="bg-muted/50 rounded-lg p-3">
            <div className="flex items-center gap-3">
              <div className="w-8 h-8 rounded-full bg-gradient-to-tr from-blue-500 to-purple-500 flex items-center justify-center text-white text-xs font-bold">
                AD
              </div>
              <div className="flex-1 overflow-hidden">
                <p className="text-sm font-medium truncate">Administrator</p>
                <p className="text-xs text-muted-foreground truncate">admin@orbit.local</p>
              </div>
              <button className="text-muted-foreground hover:text-red-500 transition-colors">
                <LogOut size={16} />
              </button>
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

        <div className="max-w-7xl mx-auto p-4 md:p-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
          {children}
        </div>
      </main>
    </div>
  );
}
