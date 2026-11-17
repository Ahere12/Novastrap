import { useEffect, useState } from "react";
import { backend, type Settings as S } from "../api/backend";
import { useApp } from "../app/context";
import { Card, Empty, ErrorBox, Field, Msg, Page, Skeletons, Tabs, Toggle, fmtBytes, useAction, useAsync } from "../ui/kit";

type Tab = "general" | "accounts" | "integrations" | "notifications" | "privacy" | "storage" | "logs" | "about";
const TABS: { id: Tab; label: string }[] = [
  { id: "general", label: "General" }, { id: "accounts", label: "Accounts" }, { id: "integrations", label: "Integrations" },
  { id: "notifications", label: "Notifications" }, { id: "privacy", label: "Privacy" }, { id: "storage", label: "Storage & backups" },
  { id: "logs", label: "Logs" }, { id: "about", label: "About & updates" },
];

function Storage() {
  const usage = useAsync(() => backend.storageUsage());
  const action = useAction();
  return (
    <>
      <Card title="Storage" right={<button className="btn btn-sm" onClick={usage.reload} disabled={usage.loading}>Recalculate</button>}>
        <Msg msg={action.msg} />
        {usage.loading && !usage.data && <Skeletons n={4} />}
        {usage.error && <ErrorBox error={usage.error} onRetry={usage.reload} />}
        <ul className="list">
          {usage.data?.map((s) => (
            <li key={s.id}>
              <span className="grow">{s.label}</span>
              <span className="muted">{fmtBytes(s.bytes)}</span>
              {s.clearable && (
                <button className="btn btn-sm danger" disabled={action.busy} onClick={() => {
                  const warn = s.id === "oldVersions" ? "Delete older Roblox version folders? The newest install is kept." : "Clear Novastrap's downloads and web cache?";
                  if (confirm(warn)) void action.run(async () => { const n = await backend.storageClear(s.id); await usage.reload(); return n; }, (n) => `Freed ${fmtBytes(n as number)}`);
                }}>Clear</button>
              )}
            </li>
          ))}
        </ul>
      </Card>
      <Card title="Configuration backups">
        <p className="muted small">Exports settings, accounts (profiles only), instances, favorites, performance profiles and mod choices to a JSON file. Sign-in tokens are never exported.</p>
        <div className="row">
          <button className="btn" onClick={() => action.run(() => backend.backupExport(), (p) => (p ? `Saved to ${p}` : "Cancelled"))}>Export backup</button>
          <button className="btn" onClick={() => confirm("Importing replaces your current settings. Continue?") && action.run(async () => { const ok = await backend.backupImport(); if (ok) location.reload(); return ok; }, (ok) => (ok ? "Imported" : "Cancelled"))}>Import backup</button>
        </div>
      </Card>
    </>
  );
}

function Logs() {
  const lines = useAsync(() => backend.appLogTail());
  return (
    <Card title="Novastrap log" right={
      <div className="row">
        <button className="btn btn-sm" onClick={lines.reload}>Refresh</button>
        <button className="btn btn-sm" onClick={() => backend.openFolder("logs")}>Open folder</button>
        <button className="btn btn-sm" onClick={() => backend.openFolder("robloxLogs")}>Roblox logs</button>
      </div>}>
      <p className="muted small">Logs never contain passwords or tokens.</p>
      {lines.data && !lines.data.length && <Empty>The log is empty.</Empty>}
      {lines.data && lines.data.length > 0 && <pre className="log tall">{lines.data.join("\n")}</pre>}
    </Card>
  );
}

function About() {
  const info = useAsync(() => backend.appInfo());
  const upd = useAsync(() => backend.checkAppUpdate());
  return (
    <Card title="About Novastrap">
      <p>Version {info.data?.version ?? "…"}. An open-source (MIT) launcher for the Roblox desktop client. Not affiliated with or endorsed by Roblox Corporation.</p>
      {upd.loading && <p className="muted">Checking GitHub for a newer release…</p>}
      {upd.error && <ErrorBox error={upd.error} onRetry={upd.reload} />}
      {upd.data && (
        upd.data.updateAvailable ? (
          <p className="ok">Version {upd.data.latest} is available. <button className="link" onClick={() => backend.openExternal(upd.data!.url!)}>Open release page</button></p>
        ) : <p className="muted">{upd.data.latest ? `You're on the latest release (${upd.data.latest}).` : "No releases have been published yet."}</p>
      )}
      <div className="row">
        <button className="btn" onClick={() => backend.openExternal("https://github.com/Ahere12/Novastrap")}>Source code</button>
        <button className="btn" onClick={upd.reload}>Check again</button>
      </div>
    </Card>
  );
}

export function Settings() {
  const { settings, saveSettings, params } = useApp();
  const [tab, setTab] = useState<Tab>((params.tab as Tab) ?? "general");
  const [draft, setDraft] = useState<S | null>(settings);
  const action = useAction();
  useEffect(() => setDraft(settings), [settings]);
  if (!draft) return <Skeletons n={5} />;

  const set = <K extends keyof S>(k: K, patch: Partial<S[K]>) => setDraft({ ...draft, [k]: { ...draft[k], ...patch } });
  const instant = async <K extends keyof S>(k: K, patch: Partial<S[K]>) => {
    const next = { ...draft, [k]: { ...draft[k], ...patch } };
    setDraft(next);
    await saveSettings(next);
  };
  const saveText = () => action.run(async () => { if (!(await saveSettings(draft))) throw new Error("Not saved"); }, "Saved");

  return (
    <Page title="Settings">
      <Tabs tabs={TABS} value={tab} onChange={setTab} />
      <Msg msg={action.msg} />
      {tab === "general" && (
        <Card title="General">
          <Toggle label="Track game activity" hint="Reads Roblox's own log files to show recently played, playtime, the HUD and Discord presence. Stays on this PC."
            checked={draft.general.trackActivity} onChange={(v) => instant("general", { trackActivity: v })} />
          <Toggle label="Minimize Novastrap when Roblox starts" checked={draft.general.minimizeOnLaunch} onChange={(v) => instant("general", { minimizeOnLaunch: v })} />
          <p className="muted small">Theme, background and fonts are under Customize.</p>
        </Card>
      )}
      {tab === "accounts" && (
        <Card title="Roblox sign-in">
          <Field label="OAuth client ID" hint={<>From an OAuth app at create.roblox.com → Credentials. Redirect URI: <code>http://localhost:47821/oauth/callback</code>, scopes <code>openid profile</code>.</>}>
            <input className="input" inputMode="numeric" value={draft.accounts.oauthClientId} onChange={(e) => set("accounts", { oauthClientId: e.target.value.trim() })} placeholder="e.g. 1234567890123456789" />
          </Field>
          <div className="row"><button className="btn btn-primary" onClick={saveText}>Save</button></div>
        </Card>
      )}
      {tab === "integrations" && (
        <Card title="Discord Rich Presence">
          <Toggle label="Show what I'm playing on Discord" hint="Needs Discord running and activity tracking on." checked={draft.integrations.discordEnabled} onChange={(v) => instant("integrations", { discordEnabled: v })} />
          <Field label="Discord application ID" hint="Create an application at discord.com/developers; its name is what Discord shows as the activity title.">
            <input className="input" inputMode="numeric" value={draft.integrations.discordAppId} onChange={(e) => set("integrations", { discordAppId: e.target.value.trim() })} />
          </Field>
          <Field label="Custom status line" hint="Optional, up to 120 characters. Replaces the account line.">
            <input className="input" maxLength={120} value={draft.integrations.discordCustomState} onChange={(e) => set("integrations", { discordCustomState: e.target.value })} />
          </Field>
          <h3>Privacy</h3>
          <Toggle label="Show the game name and icon" checked={draft.integrations.discordShowGame} onChange={(v) => instant("integrations", { discordShowGame: v })} />
          <Toggle label="Show a “Join server” button" hint="Lets anyone who sees your profile join your exact server." checked={draft.integrations.discordShowJoin} onChange={(v) => instant("integrations", { discordShowJoin: v })} />
          <Toggle label="Show my active account name" checked={draft.integrations.discordShowAccount} onChange={(v) => instant("integrations", { discordShowAccount: v })} />
          <div className="row"><button className="btn btn-primary" onClick={saveText}>Save</button></div>
        </Card>
      )}
      {tab === "notifications" && (
        <Card title="Notifications">
          <Toggle label="When I join a game" checked={draft.notifications.gameJoined} onChange={(v) => instant("notifications", { gameJoined: v })} />
          <Toggle label="When a Roblox update is available" hint="Checked when Novastrap opens." checked={draft.notifications.updateAvailable} onChange={(v) => instant("notifications", { updateAvailable: v })} />
        </Card>
      )}
      {tab === "privacy" && (
        <Card title="Privacy">
          <ul className="plain">
            <li>Novastrap never asks for your Roblox password or cookies. Sign-in uses Roblox's official OAuth in your browser.</li>
            <li>Sign-in tokens live in Windows Credential Manager only.</li>
            <li>Activity history stays on this PC; server IP addresses are never saved.</li>
            <li>Network requests go to Roblox, GitHub (update check), and Rolimons (only when you open Trading).</li>
          </ul>
          <button className="btn danger" onClick={() => confirm("Delete all play history?") && action.run(() => backend.clearHistory(), "History cleared")}>Clear play history</button>
        </Card>
      )}
      {tab === "storage" && <Storage />}
      {tab === "logs" && <Logs />}
      {tab === "about" && <About />}
    </Page>
  );
}
