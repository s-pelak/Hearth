import { AlertCircle, CheckCircle2, Circle, XCircle } from "lucide-react";
import type { Status } from "../data/sample";
import { statusLabel } from "../data/sample";

/** Status always pairs a word with a shape, never color alone. */
export function StatusTag({ status }: { status: Status }) {
  const icon = {
    working: <span className="pulse" aria-hidden />,
    needs_you: <AlertCircle size={12} aria-hidden />,
    review: <CheckCircle2 size={12} aria-hidden />,
    done: <Circle size={12} aria-hidden />,
    failed: <XCircle size={12} aria-hidden />,
  }[status];
  return (
    <span className={`status ${status}`}>
      {icon}
      {statusLabel[status]}
    </span>
  );
}

export function Switch({ label, checked, onChange, detail }: { label: string; checked: boolean; onChange: (v: boolean) => void; detail?: string }) {
  return (
    <label className="setting">
      <span className="setting-text">
        <span>{label}</span>
        {detail && <span className="caption">{detail}</span>}
      </span>
      <input type="checkbox" role="switch" className="switch" checked={checked} onChange={(e) => onChange(e.target.checked)} />
    </label>
  );
}

export function ContextMeter({ used, window }: { used: number; window: number }) {
  const share = used / window;
  return (
    <span className={`meter${share > 0.8 ? " high" : ""}`} title={`${used}k of ${window}k tokens of context used`}>
      <i style={{ ["--w" as string]: `${Math.round(share * 100)}%` }} />
      {used}k / {window}k
    </span>
  );
}
