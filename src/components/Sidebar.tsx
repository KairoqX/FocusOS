import {
  BarChart3, BookOpen, LayoutDashboard, ListChecks,
  Moon, Settings, Sun, Timer, type LucideIcon,
} from "lucide-react";
import type { Theme } from "../useTheme";

export type View =
  | "dashboard" | "tasks" | "subjects" | "focus" | "insights" | "settings";

export const NAV: { id: View; label: string; icon: LucideIcon }[] = [
  { id: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { id: "tasks", label: "Tasks", icon: ListChecks },
  { id: "subjects", label: "Subjects", icon: BookOpen },
  { id: "focus", label: "Focus", icon: Timer },
  { id: "insights", label: "Insights", icon: BarChart3 },
  { id: "settings", label: "Settings", icon: Settings },
];

interface Props {
  active: View;
  onSelect: (v: View) => void;
  theme: Theme;
  onToggleTheme: () => void;
}

export function Sidebar({ active, onSelect, theme, onToggleTheme }: Props) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <img className="brand-icon" src="/app-icon.png" alt="" width={24} height={24} />
        FocusOS
      </div>
      <nav aria-label="Primary">
        <ul>
          {NAV.map(({ id, label, icon: Icon }, i) => (
            <li key={id}>
              <button
                className="nav-item"
                aria-current={active === id ? "page" : undefined}
                onClick={() => onSelect(id)}
                title={`${label} (Ctrl+${i + 1})`}
              >
                <Icon size={18} aria-hidden="true" />
                {label}
              </button>
            </li>
          ))}
        </ul>
      </nav>
      <button className="nav-item theme-toggle" onClick={onToggleTheme}>
        {theme === "dark" ? <Sun size={18} /> : <Moon size={18} />}
        {theme === "dark" ? "Light mode" : "Dark mode"}
      </button>
    </aside>
  );
}
