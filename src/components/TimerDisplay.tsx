import { useFocus, useRemainingMs } from "../focus/FocusContext";
import { formatClock } from "../format";
import { ProgressBar } from "./ProgressBar";

/** The big countdown. Ticking is confined to this component. */
export function TimerDisplay({ idleMs }: { idleMs: number }) {
  const { snapshot } = useFocus();
  const remaining = useRemainingMs();
  const active = snapshot?.active ?? null;
  const planned = active?.plannedMs ?? idleMs;
  const shown = remaining ?? idleMs;
  const percent = active && planned > 0 ? Math.round(((planned - shown) / planned) * 100) : 0;
  return (
    <div className="timer-display">
      <div className="clock" role="timer" aria-label={active ? "Time remaining" : "Session length"}>
        {formatClock(shown)}
      </div>
      <ProgressBar percent={Math.min(100, Math.max(0, percent))} empty={!active} label="Session progress" />
    </div>
  );
}
