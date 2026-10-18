import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  completeFocusSession,
  focusHeartbeat,
  getFocusState,
  pauseFocusSession,
  resetFocusSession,
  resumeFocusSession,
  startFocusSession,
  stopFocusSession,
  type FocusSession,
  type SessionKind,
  type TimerSnapshot,
} from "../api";
import { messageOf } from "../format";

/** How often the app proves it is alive while a session runs (see `kaze-core::timer`). */
export const HEARTBEAT_MS = 15_000;
const MAX_COMPLETE_ATTEMPTS = 3;

interface FocusContextValue {
  /** Latest backend state. The backend, not this object, is the source of truth. */
  snapshot: TimerSnapshot | null;
  /** `performance.now()` when `snapshot` arrived; used to interpolate the countdown. */
  receivedAt: number;
  /** The session that just ended and has not been dismissed yet. */
  finished: FocusSession | null;
  /** Increments whenever a session finishes, so history lists know to reload. */
  historyVersion: number;
  busy: boolean;
  error: string | null;
  start: (kind: SessionKind, taskId: number | null) => Promise<void>;
  pause: () => Promise<void>;
  resume: () => Promise<void>;
  stop: () => Promise<void>;
  reset: () => Promise<void>;
  dismissFinished: () => void;
}

const FocusContext = createContext<FocusContextValue | null>(null);

export function useFocus(): FocusContextValue {
  const value = useContext(FocusContext);
  if (!value) throw new Error("useFocus must be used inside <FocusProvider>");
  return value;
}

/**
 * Owns the timer for the whole app (not a single screen), so heartbeats and automatic completion
 * keep working while the user browses other views.
 */
export function FocusProvider({ children }: { children: ReactNode }) {
  const [snapshot, setSnapshot] = useState<TimerSnapshot | null>(null);
  const [receivedAt, setReceivedAt] = useState(0);
  const [finished, setFinished] = useState<FocusSession | null>(null);
  const [historyVersion, setHistoryVersion] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const activeRef = useRef<FocusSession | null>(null);
  const attempts = useRef(new Map<number, number>());

  const apply = useCallback((s: TimerSnapshot) => {
    activeRef.current = s.active;
    setSnapshot(s);
    setReceivedAt(performance.now());
    if (s.finished) {
      setFinished(s.finished);
      setHistoryVersion((v) => v + 1);
    }
  }, []);

  const sync = useCallback(
    () => getFocusState().then(apply).catch((e: unknown) => setError(messageOf(e))),
    [apply],
  );

  // Initial load: this is also where an interrupted session is recovered by the backend.
  useEffect(() => {
    void sync();
  }, [sync]);

  // Coming back to the window: re-sync immediately instead of trusting a possibly throttled timer.
  useEffect(() => {
    const onVisible = () => {
      if (document.visibilityState === "visible") void sync();
    };
    document.addEventListener("visibilitychange", onVisible);
    window.addEventListener("focus", onVisible);
    return () => {
      document.removeEventListener("visibilitychange", onVisible);
      window.removeEventListener("focus", onVisible);
    };
  }, [sync]);

  const active = snapshot?.active ?? null;
  const activeId = active?.id ?? null;
  const activeState = active?.state ?? null;
  const remainingAtReceipt = active?.remainingMs ?? null;

  // Heartbeat while running.
  useEffect(() => {
    if (activeState !== "running") return;
    const id = window.setInterval(() => {
      focusHeartbeat()
        .then(apply)
        .catch((e: unknown) => setError(messageOf(e)));
    }, HEARTBEAT_MS);
    return () => window.clearInterval(id);
  }, [activeId, activeState, apply]);

  // Ask the backend to complete the session when its time is up. One timeout (not a tick loop);
  // the backend validates the elapsed time, so an early or duplicate call is harmless.
  useEffect(() => {
    if (activeId === null || activeState === null || remainingAtReceipt === null) return;
    const attempt = () => {
      const n = (attempts.current.get(activeId) ?? 0) + 1;
      if (n > MAX_COMPLETE_ATTEMPTS) return;
      attempts.current.set(activeId, n);
      completeFocusSession(activeId)
        .then((s) => {
          attempts.current.delete(activeId);
          apply(s);
        })
        .catch(() => {
          void sync(); // re-read the truth; the effect re-runs with fresh numbers
        });
    };
    if (remainingAtReceipt <= 0) {
      attempt();
      return;
    }
    if (activeState === "running") {
      const t = window.setTimeout(attempt, remainingAtReceipt + 50);
      return () => window.clearTimeout(t);
    }
  }, [activeId, activeState, remainingAtReceipt, receivedAt, apply, sync]);

  const run = useCallback(
    async (action: () => Promise<TimerSnapshot>) => {
      setBusy(true);
      setError(null);
      try {
        apply(await action());
      } catch (e) {
        setError(messageOf(e));
        await sync(); // never leave the UI out of step with the backend
      } finally {
        setBusy(false);
      }
    },
    [apply, sync],
  );

  const value = useMemo<FocusContextValue>(() => {
    const withActive = (fn: (id: number) => Promise<TimerSnapshot>) => async () => {
      const current = activeRef.current;
      if (current) await run(() => fn(current.id));
    };
    return {
      snapshot,
      receivedAt,
      finished,
      historyVersion,
      busy,
      error,
      start: async (kind, taskId) => {
        setFinished(null);
        await run(() => startFocusSession(kind, taskId));
      },
      pause: withActive(pauseFocusSession),
      resume: withActive(resumeFocusSession),
      stop: withActive(stopFocusSession),
      reset: withActive(resetFocusSession),
      dismissFinished: () => setFinished(null),
    };
  }, [snapshot, receivedAt, finished, historyVersion, busy, error, run]);

  return <FocusContext.Provider value={value}>{children}</FocusContext.Provider>;
}

/**
 * Live remaining time (ms) of the active session, or null when idle. Interpolates from the last
 * backend snapshot using the monotonic clock. Only the component that calls this re-renders on
 * each tick, so use it in small leaf components.
 */
export function useRemainingMs(): number | null {
  const { snapshot, receivedAt } = useFocus();
  const active = snapshot?.active ?? null;
  const running = active?.state === "running";
  const [, setTick] = useState(0);
  useEffect(() => {
    if (!running) return;
    const id = window.setInterval(() => setTick((n) => n + 1), 250);
    return () => window.clearInterval(id);
  }, [running]);
  if (!active) return null;
  const drift = running ? performance.now() - receivedAt : 0;
  return Math.max(0, active.remainingMs - drift);
}
