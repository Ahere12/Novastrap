// Second-screen HUD (separate always-on-top window).
import { useEffect, useState } from "react";
import { backend, type AccountView, type Live } from "../api/backend";
import { Img, fmtDuration } from "../ui/kit";

export function Hud() {
  const [live, setLive] = useState<Live | null>(null);
  const [account, setAccount] = useState<AccountView | null>(null);
  const [now, setNow] = useState(Date.now() / 1000);
  const [events, setEvents] = useState<string[]>([]);

  useEffect(() => {
    backend.liveActivity().then(setLive).catch(() => undefined);
    backend.accountsList().then((a) => setAccount(a.accounts.find((x) => x.active) ?? null)).catch(() => undefined);
    const un = backend.onActivity((l) => {
      setLive((prev) => {
        const t = new Date().toLocaleTimeString();
        if (l.session && prev?.session?.jobId !== l.session.jobId) setEvents((e) => [`${t} Joined ${l.session!.gameName ?? `place ${l.session!.placeId}`}`, ...e].slice(0, 8));
        if (!l.session && prev?.session) setEvents((e) => [`${t} Left ${prev.session!.gameName ?? "game"}`, ...e].slice(0, 8));
        return l;
      });
    });
    const t = setInterval(() => setNow(Date.now() / 1000), 1000);
    return () => { void un.then((f) => f()); clearInterval(t); };
  }, []);

  const s = live?.session;
  return (
    <div className="hud">
      <header className="hud-head">
        {account ? <><Img src={account.avatarUrl} alt="" className="avatar sm" /><span>{account.displayName}</span></> : <span className="muted">No active account</span>}
      </header>
      {s ? (
        <section className="hud-game">
          <Img src={live?.iconUrl} alt="" className="hud-icon" />
          <h2>{s.gameName ?? `Place ${s.placeId}`}</h2>
          <dl className="facts">
            <dt>Session</dt><dd>{fmtDuration(now - s.startedAt)}</dd>
            <dt>Server</dt><dd className="mono small">{s.jobId.slice(0, 13)}…</dd>
            <dt>Place</dt><dd className="mono small">{s.placeId}</dd>
            <dt>Discord</dt><dd className="small">{live?.discordStatus ?? "Off"}</dd>
            <dt>FPS</dt><dd className="small muted">Use Roblox's Shift+F5 stats; reading it from outside would require hooking the client.</dd>
          </dl>
        </section>
      ) : (
        <p className="muted hud-idle">Not in a game. Join one and it shows up here.{live === null ? "" : " (Activity tracking must be on.)"}</p>
      )}
      <section>
        <h3>Notifications</h3>
        {events.length ? <ul className="plain small">{events.map((e, i) => <li key={i}>{e}</li>)}</ul> : <p className="muted small">Nothing yet.</p>}
      </section>
    </div>
  );
}
