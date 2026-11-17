// Notification center: a bell in the top bar with the persisted notice log.
import { useEffect, useRef, useState } from "react";
import { backend, type Notice, type NoticeKind } from "../api/backend";
import { timeAgo } from "./kit";

const KIND_LABEL: Record<NoticeKind, string> = { update: "Update", game: "Game", launch: "Launch", error: "Problem", info: "Info" };

export function NoticeBell() {
  const [items, setItems] = useState<Notice[]>([]);
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement>(null);

  useEffect(() => {
    backend.noticesList().then((f) => setItems(f.items)).catch(() => undefined);
    const un = backend.onNotice((n) => setItems((list) => [n, ...list].slice(0, 200)));
    return () => { void un.then((f) => f()); };
  }, []);

  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => { if (box.current && !box.current.contains(e.target as Node)) setOpen(false); };
    const esc = (e: KeyboardEvent) => { if (e.key === "Escape") setOpen(false); };
    document.addEventListener("mousedown", close);
    document.addEventListener("keydown", esc);
    return () => { document.removeEventListener("mousedown", close); document.removeEventListener("keydown", esc); };
  }, [open]);

  const unread = items.filter((n) => !n.read).length;
  const markRead = async () => { await backend.noticesMarkRead().catch(() => undefined); setItems((l) => l.map((n) => ({ ...n, read: true }))); };
  const clear = async () => { await backend.noticesClear().catch(() => undefined); setItems([]); };

  return (
    <div className="bell-wrap" ref={box}>
      <button className="bell" aria-label={`Notifications${unread ? `, ${unread} unread` : ""}`} aria-expanded={open}
        onClick={() => { setOpen(!open); if (!open && unread) void markRead(); }}>
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9M10.3 21a1.94 1.94 0 0 0 3.4 0" />
        </svg>
        {unread > 0 && <span className="bell-count">{unread > 99 ? "99+" : unread}</span>}
      </button>
      {open && (
        <div className="bell-panel" role="dialog" aria-label="Notifications">
          <div className="bell-head">
            <b>Notifications</b>
            {items.length > 0 && <button className="link small" onClick={clear}>Clear all</button>}
          </div>
          {items.length === 0 ? <p className="muted small bell-empty">Nothing yet. Launches, games you join and Roblox updates show up here.</p> : (
            <ul className="bell-list">
              {items.map((n) => (
                <li key={n.id} className={`notice notice-${n.kind}`}>
                  <div className="row between">
                    <b>{n.title}</b>
                    <span className="muted small">{timeAgo(n.at)}</span>
                  </div>
                  {n.body && <p className="small">{n.body}</p>}
                  <span className="muted small">{KIND_LABEL[n.kind]}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}
