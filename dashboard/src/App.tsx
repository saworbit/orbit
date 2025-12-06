import { useState } from "react";
import { Header } from "./components/Header";
import { Sidebar } from "./components/Sidebar";
import { Dashboard } from "./components/screens/Dashboard";
import { Transfers } from "./components/screens/Transfers";
import { Files } from "./components/screens/Files";
import { Pipelines } from "./components/screens/Pipelines";
import { Analytics } from "./components/screens/Analytics";
import { Settings } from "./components/screens/Settings";
import { Footer } from "./components/Footer";
import { ProtectedRoute } from "./components/auth/ProtectedRoute";

type Screen =
  | "dashboard"
  | "transfers"
  | "files"
  | "pipelines"
  | "analytics"
  | "settings";

export default function App() {
  const [currentScreen, setCurrentScreen] = useState<Screen>("dashboard");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

  const renderScreen = () => {
    switch (currentScreen) {
      case "dashboard":
        return <Dashboard />;
      case "transfers":
        return <Transfers />;
      case "files":
        return <Files />;
      case "pipelines":
        return <Pipelines />;
      case "analytics":
        return <Analytics />;
      case "settings":
        return <Settings />;
      default:
        return <Dashboard />;
    }
  };

  return (
    <ProtectedRoute>
      <div className="h-screen flex flex-col bg-slate-50">
        <Header />
        <div className="flex flex-1 overflow-hidden">
          <Sidebar
            currentScreen={currentScreen}
            onNavigate={setCurrentScreen}
            collapsed={sidebarCollapsed}
            onToggleCollapse={() => setSidebarCollapsed(!sidebarCollapsed)}
          />
          <main className="flex-1 overflow-auto">{renderScreen()}</main>
        </div>
        <Footer />
      </div>
    </ProtectedRoute>
  );
}
