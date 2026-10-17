import { useEffect, useState } from "react";
import { getAppInfo, type AppInfo } from "./api";
import { NAV, Sidebar, type View } from "./components/Sidebar";
import { useTheme } from "./useTheme";
import { TasksView } from "./views/TasksView";
import "./App.css";

const MILESTONE: Record<Exclude<View, "dashboard" | "tasks">, string> = {
  subjects: "Milestone 3",
  focus: "Milestone 4",
  insights: "Milestone 5",
  settings: "Milestone 5",
};

function Dashboard() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getAppInfo().then(setInfo).catch((e) => setError(String(e)));
  }, []);

  return (
    <>
      <h1>Dashboard</h1>
      <p className="muted">
        Statistics appear here once the database exists (Milestone 5).
      </p>
      <section className="panel" aria-live="polite">
        <h2>Backend connection</h2>
        {error && <p role="alert" className="error">Backend error: {error}</p>}
        {!error && !info && <p className="muted">Connecting…</p>}
        {info && (
          <p>
            {info.name} v{info.version} running natively on {info.os} ({info.arch}).
          </p>
        )}
        {info && (
          <p className="muted">
            Database schema v{info.schemaVersion} · {info.dbPath}
          </p>
        )}
      </section>
    </>
  );
}

function Placeholder({ title, milestone }: { title: string; milestone: string }) {
  return (
    <>
      <h1>{title}</h1>
      <section className="panel empty">
        <p>Nothing here yet.</p>
        <p className="muted">This view is built in {milestone}.</p>
      </section>
    </>
  );
}

export default function App() {
  const [view, setView] = useState<View>("dashboard");
  const { theme, toggle } = useTheme();

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!e.ctrlKey || e.altKey || e.shiftKey) return;
      const item = NAV[Number(e.key) - 1];
      if (item) {
        e.preventDefault();
        setView(item.id);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const label = NAV.find((n) => n.id === view)?.label ?? "";

  return (
    <div className="app">
      <Sidebar active={view} onSelect={setView} theme={theme} onToggleTheme={toggle} />
      <main className="content">
        {view === "dashboard" && <Dashboard />}
        {view === "tasks" && <TasksView />}
        {view !== "dashboard" && view !== "tasks" && (
          <Placeholder title={label} milestone={MILESTONE[view]} />
        )}
      </main>
    </div>
  );
}
