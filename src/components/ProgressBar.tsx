interface Props {
  percent: number;
  label: string;
  /** Dashed, empty-looking bar used when there is nothing to measure yet (e.g. no tasks). */
  empty?: boolean;
}

export function ProgressBar({ percent, label, empty = false }: Props) {
  return (
    <div
      className={empty ? "progress progress-empty" : "progress"}
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={percent}
    >
      <div className="progress-fill" style={{ width: `${percent}%` }} />
    </div>
  );
}
