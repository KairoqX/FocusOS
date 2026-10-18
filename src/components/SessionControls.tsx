import { useState } from "react";
import { Pause, Play, RotateCcw, Square } from "lucide-react";
import { useFocus, useRemainingMs } from "../focus/FocusContext";
import { ConfirmDialog } from "./ConfirmDialog";

/** Below this much elapsed time "Stop & save" is hidden (the backend refuses tiny sessions). */
const MIN_SAVE_MS = 60_000;

export function SessionControls() {
  const { snapshot, busy, pause, resume, stop, reset } = useFocus();
  const remaining = useRemainingMs();
  const [confirmReset, setConfirmReset] = useState(false);
  const active = snapshot?.active ?? null;
  if (!active || remaining === null) return null;

  const isFocus = active.kind === "focus";
  const elapsed = active.plannedMs - remaining;
  const worthSaving = isFocus && elapsed >= MIN_SAVE_MS;
  const running = active.state === "running";

  return (
    <>
      <div className="timer-actions">
        {running ? (
          <button className="btn btn-primary" onClick={() => void pause()} disabled={busy}>
            <Pause size={16} aria-hidden="true" /> Pause
          </button>
        ) : (
          <button className="btn btn-primary" onClick={() => void resume()} disabled={busy}>
            <Play size={16} aria-hidden="true" /> Resume
          </button>
        )}
        {worthSaving && (
          <button className="btn" onClick={() => void stop()} disabled={busy} title="End now and keep the time so far">
            <Square size={16} aria-hidden="true" /> Stop &amp; save
          </button>
        )}
        <button
          className="btn"
          onClick={() => (worthSaving ? setConfirmReset(true) : void reset())}
          disabled={busy}
          title={isFocus ? "Discard this session" : "Skip the break"}
        >
          <RotateCcw size={16} aria-hidden="true" /> {isFocus ? "Reset" : "Skip break"}
        </button>
      </div>
      <ConfirmDialog
        open={confirmReset}
        title="Discard this session?"
        message="The time so far will not be saved. Use “Stop & save” to keep it."
        confirmLabel="Discard"
        onCancel={() => setConfirmReset(false)}
        onConfirm={async () => {
          await reset();
          setConfirmReset(false);
        }}
      />
    </>
  );
}
