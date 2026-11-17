import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { backend, errorMessage, type Favorites, type FavGame, type Settings } from "../api/backend";

export type PageId =
  | "home" | "discover" | "servers" | "instances" | "accounts" | "performance"
  | "customize" | "catalog" | "trading" | "settings";

export interface GameRef { universeId?: number | null; placeId?: number | null }
interface Toast { id: number; ok: boolean; text: string }

interface Ctx {
  page: PageId;
  go: (p: PageId, params?: Record<string, string>) => void;
  params: Record<string, string>;
  settings: Settings | null;
  saveSettings: (next: Settings) => Promise<boolean>;
  favorites: Favorites;
  refreshFavorites: () => Promise<void>;
  toggleFavoriteGame: (g: Omit<FavGame, "addedAt">) => Promise<void>;
  openGame: (g: GameRef) => void;
  game: GameRef | null;
  closeGame: () => void;
  play: (input: string, jobId?: string) => Promise<void>;
  toast: (ok: boolean, text: string) => void;
  toasts: Toast[];
  click: () => void;
}

const AppContext = createContext<Ctx | null>(null);

export function useApp() {
  const c = useContext(AppContext);
  if (!c) throw new Error("useApp outside provider");
  return c;
}

let ctx: AudioContext | null = null;
function tick() {
  try {
    ctx ??= new AudioContext();
    const o = ctx.createOscillator(), g = ctx.createGain();
    o.frequency.value = 880;
    g.gain.setValueAtTime(0.03, ctx.currentTime);
    g.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 0.06);
    o.connect(g).connect(ctx.destination);
    o.start();
    o.stop(ctx.currentTime + 0.06);
  } catch { /* audio unavailable */ }
}

export function AppProvider({ children }: { children: ReactNode }) {
  const [page, setPage] = useState<PageId>("home");
  const [params, setParams] = useState<Record<string, string>>({});
  const [settings, setSettings] = useState<Settings | null>(null);
  const [favorites, setFavorites] = useState<Favorites>({ games: [], servers: [], items: [] });
  const [game, setGame] = useState<GameRef | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);

  const toast = useCallback((ok: boolean, text: string) => {
    const id = Date.now() + Math.random();
    setToasts((t) => [...t.slice(-3), { id, ok, text }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), ok ? 3500 : 7000);
  }, []);

  const refreshFavorites = useCallback(async () => {
    try { setFavorites(await backend.favorites()); } catch (e) { toast(false, errorMessage(e)); }
  }, [toast]);

  useEffect(() => {
    backend.getSettings().then(setSettings).catch((e) => toast(false, errorMessage(e)));
    void refreshFavorites();
  }, [refreshFavorites, toast]);

  const saveSettings = useCallback(async (next: Settings) => {
    try {
      setSettings(await backend.saveSettings(next));
      return true;
    } catch (e) {
      toast(false, errorMessage(e));
      return false;
    }
  }, [toast]);

  const click = useCallback(() => { if (settings?.appearance.uiSounds) tick(); }, [settings]);

  const value = useMemo<Ctx>(() => ({
    page, params, settings, favorites, game, toasts, toast, refreshFavorites, saveSettings, click,
    go: (p, prm = {}) => { click(); setPage(p); setParams(prm); },
    openGame: (g) => setGame(g),
    closeGame: () => setGame(null),
    toggleFavoriteGame: async (g) => {
      try {
        const now = await backend.toggleFavoriteGame({ ...g, addedAt: 0 });
        toast(true, now ? `Added ${g.name} to favorites` : `Removed ${g.name} from favorites`);
        await refreshFavorites();
      } catch (e) { toast(false, errorMessage(e)); }
    },
    play: async (input, jobId) => {
      click();
      try {
        const r = await backend.launchRoblox(input, jobId);
        toast(true, r.target.kind === "app" ? "Roblox is starting…" : `Joining place ${r.target.placeId}…`);
        r.warnings.forEach((w) => toast(false, w));
      } catch (e) { toast(false, errorMessage(e)); }
    },
  }), [page, params, settings, favorites, game, toasts, toast, refreshFavorites, saveSettings, click]);

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}
