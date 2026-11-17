import { useCallback, useEffect, useState, type FormEvent } from "react";
import {
  backend,
  errorMessage,
  type HandlerOwner,
  type InstallProgress,
  type RobloxStatus,
} from "../api/backend";

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

function progressLabel(p: InstallProgress): string {
  switch (p.stage) {
    case "downloading": {
      const mb = (n: number) => (n / 1048576).toFixed(1);
      return p.total
        ? `Downloading the official installer… ${mb(p.downloaded)} / ${mb(p.total)} MB`
        : `Downloading the official installer… ${mb(p.downloaded)} MB`;
    }
    case "verifying":
      return "Checking the installer's Roblox Corporation signature…";
    case "installing":
      return "Running the Roblox installer. Finish any steps in its window.";
    case "done":
      return "Installed.";
  }
}

function progressFraction(p: InstallProgress): number | null {
  if (p.stage === "downloading") return p.total ? p.downloaded / p.total : null;
  return p.stage === "done" ? 1 : null;
}

export function Home() {
  const [status, setStatus] = useState<RobloxStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [failure, setFailure] = useState<string | null>(null);

  const [progress, setProgress] = useState<InstallProgress | null>(null);
  const [installing, setInstalling] = useState(false);
  const [installError, setInstallError] = useState<string | null>(null);

  const [joinInput, setJoinInput] = useState("");
  const [launching, setLaunching] = useState(false);
  const [launchMsg, setLaunchMsg] = useState<{ ok: boolean; text: string } | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setFailure(null);
    try {
      setStatus(await backend.robloxStatus());
    } catch (e) {
      setFailure(errorMessage(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const unlisten = backend.onInstallProgress(setProgress);
    return () => {
      void unlisten.then((f) => f());
    };
  }, [refresh]);

  const install = async () => {
    setInstalling(true);
    setInstallError(null);
    setProgress(null);
    try {
      await backend.installRoblox();
      await refresh();
    } catch (e) {
      setInstallError(errorMessage(e));
    } finally {
      setInstalling(false);
    }
  };

  const launch = async (input: string) => {
    setLaunching(true);
    setLaunchMsg(null);
    try {
      const r = await backend.launchRoblox(input);
      setLaunchMsg({
        ok: true,
        text: r.target.kind === "app" ? "Roblox is starting…" : `Joining place ${r.target.placeId}…`,
      });
    } catch (e) {
      setLaunchMsg({ ok: false, text: errorMessage(e) });
    } finally {
      setLaunching(false);
    }
  };

  const onJoin = (e: FormEvent) => {
    e.preventDefault();
    if (joinInput.trim()) void launch(joinInput);
  };

  const current = status?.installations[0];
  const installed = !!status && status.update.state !== "notInstalled";
  const needsUpdate = status?.update.state === "updateAvailable";
  const fraction = progress ? progressFraction(progress) : null;

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <h1>Home</h1>
          <p className="muted">Your Roblox installation at a glance.</p>
        </div>
        <button className="btn" onClick={refresh} disabled={loading || installing}>
          {loading ? "Checking…" : "Refresh"}
        </button>
      </header>

      <section className="card hero">
        <div className="hero-actions">
          {installed ? (
            <button className="btn btn-primary btn-lg" onClick={() => launch("")} disabled={launching || installing}>
              {launching ? "Starting…" : "Play"}
            </button>
          ) : (
            <button className="btn btn-primary btn-lg" onClick={install} disabled={installing || loading || !status}>
              {installing ? "Installing…" : "Install Roblox"}
            </button>
          )}
          {needsUpdate && (
            <button className="btn" onClick={install} disabled={installing}>
              {installing ? "Updating…" : "Update Roblox"}
            </button>
          )}
        </div>

        <form className="join" onSubmit={onJoin}>
          <input
            className="input"
            value={joinInput}
            onChange={(e) => setJoinInput(e.target.value)}
            placeholder="Place ID or roblox.com game link"
            aria-label="Place ID or roblox.com game link"
            disabled={!installed || installing}
          />
          <button className="btn" type="submit" disabled={!installed || launching || installing || !joinInput.trim()}>
            Join
          </button>
        </form>

        {launchMsg && <p className={launchMsg.ok ? "ok" : "error"}>{launchMsg.text}</p>}

        {installing && progress && (
          <div className="progress-block" role="status">
            <p className="muted">{progressLabel(progress)}</p>
            <div className={`progress ${fraction === null ? "indeterminate" : ""}`}>
              <div style={{ width: fraction === null ? undefined : `${Math.round(fraction * 100)}%` }} />
            </div>
          </div>
        )}
        {installError && <p className="error">Install failed: {installError}</p>}
      </section>

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
      </section>
    </div>
  );
}
