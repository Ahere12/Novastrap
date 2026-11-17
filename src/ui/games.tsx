// Game tiles, rows and the game detail modal (with its own server list).
import { useState } from "react";
import { backend, errorMessage, type GameCard, type GameDetail } from "../api/backend";
import { useApp, type GameRef } from "../app/context";
import { ServerList } from "../pages/Servers";
import { Empty, ErrorBox, Img, Modal, Skeletons, Tabs, fmtNum, useAsync } from "./kit";

export function rating(up: number | null, down: number | null) {
  if (up == null || down == null || up + down === 0) return null;
  return Math.round((up * 100) / (up + down));
}

export function GameTile({ g, subtitle }: { g: Pick<GameCard, "universeId" | "rootPlaceId" | "name" | "iconUrl"> & Partial<GameCard>; subtitle?: string }) {
  const { openGame } = useApp();
  const r = rating(g.totalUpVotes ?? null, g.totalDownVotes ?? null);
  return (
    <button className="game-tile" onClick={() => openGame({ universeId: g.universeId, placeId: g.rootPlaceId })} title={g.name}>
      <Img src={g.iconUrl} alt="" className="game-icon" />
      <span className="game-name">{g.name || `Place ${g.rootPlaceId}`}</span>
      <span className="game-meta muted">
        {subtitle ?? (
          <>
            {r != null && <span>👍 {r}%</span>}
            {g.playerCount != null && <span>● {fmtNum(g.playerCount)}</span>}
          </>
        )}
      </span>
    </button>
  );
}

export function GameRow({ title, hint, games }: { title: string; hint?: string | null; games: GameCard[] }) {
  return (
    <section className="game-row">
      <div className="row-head">
        <h2>{title}</h2>
        {hint && <span className="muted small">{hint}</span>}
      </div>
      {games.length ? (
        <div className="tiles">{games.map((g) => <GameTile key={g.universeId} g={g} />)}</div>
      ) : (
        <Empty>Nothing here right now.</Empty>
      )}
    </section>
  );
}

export function GameModal({ refGame }: { refGame: GameRef }) {
  const { closeGame, play, favorites, toggleFavoriteGame, toast } = useApp();
  const [tab, setTab] = useState<"about" | "servers">("about");
  const [shot, setShot] = useState(0);
  const { data: g, error, loading, reload } = useAsync<GameDetail>(
    () => backend.gameDetails(refGame.universeId ?? null, refGame.placeId ?? null),
    [refGame.universeId, refGame.placeId],
  );
  const fav = g ? favorites.games.some((f) => f.universeId === g.id) : false;
  const r = g ? rating(g.upVotes, g.downVotes) : null;

  return (
    <Modal onClose={closeGame} wide>
      {loading && !g && <Skeletons n={4} tall />}
      {error && <ErrorBox error={error} onRetry={reload} />}
      {g && (
        <div className="game-detail">
          <div className="gd-media">
            {g.thumbnails.length ? (
              <>
                <Img src={g.thumbnails[shot]} alt={`${g.name} screenshot`} className="gd-shot" />
                {g.thumbnails.length > 1 && (
                  <div className="gd-dots">
                    {g.thumbnails.map((_, i) => (
                      <button key={i} className={`dot ${i === shot ? "on" : ""}`} onClick={() => setShot(i)} aria-label={`Screenshot ${i + 1}`} />
                    ))}
                  </div>
                )}
              </>
            ) : (
              <Img src={g.iconUrl} alt="" className="gd-shot" />
            )}
          </div>
          <div className="gd-head">
            <Img src={g.iconUrl} alt="" className="gd-icon" />
            <div className="gd-title">
              <h2>{g.name}</h2>
              <p className="muted">
                by {g.creator.name}{g.creator.hasVerifiedBadge ? " ✔" : ""} · {g.genre ?? "Experience"}{g.subgenre ? ` · ${g.subgenre}` : ""}
              </p>
            </div>
            <div className="row">
              <button className="btn" onClick={() => toggleFavoriteGame({ universeId: g.id, placeId: g.rootPlaceId, name: g.name, iconUrl: g.iconUrl, genre: g.genre })}>
                {fav ? "★ Favorited" : "☆ Favorite"}
              </button>
              <button className="btn" title="Adds a desktop icon that opens this experience through Roblox's own link"
                onClick={async () => { try { await backend.createGameShortcut(g.rootPlaceId, g.name); toast(true, `Desktop shortcut created for ${g.name}`); } catch (e) { toast(false, errorMessage(e)); } }}>
                Desktop shortcut
              </button>
              <button className="btn btn-primary" onClick={() => play(String(g.rootPlaceId))}>Play</button>
            </div>
          </div>
          <div className="stats-strip">
            <div><b>{fmtNum(g.playing)}</b><span>playing</span></div>
            <div><b>{fmtNum(g.visits)}</b><span>visits</span></div>
            <div><b>{r != null ? `${r}%` : "—"}</b><span>rating</span></div>
            <div><b>{fmtNum(g.favoritedCount)}</b><span>favorites</span></div>
            <div><b>{g.maxPlayers}</b><span>per server</span></div>
            <div><b>{new Date(g.updated).toLocaleDateString()}</b><span>updated</span></div>
          </div>
          <Tabs tabs={[{ id: "about", label: "About" }, { id: "servers", label: "Servers" }]} value={tab} onChange={setTab} />
          {tab === "about" ? (
            <p className="description">{g.description || "No description."}</p>
          ) : (
            <ServerList placeId={g.rootPlaceId} gameName={g.name} />
          )}
        </div>
      )}
    </Modal>
  );
}
