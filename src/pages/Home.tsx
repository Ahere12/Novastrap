import { useCallback, useEffect, useState } from "react";
import { backend, type HandlerOwner, type RobloxStatus } from "../api/backend";

const HANDLER_LABEL: Record<HandlerOwner["kind"], string> = {
  none: "Not registered",
  official: "Official Roblox client",
  bloxstrap: "Bloxstrap",
  fishstrap: "Fishstrap",
  novastrap: "Novastrap",
  other: "Another application",
};

function UpdateBadge({ status }: { status: RobloxStatus }) {
  switch (status.update.state) {
    case "notInstalled":
      return <span className="badge badge-warn">Not installed</span>;
    case "upToDate":
      return <span className="badge badge-ok">Up to date</span>;
    case "updateAvailable":
      return <span className="badge badge-info">Update available</span>;
    case "unknown":
      return <span className="badge badge-muted">Couldn't check</span>;
  }
}

export function Home() {
  const [status, setStatus] = useState<RobloxStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [failure, setFailure] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setFailure(null);
    try {
      setStatus(await backend.robloxStatus());
    } catch (e) {
      setFailure(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const current = status?.installations[0];

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <h1>Home</h1>
          <p className="muted">Your Roblox installation at a glance.</p>
        </div>
        <button className="btn" onClick={refresh} disabled={loading}>
          {loading ? "Checking…" : "Refresh"}
        </button>
      </header>

      <section className="card">
        <div className="card-title">
          <h2>Roblox Player</h2>
          {status && !loading && <UpdateBadge status={status} />}
        </div>

        {loading && !status && (
          <div className="skeleton-stack">
            <div className="skeleton" />
            <div className="skeleton" />
            <div className="skeleton short" />
          </div>
        )}

        {failure && <p className="error">Status check failed: {failure}</p>}

        {status && (
          <dl className="facts">
            <dt>Installed build</dt>
            <dd className="mono">{current?.versionGuid ?? "—"}</dd>

            <dt>Latest build</dt>
            <dd className="mono">
              {status.latest ? `${status.latest.clientVersionUpload} (${status.latest.version})` : "—"}
            </dd>

            <dt>Install location</dt>
            <dd className="mono path">{current?.directory ?? "—"}</dd>

            <dt>roblox-player links open with</dt>
            <dd>{HANDLER_LABEL[status.handler.kind]}</dd>
          </dl>
        )}

        {status?.update.state === "unknown" && (
          <p className="error">
            Couldn't reach Roblox: {status.update.error.message}
            {status.update.error.retryable && " You can try Refresh again."}
          </p>
        )}
        {status?.update.state === "notInstalled" && (
          <p className="muted">
            Roblox isn't installed for this Windows user. One-click install arrives in phase 4.
          </p>
        )}
      </section>
    </div>
  );
}
