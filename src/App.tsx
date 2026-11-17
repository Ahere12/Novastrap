import { useEffect, useState } from "react";
import { backend } from "./api/backend";
import { Home } from "./pages/Home";
import { Logo } from "./ui/Logo";
import "./App.css";

type Section = {
  id: string;
  label: string;
  icon: string; // SVG path (24×24)
  phase?: number; // set while the section isn't built yet
};

const SECTIONS: Section[] = [
  { id: "home", label: "Home", icon: "M3 11 12 3l9 8v9a1 1 0 0 1-1 1h-5v-6h-6v6H4a1 1 0 0 1-1-1z" },
  { id: "discover", label: "Discover", phase: 5, icon: "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20m4 6-2.5 5.5L8 16l2.5-5.5z" },
  { id: "servers", label: "Servers", phase: 5, icon: "M4 4h16v6H4zm0 10h16v6H4zm3-7h.01M7 17h.01" },
  { id: "accounts", label: "Accounts", phase: 6, icon: "M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8m-8 9a8 8 0 0 1 16 0" },
  { id: "client", label: "Client", phase: 6, icon: "M4 6h10M18 6h2M4 12h4M12 12h8M4 18h12M20 18h0M16 4v4M10 10v4M18 16v4" },
  { id: "mods", label: "Customize", phase: 7, icon: "M12 3a9 9 0 1 0 0 18c1 0 1.5-.8 1.5-1.5 0-1.2-1-1.5-1-2.5s.8-1.5 2-1.5H17a4 4 0 0 0 4-4c0-4.4-4-8.5-9-8.5M7.5 11h.01M10 7h.01M15 7h.01" },
  { id: "settings", label: "Settings", phase: 6, icon: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6m7.4-3a7.4 7.4 0 0 0-.1-1.2l2-1.6-2-3.4-2.4 1a7 7 0 0 0-2-1.2L14.5 3h-5l-.4 2.6a7 7 0 0 0-2 1.2l-2.4-1-2 3.4 2 1.6a7.4 7.4 0 0 0 0 2.4l-2 1.6 2 3.4 2.4-1a7 7 0 0 0 2 1.2l.4 2.6h5l.4-2.6a7 7 0 0 0 2-1.2l2.4 1 2-3.4-2-1.6c.1-.4.1-.8.1-1.2" },
];

function Icon({ d }: { d: string }) {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor"
      strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={d} />
    </svg>
  );
}

function Planned({ section }: { section: Section }) {
  return (
    <div className="page">
      <header className="page-header">
        <h1>{section.label}</h1>
      </header>
      <section className="card planned">
        <p>This section is planned for <strong>phase {section.phase}</strong> of the roadmap.</p>
        <p className="muted">It stays empty until it works for real, so there are no placeholder controls here.</p>
      </section>
    </div>
  );
}

export default function App() {
  const [active, setActive] = useState("home");
  const [version, setVersion] = useState("");

  useEffect(() => {
    backend.appInfo().then((i) => setVersion(i.version)).catch(() => setVersion(""));
  }, []);

  const section = SECTIONS.find((s) => s.id === active)!;

  return (
    <div className="shell">
      <nav className="sidebar" aria-label="Main">
        <div className="brand">
          <Logo />
          <span>Novastrap</span>
        </div>
        <ul>
          {SECTIONS.map((s) => (
            <li key={s.id}>
              <button
                className={`nav-item ${s.id === active ? "active" : ""}`}
                aria-current={s.id === active ? "page" : undefined}
                onClick={() => setActive(s.id)}
              >
                <Icon d={s.icon} />
                <span>{s.label}</span>
                {s.phase && <span className="soon">soon</span>}
              </button>
            </li>
          ))}
        </ul>
        <div className="sidebar-foot muted">{version && `v${version}`}</div>
      </nav>
      <main className="content" key={active}>
        {section.phase ? <Planned section={section} /> : <Home />}
      </main>
    </div>
  );
}
