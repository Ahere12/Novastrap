import { useCallback, useEffect, useState, type FormEvent } from "react";
import {
  backend, errorMessage, type HandlerOwner, type HomeData, type InstallProgress, type Live, type RobloxStatus,
} from "../api/backend";
import { useApp } from "../app/context";
import { GameTile } from "../ui/games";
import { Logo } from "../ui/Logo";
import { Card, Empty, ErrorBox, Img, Skeletons, fmtDuration, timeAgo, useAsync } from "../ui/kit";

const HANDLER_LABEL: Record<HandlerOwner["kind"], string> = {
  none: "Not registered", official: "Official Roblox client", bloxstrap: "Bloxstrap",
  fishstrap: "Fishstrap", novastrap: "Novastrap", other: "Another application",
};

function UpdateBadge({ status }: { status: RobloxStatus }) {
  switch (status.update.state) {
    case "notInstalled": return <span className="badge badge-warn">Not installed</span>;
    case "upToDate": return <span className="badge badge-ok">Up to date</span>;
    case "updateAvailable": return <span className="badge badge-info">Update available</span>;
    case "unknown": return <span className="badge badge-muted">Couldn't check</span>;
  }
}

function progressLabel(p: InstallProgress): string {
  const mb = (n: number) => (n / 1048576).toFixed(1);
  switch (p.stage) {
    case "downloading": return `Downloading the official installer… ${mb(p.downloaded)}${p.total ? ` / ${mb(p.total)}` : ""} MB`;
    case "verifying": return "Checking the installer's Roblox Corporation signature…";
    case "installing": return "Running the Roblox installer. Finish any steps in its window.";
    case "done": return "Installed.";
  }
}

export function NowPlaying({ live }: { live: Live }) {
  const [now, setNow] = useState(Date.now() / 1000);
  useEffect(() => { const t = setInterval(() => setNow(Date.now() / 1000), 1000); return () => clearInterval(t); }, []);
  const s = live.session;
  if (!s) return null;
  return (
    <div className="now-playing">
      <Img src={live.iconUrl} alt="" className="np-icon" />
      <div>
        <span className="badge badge-ok">In game</span>
        <b>{s.gameName ?? `Place ${s.placeId}`}</b>
        <span className="muted small">{fmtDuration(now - s.startedAt)} this session</span>
      </div>
    </div>
  );
}

/** Banner art: a screenshot of the last-played experience, when there is one. */
function useHeroImage(universeId: number | null | undefined) {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    if (!universeId) { setUrl(null); return; }
    let live = true;
    backend.gameDetails(universeId, null)
      .then((d) => { if (live) setUrl(d.thumbnails[0] ?? null); })
      .catch(() => { if (live) setUrl(null); });
    return () => { live = false; };
  }, [universeId]);
  return url;
}

export function Home() {
  const { play, favorites, go, settings, saveSettings, toast, click } = useApp();
  const status = useAsync(() => backend.robloxStatus());
  const home = useAsync<HomeData>(() => backend.homeData());
  const accounts = useAsync(() => backend.accountsList());
  const profiles = useAsync(() => backend.instanceProfiles());
  const [progress, setProgress] = useState<InstallProgress | null>(null);
  const [installing, setInstalling] = useState(false);
  const [launching, setLaunching] = useState(false);
  const [installError, setInstallError] = useState<string | null>(null);
  const [joinInput, setJoinInput] = useState("");
  const [live, setLive] = useState<Live | null>(null);

  useEffect(() => {
    const a = backend.onInstallProgress(setProgress);
    const b = backend.onActivity((l) => { setLive(l); if (!l.session) void home.reload(); });
    return () => { void a.then((f) => f()); void b.then((f) => f()); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const install = useCallback(async () => {
    setInstalling(true); setInstallError(null); setProgress(null);
    try { await backend.installRoblox(); await status.reload(); }
    catch (e) { setInstallError(errorMessage(e)); }
    finally { setInstalling(false); }
  }, [status]);

  const onJoin = (e: FormEvent) => { e.preventDefault(); if (joinInput.trim()) void play(joinInput); };

  const st = status.data;
  const installed = !!st && st.update.state !== "notInstalled";
  const current = st?.installations[0];
  const frac = progress?.stage === "downloading" && progress.total ? progress.downloaded / progress.total : null;
  const h = home.data;
  const nowLive = live ?? h?.live ?? null;
  const active = accounts.data?.accounts.find((a) => a.active);
  const last = h?.stats.recent[0];
  const hero = useHeroImage(last?.universeId);

  // The instance picked in the Play bar is remembered in settings.
  const list = profiles.data ?? [];
  const chosen = settings?.instances.defaultProfile ?? "";
  const selected = list.some((p) => p.id === chosen) ? chosen : "";
  const choose = (id: string) => {
    if (settings) void saveSettings({ ...settings, instances: { ...settings.instances, defaultProfile: id || null } });
  };

  const onPlay = async () => {
    if (!installed) { void install(); return; }
    if (!selected) { void play(""); return; }
    click();
    setLaunching(true);
    try {
      const r = await backend.launchInstance(selected);
      toast(true, `Started ${list.find((p) => p.id === selected)?.name ?? "instance"}`);
      r.warnings.forEach((w) => toast(false, w));
    } catch (e) { toast(false, errorMessage(e)); }
    finally { setLaunching(false); }
  };

  const playLabel = !st ? "Play" : !installed ? (installing ? "Installing…" : "Install") : launching ? "Starting…" : "Play";

  return (
    <div className="play">
      <section className={`play-hero ${hero ? "" : "art"}`} style={hero ? { backgroundImage: `url(${hero})` } : undefined}>
        {!hero && <div className="play-hero-mark"><Logo size={180} /></div>}
        <div className="play-hero-shade" />
        <div className="play-hero-info">
          {nowLive?.session ? <NowPlaying live={nowLive} /> : last ? (
            <>
              <span className="eyebrow">Last played {timeAgo(last.lastPlayed)}</span>
              <h2 className="hero-title">{last.gameName ?? `Place ${last.placeId}`}</h2>
              <button className="btn btn-ghost" onClick={() => play(String(last.placeId))} disabled={!installed}>Rejoin</button>
            </>
          ) : (
            <>
              <span className="eyebrow">Roblox Player</span>
              <h2 className="hero-title">Ready when you are</h2>
            </>
          )}
        </div>
      </section>

      <div className="play-bar">
        <div className="play-bar-side">
          <label className="play-bar-label" htmlFor="instance-pick">Instance</label>
          <select id="instance-pick" className="input play-select" value={selected} onChange={(e) => choose(e.target.value)} disabled={!settings}>
            <option value="">Roblox app (active profile)</option>
            {list.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
          </select>
          <button className="link small" onClick={() => go("instances")}>Manage instances</button>
        </div>
        <div className="play-bar-center">
          <button className="play-btn" onClick={onPlay} disabled={!st || installing || launching || status.loading}>{playLabel}</button>
          <span className="muted small">{active ? `Playing as @${active.username}` : "Roblox uses the account it's signed in to"}</span>
        </div>
        <div className="play-bar-side right">
          {st && <UpdateBadge status={st} />}
          <span className="muted small mono">{st?.latest?.version ?? (status.loading ? "Checking…" : "")}</span>
          {st?.update.state === "updateAvailable" && (
            <button className="btn btn-sm" onClick={install} disabled={installing}>{installing ? "Updating…" : "Update now"}</button>
          )}
        </div>
      </div>

      {(installing && progress) || installError ? (
        <div className="play-status" role="status">
          {installing && progress && (
            <>
              <p className="muted">{progressLabel(progress)}</p>
              <div className={`progress ${frac === null ? "indeterminate" : ""}`}><div style={{ width: frac === null ? undefined : `${Math.round(frac * 100)}%` }} /></div>
            </>
          )}
          {installError && <p className="error">Install failed: {installError}</p>}
        </div>
      ) : null}

      <div className="play-below">
        <Card title="Join a game">
          <form className="join" onSubmit={onJoin}>
            <input className="input" value={joinInput} onChange={(e) => setJoinInput(e.target.value)}
              placeholder="Place ID or roblox.com game link" aria-label="Place ID or roblox.com game link" disabled={!installed || installing} />
            <button className="btn" type="submit" disabled={!installed || installing || !joinInput.trim()}>Join</button>
          </form>
        </Card>

        <div className="grid-2">
          <Card title="Recently played" right={h && <span className="muted small">{h.stats.sessions} sessions</span>}>
            {home.loading && !h && <Skeletons n={2} />}
            {home.error && <ErrorBox error={home.error} onRetry={home.reload} />}
            {h && !h.tracking && (
              <Empty>Activity tracking is off. Turn it on in Settings → General to see recently played games and playtime.</Empty>
            )}
            {h && h.tracking && !h.stats.recent.length && <Empty>Games you play will appear here.</Empty>}
            {h && h.stats.recent.length > 0 && (
              <div className="tiles small">
                {h.stats.recent.slice(0, 8).map((g) => (
                  <GameTile key={g.placeId}
                    g={{ universeId: g.universeId ?? 0, rootPlaceId: g.placeId, name: g.gameName ?? "", iconUrl: g.universeId ? h.icons[g.universeId] ?? null : null }}
                    subtitle={timeAgo(g.lastPlayed)} />
                ))}
              </div>
            )}
          </Card>
          <Card title="Playtime">
            {h ? (
              <>
                <div className="stats-strip compact">
                  <div><b>{fmtDuration(h.stats.last7DaysSeconds)}</b><span>last 7 days</span></div>
                  <div><b>{fmtDuration(h.stats.totalSeconds)}</b><span>tracked total</span></div>
                </div>
                {h.stats.top.length ? (
                  <ol className="top-list">
                    {h.stats.top.map((g) => (
                      <li key={g.placeId}><span>{g.gameName ?? `Place ${g.placeId}`}</span><span className="muted">{fmtDuration(g.seconds)}</span></li>
                    ))}
                  </ol>
                ) : <Empty>No tracked sessions yet.</Empty>}
              </>
            ) : home.error ? null : <Skeletons n={2} />}
          </Card>
        </div>

        <div className="grid-2">
          <Card title="Favorites" right={<button className="btn btn-sm" onClick={() => go("discover")}>Discover</button>}>
            {favorites.games.length ? (
              <div className="tiles small">
                {favorites.games.slice(0, 8).map((g) => (
                  <GameTile key={g.universeId} g={{ universeId: g.universeId, rootPlaceId: g.placeId, name: g.name, iconUrl: g.iconUrl }} subtitle={g.genre ?? ""} />
                ))}
              </div>
            ) : <Empty>Favorite experiences from Discover to pin them here.</Empty>}
          </Card>
          <Card title="Recent accounts" right={<button className="btn btn-sm" onClick={() => go("accounts")}>Manage</button>}>
            {accounts.data?.accounts.length ? (
              <ul className="account-mini">
                {accounts.data.accounts.slice(0, 4).map((a) => (
                  <li key={a.userId}>
                    <Img src={a.avatarUrl} alt="" className="avatar sm" />
                    <span>{a.displayName} <span className="muted">@{a.username}</span></span>
                    {a.active && <span className="badge badge-ok">Active</span>}
                  </li>
                ))}
              </ul>
            ) : <Empty>No accounts saved yet.</Empty>}
          </Card>
        </div>

        <Card title="Roblox Player" right={<button className="btn btn-sm" onClick={() => { void status.reload(); void home.reload(); }} disabled={status.loading || installing}>{status.loading ? "Checking…" : "Refresh"}</button>}>
          {status.loading && !st && <Skeletons n={3} />}
          {status.error && <ErrorBox error={status.error} onRetry={status.reload} />}
          {st && (
            <dl className="facts">
              <dt>Installed build</dt><dd className="mono">{current?.versionGuid ?? "—"}</dd>
              <dt>Latest build</dt><dd className="mono">{st.latest ? `${st.latest.clientVersionUpload} (${st.latest.version})` : "—"}</dd>
              <dt>Install location</dt><dd className="mono path">{current?.directory ?? "—"}</dd>
              <dt>roblox-player links open with</dt><dd>{HANDLER_LABEL[st.handler.kind]}</dd>
              <dt>Discord presence</dt><dd>{settings?.integrations.discordEnabled ? (nowLive?.discordStatus ?? "Waiting for a game") : "Off"}</dd>
            </dl>
          )}
          {st?.update.state === "unknown" && <p className="error">Couldn't reach Roblox: {st.update.error.message}</p>}
        </Card>
      </div>
    </div>
  );
}
