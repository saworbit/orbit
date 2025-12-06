import { Home, Activity, FolderTree, GitBranch, BarChart3, Settings as SettingsIcon, HelpCircle, ChevronLeft, ChevronRight } from 'lucide-react';

interface SidebarProps {
  currentScreen: string;
  onNavigate: (screen: any) => void;
  collapsed: boolean;
  onToggleCollapse: () => void;
}

export function Sidebar({ currentScreen, onNavigate, collapsed, onToggleCollapse }: SidebarProps) {
  const navItems = [
    { id: 'dashboard', label: 'Dashboard', icon: Home },
    { id: 'transfers', label: 'Transfers', icon: Activity },
    { id: 'files', label: 'Files', icon: FolderTree },
    { id: 'pipelines', label: 'Pipelines', icon: GitBranch },
    { id: 'analytics', label: 'Analytics', icon: BarChart3 },
    { id: 'settings', label: 'Settings', icon: SettingsIcon },
  ];

  return (
    <aside className={`bg-slate-900 border-r border-slate-700 flex flex-col transition-all duration-300 ${collapsed ? 'w-16' : 'w-64'}`}>
      {/* Navigation Items */}
      <nav className="flex-1 py-4">
        {navItems.map((item) => {
          const Icon = item.icon;
          const isActive = currentScreen === item.id;
          
          return (
            <button
              key={item.id}
              onClick={() => onNavigate(item.id)}
              className={`w-full flex items-center gap-3 px-4 py-3 transition-colors ${
                isActive
                  ? 'bg-blue-600 text-white'
                  : 'text-slate-300 hover:bg-slate-800 hover:text-white'
              }`}
            >
              <Icon className="w-5 h-5 flex-shrink-0" />
              {!collapsed && <span>{item.label}</span>}
            </button>
          );
        })}
      </nav>

      {/* Help & Collapse Button */}
      <div className="border-t border-slate-700">
        <button className="w-full flex items-center gap-3 px-4 py-3 text-slate-300 hover:bg-slate-800 hover:text-white">
          <HelpCircle className="w-5 h-5 flex-shrink-0" />
          {!collapsed && <span>Help</span>}
        </button>
        
        <button
          onClick={onToggleCollapse}
          className="w-full flex items-center justify-center py-3 text-slate-400 hover:text-white hover:bg-slate-800"
        >
          {collapsed ? <ChevronRight className="w-5 h-5" /> : <ChevronLeft className="w-5 h-5" />}
        </button>
      </div>
    </aside>
  );
}
