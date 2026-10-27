import { useState } from "react";
import { TitleBar } from "./components/TitleBar";
import { Sidebar, type Page } from "./components/Sidebar";
import { Dashboard } from "./pages/Dashboard";
import { Parameters } from "./pages/Parameters";
import { Profiles } from "./pages/Profiles";
import { Settings } from "./pages/Settings";
import { Wizard } from "./components/Wizard";

function App() {
  const [page, setPage] = useState<Page>("dashboard");

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
      <TitleBar />
      <div className="flex min-h-0 flex-1">
        <Sidebar page={page} onNavigate={setPage} />
        <main className="min-w-0 flex-1 overflow-y-auto">
          {page === "dashboard" && <Dashboard />}
          {page === "parameters" && <Parameters />}
          {page === "profiles" && <Profiles />}
          {page === "settings" && <Settings />}
        </main>
      </div>
      <Wizard />
    </div>
  );
}

export default App;
