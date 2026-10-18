import { useFocus, useRemainingMs } from "../focus/FocusContext";
import { formatClock } from "../format";

/** Small countdown shown next to "Focus" in the sidebar while a session is active. */
export function NavTimer() {
  const { snapshot } = useFocus();
  const remaining = useRemainingMs();
  const active = snapshot?.active ?? null;
  if (!active || remaining === null) return null;
  const paused = active.state === "paused";
  return (
    <span
      className={paused ? "nav-timer nav-timer-paused" : "nav-timer"}
      title={`${active.kind === "break" ? "Break" : "Focus"} ${paused ? "paused" : "running"}`}
    >
      {formatClock(remaining)}
    </span>
  );
}
