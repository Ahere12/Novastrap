import { useCallback, useEffect, useState, type FormEvent } from "react";
import { backend, errorMessage, type Server, type ServerOrder } from "../api/backend";
import { useApp } from "../app/context";
import { Card, Empty, ErrorBox, Page, Skeletons, timeAgo, useAsync } from "../ui/kit";

function parsePlace(input: string): number | null {
  const s = input.trim();
  if (/^\d+$/.test(s)) return Number(s) || null;
  const m = s.match(/roblox\.com\/games\/(\d+)/i) ?? s.match(/placeId=(\d+)/i);
  return m ? Number(m[1]) : null;
}

export function ServerList({ placeId, gameName }: { placeId: number; gameName: string }) {
  const { play, favorites, refreshFavorites, toast } = useApp();
  const [order, setOrder] = useState<ServerOrder>("desc");
  const [servers, setServers] = useState<Server[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [updated, setUpdated] = useState<Date | null>(null);

  const load = useCallback(async (more: boolean, fresh: boolean) => {
    setLoading(true); setError(null);
    try {
      const p = await backend.serverList(placeId, order, more ? cursor : null, fresh);
      setServers((s) => (more ? [...s, ...p.servers] : p.servers));
      setCursor(p.nextCursor);
      setUpdated(new Date());
    } catch (e) { setError(errorMessage(e)); }
    finally { setLoading(false); }
  }, [placeId, order, cursor]);

  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => { void load(false, false); }, [placeId, order]);

  const favIds = new Set(favorites.servers.map((f) => f.jobId));
  const toggleFav = async (s: Server) => {
    try {
      const on = await backend.toggleFavoriteServer({ placeId, jobId: s.id, label: gameName, addedAt: 0 });
      toast(true, on ? "Server saved to favorites" : "Server removed from favorites");
      await refreshFavorites();
    } catch (e) { toast(false, errorMessage(e)); }
  };

  return (
    <div className="server-list">
      <div className="row between">
        <div className="row">
          <select className="input" value={order} onChange={(e) => setOrder(e.target.value as ServerOrder)} aria-label="Sort servers">
            <option value="desc">Fullest first</option>
            <option value="asc">Emptiest first</option>
          </select>
          <button className="btn" onClick={() => load(false, true)} disabled={loading}>{loading ? "Loading…" : "Refresh"}</button>
        </div>
        <span className="muted small">{updated && `Updated ${updated.toLocaleTimeString()} · `}Ping is reported by Roblox</span>
      </div>
      {error && <ErrorBox error={error} onRetry={() => load(false, true)} />}
      {loading && !servers.length && <Skeletons n={4} />}
      {!loading && !error && !servers.length && <Empty>No public servers are running right now.</Empty>}
      {servers.length > 0 && (
        <table className="table">
          <thead><tr><th>Players</th><th>Capacity</th><th>Server FPS</th><th>Ping</th><th /></tr></thead>
          <tbody>
            {servers.map((s) => (
              <tr key={s.id}>
                <td>{s.playing} / {s.maxPlayers}</td>
                <td><div className="bar"><div style={{ width: `${Math.min(100, (s.playing / Math.max(1, s.maxPlayers)) * 100)}%` }} /></div></td>
                <td>{s.fps != null ? s.fps.toFixed(0) : "—"}</td>
                <td>{s.ping != null ? `${s.ping} ms` : "—"}</td>
                <td className="actions">
                  <button className="btn btn-sm" onClick={() => toggleFav(s)} aria-label="Favorite server">{favIds.has(s.id) ? "★" : "☆"}</button>
                  <button className="btn btn-sm btn-primary" disabled={s.playing >= s.maxPlayers} onClick={() => play(String(placeId), s.id)}>Join</button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {cursor && <button className="btn" onClick={() => load(true, false)} disabled={loading}>Load more</button>}
    </div>
  );
}

/** Servers you've played on, from local activity history. */
function ServerHistory() {
  const { play } = useApp();
  const hist = useAsync(() => backend.serverHistory());
  return (
    <Card title="Server history" right={<button className="btn btn-sm" onClick={hist.reload}>Refresh</button>}>
      {hist.loading && !hist.data && <Skeletons n={2} />}
      {hist.error && <ErrorBox error={hist.error} onRetry={hist.reload} />}
      {hist.data && !hist.data.length && <Empty>Servers you join appear here (activity tracking must be on).</Empty>}
      {hist.data && hist.data.length > 0 && (
        <table className="table">
          <thead><tr><th>Experience</th><th>Server</th><th>Played</th><th /></tr></thead>
          <tbody>
            {hist.data.map((s) => (
              <tr key={s.jobId}>
                <td>{s.gameName ?? `Place ${s.placeId}`}</td>
                <td className="mono small">{s.jobId.slice(0, 8)}…</td>
                <td className="muted small">{timeAgo(s.endedAt ?? s.startedAt)}</td>
                <td className="actions"><button className="btn btn-sm btn-primary" onClick={() => play(String(s.placeId), s.jobId)}>Rejoin</button></td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      <p className="muted small">Rejoin only works while that server is still running; Roblox closes empty servers.</p>
    </Card>
  );
}

export function Servers() {
  const { params, favorites, play, refreshFavorites, toast } = useApp();
  const [input, setInput] = useState(params.place ?? "");
  const [placeId, setPlaceId] = useState<number | null>(params.place ? Number(params.place) : null);
  const game = useAsync(() => (placeId ? backend.gameDetails(null, placeId) : Promise.resolve(null)), [placeId]);
  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    const p = parsePlace(input);
    if (p) setPlaceId(p); else toast(false, "Enter a place ID or a roblox.com game link");
  };
  const removeFav = async (jobId: string, pid: number, label: string) => {
    await backend.toggleFavoriteServer({ placeId: pid, jobId, label, addedAt: 0 }).catch(() => undefined);
    await refreshFavorites();
  };

  return (
    <Page title="Servers" subtitle="Browse an experience's public servers and join a specific one.">
      <form className="join" onSubmit={onSubmit}>
        <input className="input" value={input} onChange={(e) => setInput(e.target.value)} placeholder="Place ID or roblox.com game link" aria-label="Place ID or game link" />
        <button className="btn" type="submit">Browse</button>
      </form>
      {placeId && (
        <Card title={game.data ? game.data.name : `Place ${placeId}`} right={game.data && <span className="muted small">{game.data.playing.toLocaleString()} playing</span>}>
          {game.error && <ErrorBox error={game.error} onRetry={game.reload} />}
          <ServerList placeId={placeId} gameName={game.data?.name ?? `Place ${placeId}`} />
        </Card>
      )}
      <Card title="Favorite servers">
        {favorites.servers.length ? (
          <table className="table">
            <thead><tr><th>Experience</th><th>Server</th><th /></tr></thead>
            <tbody>
              {favorites.servers.map((s) => (
                <tr key={s.jobId}>
                  <td>{s.label}</td>
                  <td className="mono small">{s.jobId.slice(0, 8)}…</td>
                  <td className="actions">
                    <button className="btn btn-sm" onClick={() => removeFav(s.jobId, s.placeId, s.label)}>Remove</button>
                    <button className="btn btn-sm btn-primary" onClick={() => play(String(s.placeId), s.jobId)}>Join</button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : <Empty>Star a server to keep it here. Servers close when empty, so saved ones can expire.</Empty>}
        <p className="muted small">Joining friends needs Roblox's signed-in session, which Novastrap doesn't use. Use Roblox's own friends list for that.</p>
      </Card>
      <ServerHistory />
    </Page>
  );
}
