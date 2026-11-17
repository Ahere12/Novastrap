import { useEffect, useState, type ReactElement } from "react";
import { backend, errorMessage } from "./api/backend";
import { AppProvider, useApp, type PageId } from "./app/context";
import { Accounts } from "./pages/Accounts";
import { Catalog } from "./pages/Catalog";
import { Customize } from "./pages/Customize";
import { Discover } from "./pages/Discover";
import { Home } from "./pages/Home";
import { Instances } from "./pages/Instances";
import { Performance } from "./pages/Performance";
import { Servers } from "./pages/Servers";
import { Settings } from "./pages/Settings";
import { Trading } from "./pages/Trading";
import { GameModal } from "./ui/games";
import { Logo } from "./ui/Logo";
import "./App.css";

const NAV: { id: PageId; label: string; icon: string }[] = [
  { id: "home", label: "Home", icon: "M3 11 12 3l9 8v9a1 1 0 0 1-1 1h-5v-6h-6v6H4a1 1 0 0 1-1-1z" },
  { id: "discover", label: "Discover", icon: "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20m4 6-2.5 5.5L8 16l2.5-5.5z" },
  { id: "servers", label: "Servers", icon: "M4 4h16v6H4zm0 10h16v6H4zm3-7h.01M7 17h.01" },
  { id: "instances", label: "Instances", icon: "M3 5h12v10H3zM9 9h12v10H9" },
  { id: "accounts", label: "Accounts", icon: "M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8m-8 9a8 8 0 0 1 16 0" },
  { id: "performance", label: "Performance", icon: "M4 6h10M18 6h2M4 12h4M12 12h8M4 18h12M20 18h0M16 4v4M10 10v4M18 16v4" },
  { id: "customize", label: "Customize", icon: "M12 3a9 9 0 1 0 0 18c1 0 1.5-.8 1.5-1.5 0-1.2-1-1.5-1-2.5s.8-1.5 2-1.5H17a4 4 0 0 0 4-4c0-4.4-4-8.5-9-8.5M7.5 11h.01M10 7h.01M15 7h.01" },
  { id: "catalog", label: "Catalog", icon: "M6 7h12l-1 13H7zM9 7a3 3 0 0 1 6 0" },
  { id: "trading", label: "Trading", icon: "M4 8h13l-3-3M20 16H7l3 3" },
  { id: "settings", label: "Settings", icon: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6m7.4-3a7.4 7.4 0 0 0-.1-1.2l2-1.6-2-3.4-2.4 1a7 7 0 0 0-2-1.2L14.5 3h-5l-.4 2.6a7 7 0 0 0-2 1.2l-2.4-1-2 3.4 2 1.6a7.4 7.4 0 0 0 0 2.4l-2 1.6 2 3.4 2.4-1a7 7 0 0 0 2 1.2l.4 2.6h5l.4-2.6a7 7 0 0 0 2-1.2l2.4 1 2-3.4-2-1.6c.1-.4.1-.8.1-1.2" },
];

const FONT_STACK: Record<string, string> = {
  system: `"Segoe UI Variable", "Segoe UI", system-ui, sans-serif`,
  segoe: `"Segoe UI", sans-serif`,
  bahnschrift: `Bahnschrift, "Segoe UI", sans-serif`,
  consolas: `Consolas, "Cascadia Code", monospace`,
  georgia: `Georgia, serif`,
};

function Icon({ d }: { d: string }) {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={d} />
    </svg>
  );
}

/** Applies appearance settings (accent, font, motion, background) to the document. */
function useTheme() {
  const { settings } = useApp();
  const [bg, setBg] = useState<string | null>(null);
  const ap = settings?.appearance;
  useEffect(() => {
    if (!ap) return;
    const r = document.documentElement.style;
    r.setProperty("--accent", ap.accent);
    r.setProperty("--font", FONT_STACK[ap.font] ?? FONT_STACK.system);
    document.documentElement.dataset.motion = ap.reduceMotion ? "reduced" : "full";
  }, [ap]);
  useEffect(() => {
    const load = () => backend.backgroundData().then(setBg).catch(() => setBg(null));
    void load();
    window.addEventListener("novastrap-background", load);
    return () => window.removeEventListener("novastrap-background", load);
  }, [ap?.background]);
  return { bg: ap?.background ? bg : null, dim: ap?.backgroundDim ?? 70 };
}

function Shell() {
  const { page, go, game, toasts, toast } = useApp();
  const [version, setVersion] = useState("");
  const { bg, dim } = useTheme();
  useEffect(() => { backend.appInfo().then((i) => setVersion(i.version)).catch(() => undefined); }, []);

  const pages: Record<PageId, ReactElement> = {
    home: <Home />, discover: <Discover />, servers: <Servers />, instances: <Instances />, accounts: <Accounts />,
    performance: <Performance />, customize: <Customize />, catalog: <Catalog />, trading: <Trading />, settings: <Settings />,
  };

  return (
    <div className="shell">
      {bg && <div className="bg-image" style={{ backgroundImage: `url(${bg})` }}><div style={{ background: `rgba(11,12,17,${dim / 100})` }} /></div>}
      <nav className="sidebar" aria-label="Main">
        <div className="brand"><Logo /><span>Novastrap</span></div>
        <ul>
          {NAV.map((s) => (
            <li key={s.id}>
              <button className={`nav-item ${s.id === page ? "active" : ""}`} aria-current={s.id === page ? "page" : undefined} onClick={() => go(s.id)}>
                <Icon d={s.icon} /><span>{s.label}</span>
              </button>
            </li>
          ))}
        </ul>
        <div className="sidebar-foot">
          <button className="btn btn-sm full" title="Opens or closes an always-on-top window for a second screen"
            onClick={async () => { try { await backend.hudToggle(); } catch (e) { toast(false, errorMessage(e)); } }}>
            Second-screen HUD
          </button>
          <span className="muted small">{version && `v${version}`}</span>
        </div>
      </nav>
      <main className="content" key={page}>{pages[page]}</main>
      {game && <GameModal refGame={game} />}
      <div className="toasts" aria-live="polite">
        {toasts.map((t) => <div key={t.id} className={`toast ${t.ok ? "" : "bad"}`}>{t.text}</div>)}
      </div>
    </div>
  );
}

export default function App() {
  return <AppProvider><Shell /></AppProvider>;
}
