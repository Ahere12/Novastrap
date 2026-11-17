// Shared UI building blocks and formatters.
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { errorMessage } from "../api/backend";

export const fmtNum = (n: number | null | undefined) =>
  n == null ? "—" : Intl.NumberFormat(undefined, { notation: n >= 100000 ? "compact" : "standard", maximumFractionDigits: 1 }).format(n);

export const fmtBytes = (n: number) => {
  const u = ["B", "KB", "MB", "GB"];
  let i = 0;
  while (n >= 1024 && i < u.length - 1) { n /= 1024; i++; }
  return `${n.toFixed(i ? 1 : 0)} ${u[i]}`;
};

export const fmtDuration = (s: number) => {
  const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60);
  return h ? `${h}h ${m}m` : m ? `${m}m` : `${Math.max(0, Math.floor(s))}s`;
};

export const timeAgo = (unix: number) => {
  const d = Date.now() / 1000 - unix;
  if (d < 60) return "just now";
  if (d < 3600) return `${Math.floor(d / 60)}m ago`;
  if (d < 86400) return `${Math.floor(d / 3600)}h ago`;
  return `${Math.floor(d / 86400)}d ago`;
};

/** Loads async data with loading/error state and a reload function. */
export function useAsync<T>(fn: () => Promise<T>, deps: unknown[] = []) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const seq = useRef(0);
  const reload = useCallback(async () => {
    const id = ++seq.current;
    setLoading(true);
    setError(null);
    try {
      const v = await fn();
      if (id === seq.current) setData(v);
    } catch (e) {
      if (id === seq.current) setError(errorMessage(e));
    } finally {
      if (id === seq.current) setLoading(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  useEffect(() => { void reload(); }, [reload]);
  return { data, setData, error, loading, reload };
}

/** Runs an action, exposing busy state and a message. */
export function useAction() {
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null);
  const run = useCallback(async (fn: () => Promise<unknown>, okText?: string | ((r: unknown) => string)) => {
    setBusy(true);
    setMsg(null);
    try {
      const r = await fn();
      if (okText) setMsg({ ok: true, text: typeof okText === "function" ? okText(r) : okText });
      return r;
    } catch (e) {
      setMsg({ ok: false, text: errorMessage(e) });
      return undefined;
    } finally {
      setBusy(false);
    }
  }, []);
  return { busy, msg, setMsg, run };
}

export function Page({ title, subtitle, actions, children }: { title: string; subtitle?: string; actions?: ReactNode; children: ReactNode }) {
  return (
    <div className="page">
      <header className="page-header">
        <div>
          <h1>{title}</h1>
          {subtitle && <p className="muted">{subtitle}</p>}
        </div>
        {actions && <div className="row">{actions}</div>}
      </header>
      {children}
    </div>
  );
}

export function Card({ title, right, children, className = "" }: { title?: ReactNode; right?: ReactNode; children: ReactNode; className?: string }) {
  return (
    <section className={`card ${className}`}>
      {(title || right) && (
        <div className="card-title">
          {typeof title === "string" ? <h2>{title}</h2> : title}
          {right}
        </div>
      )}
      {children}
    </section>
  );
}

export function Msg({ msg }: { msg: { ok: boolean; text: string } | null }) {
  return msg ? <p className={msg.ok ? "ok" : "error"} role="status">{msg.text}</p> : null;
}

export function ErrorBox({ error, onRetry }: { error: string; onRetry?: () => void }) {
  return (
    <div className="error-box" role="alert">
      <span>{error}</span>
      {onRetry && <button className="btn btn-sm" onClick={onRetry}>Retry</button>}
    </div>
  );
}

export function Skeletons({ n = 3, tall = false }: { n?: number; tall?: boolean }) {
  return (
    <div className="skeleton-stack" aria-busy="true">
      {Array.from({ length: n }, (_, i) => <div key={i} className={`skeleton ${tall ? "tall" : ""}`} />)}
    </div>
  );
}

export function Empty({ children }: { children: ReactNode }) {
  return <p className="empty muted">{children}</p>;
}

export function Toggle({ checked, onChange, label, hint, disabled }: { checked: boolean; onChange: (v: boolean) => void; label: string; hint?: string; disabled?: boolean }) {
  return (
    <label className={`toggle ${disabled ? "disabled" : ""}`}>
      <span className="toggle-text">
        <span>{label}</span>
        {hint && <small className="muted">{hint}</small>}
      </span>
      <input type="checkbox" checked={checked} disabled={disabled} onChange={(e) => onChange(e.target.checked)} />
      <span className="switch" aria-hidden="true" />
    </label>
  );
}

export function Field({ label, hint, children }: { label: string; hint?: ReactNode; children: ReactNode }) {
  return (
    <label className="field">
      <span>{label}</span>
      {children}
      {hint && <small className="muted">{hint}</small>}
    </label>
  );
}

export function Img({ src, alt, className }: { src: string | null | undefined; alt: string; className?: string }) {
  const [failed, setFailed] = useState(false);
  if (!src || failed) return <div className={`img-ph ${className ?? ""}`} aria-label={alt} />;
  return <img className={className} src={src} alt={alt} loading="lazy" onError={() => setFailed(true)} />;
}

export function Modal({ onClose, children, wide }: { onClose: () => void; children: ReactNode; wide?: boolean }) {
  useEffect(() => {
    const k = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", k);
    return () => window.removeEventListener("keydown", k);
  }, [onClose]);
  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className={`modal ${wide ? "wide" : ""}`} role="dialog" aria-modal="true">
        <button className="modal-close" onClick={onClose} aria-label="Close">×</button>
        {children}
      </div>
    </div>
  );
}

export function Tabs<T extends string>({ tabs, value, onChange }: { tabs: { id: T; label: string }[]; value: T; onChange: (t: T) => void }) {
  return (
    <div className="tabs" role="tablist">
      {tabs.map((t) => (
        <button key={t.id} role="tab" aria-selected={t.id === value} className={`tab ${t.id === value ? "active" : ""}`} onClick={() => onChange(t.id)}>
          {t.label}
        </button>
      ))}
    </div>
  );
}
