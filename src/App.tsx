import { useEffect, useState, type ReactElement } from "react";
import { backend, errorMessage, type AccountView } from "./api/backend";
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
import { Img } from "./ui/kit";
import "./App.css";

/**
 * Launcher layout: a rail of sections on the left, each section owning a row of tabs across the top.
 * The Play section's first tab is the home screen with the big Play bar.
 */
interface Section { id: string; label: string; sub: string; icon: string; tabs: { id: PageId; label: string }[] }

const ICONS = {
  explore: "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20m4 6-2.5 5.5L8 16l2.5-5.5z",
  tune: "M4 6h10M18 6h2M4 12h4M12 12h8M4 18h12M16 4v4M10 10v4M18 16v4",
  market: "M6 7h12l-1 13H7zM9 7a3 3 0 0 1 6 0",
  settings: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6m7.4-3a7.4 7.4 0 0 0-.1-1.2l2-1.6-2-3.4-2.4 1a7 7 0 0 0-2-1.2L14.5 3h-5l-.4 2.6a7 7 0 0 0-2 1.2l-2.4-1-2 3.4 2 1.6a7.4 7.4 0 0 0 0 2.4l-2 1.6 2 3.4 2.4-1a7 7 0 0 0 2 1.2l.4 2.6h5l.4-2.6a7 7 0 0 0 2-1.2l2.4 1 2-3.4-2-1.6c.1-.4.1-.8.1-1.2",
  hud: "M3 5h18v11H3zM8 20h8M12 16v4",
};

const SECTIONS: Section[] = [
  { id: "play", label: "Roblox Player", sub: "Play", icon: "", tabs: [
    { id: "home", label: "Play" }, { id: "instances", label: "Instances" }, { id: "servers", label: "Servers" },
  ] },
  { id: "explore", label: "Explore", sub: "Experiences", icon: ICONS.explore, tabs: [{ id: "discover", label: "Discover" }] },
  { id: "market", label: "Marketplace", sub: "Catalog & trading", icon: ICONS.market, tabs: [
    { id: "catalog", label: "Catalog" }, { id: "trading", label: "Trading" },
  ] },
  { id: "tune", label: "Tuning", sub: "Performance & mods", icon: ICONS.tune, tabs: [
    { id: "performance", label: "Performance" }, { id: "customize", label: "Customize" },
  ] },
];
const SETTINGS_SECTION: Section = { id: "settings", label: "Settings", sub: "", icon: ICONS.settings, tabs: [
  { id: "settings", label: "Launcher settings" }, { id: "accounts", label: "Accounts" },
] };

const sectionOf = (p: PageId) => [...SECTIONS, SETTINGS_SECTION].find((s) => s.tabs.some((t) => t.id === p)) ?? SECTIONS[0];

// "system" isn't listed: it leaves the theme's own font in place (see App.css).
const FONT_STACK: Record<string, string> = {
  segoe: `"Segoe UI", sans-serif`,
  verdana: `Verdana, Tahoma, Arial, sans-serif`,
  bahnschrift: `Bahnschrift, "Segoe UI", sans-serif`,
  consolas: `Consolas, "Cascadia Code", monospace`,
  georgia: `Georgia, serif`,
};

/** "launcher" is the dark theme; anything else (including the legacy "nova") is Classic. */
export const themeOf = (t: string) => (t === "launcher" ? "launcher" : "classic");

function Icon({ d, size = 22 }: { d: string; size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
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
    const stack = FONT_STACK[ap.font];
    if (stack) r.setProperty("--font", stack); else r.removeProperty("--font");
    document.documentElement.dataset.theme = themeOf(ap.theme);
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

/** Account chip at the top of the rail; reloads when accounts change. */
function RailAccount() {
  const { go, page } = useApp();
  const [active, setActive] = useState<AccountView | null>(null);
  useEffect(() => {
    backend.accountsList().then((v) => setActive(v.accounts.find((a) => a.active) ?? null)).catch(() => setActive(null));
  }, [page]);
  return (
    <button className="rail-account" onClick={() => go("accounts")} title="Manage accounts">
      {active ? <Img src={active.avatarUrl} alt="" className="avatar sm" /> : <span className="avatar sm avatar-empty" />}
      <span className="rail-account-text">
        <b>{active ? active.displayName : "No account"}</b>
        <span className="muted small">{active ? `@${active.username}` : "Add one"}</span>
      </span>
    </button>
  );
}

function RailItem({ s, active, onClick }: { s: Section; active: boolean; onClick: () => void }) {
  return (
    <button className={`rail-item ${active ? "active" : ""}`} aria-current={active ? "page" : undefined} onClick={onClick}>
      <span className="rail-icon">{s.icon ? <Icon d={s.icon} /> : <Logo size={30} />}</span>
      <span className="rail-text"><b>{s.label}</b>{s.sub && <span>{s.sub}</span>}</span>
    </button>
  );
}

function Shell() {
  const { page, go, game, toasts, toast } = useApp();
  const [version, setVersion] = useState("");
  const { bg, dim } = useTheme();
  useEffect(() => { backend.appInfo().then((i) => setVersion(i.version)).catch(() => undefined); }, []);
  const section = sectionOf(page);

  const pages: Record<PageId, ReactElement> = {
    home: <Home />, discover: <Discover />, servers: <Servers />, instances: <Instances />, accounts: <Accounts />,
    performance: <Performance />, customize: <Customize />, catalog: <Catalog />, trading: <Trading />, settings: <Settings />,
  };

  return (
    <div className="shell">
      {bg && <div className="bg-image" style={{ backgroundImage: `url(${bg})` }}><div style={{ background: `rgba(18,18,20,${dim / 100})` }} /></div>}
      <nav className="rail" aria-label="Sections">
        <RailAccount />
        <div className="rail-list">
          {SECTIONS.map((s) => <RailItem key={s.id} s={s} active={s.id === section.id} onClick={() => go(s.tabs[0].id)} />)}
        </div>
        <div className="rail-foot">
          <button className="rail-item small" title="Opens or closes an always-on-top window for a second screen"
            onClick={async () => { try { await backend.hudToggle(); } catch (e) { toast(false, errorMessage(e)); } }}>
            <span className="rail-icon"><Icon d={ICONS.hud} size={20} /></span><span className="rail-text"><b>Second-screen HUD</b></span>
          </button>
          <RailItem s={SETTINGS_SECTION} active={section.id === "settings"} onClick={() => go("settings")} />
          <span className="rail-version">Novastrap {version && `v${version}`}</span>
        </div>
      </nav>
      <div className="main">
        <header className="topbar">
          <h1 className="topbar-title">{section.label}</h1>
          <div className="topbar-tabs" role="tablist">
            {section.tabs.map((t) => (
              <button key={t.id} role="tab" aria-selected={t.id === page} className={`top-tab ${t.id === page ? "active" : ""}`} onClick={() => go(t.id)}>
                {t.label}
              </button>
            ))}
          </div>
        </header>
        <main className={`content ${page === "home" ? "content-play" : ""}`} key={page}>{pages[page]}</main>
      </div>
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
