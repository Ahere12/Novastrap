import { useState, type FormEvent } from "react";
import { backend, type SessionStatus } from "../api/backend";
import { useApp } from "../app/context";
import { Card, Empty, ErrorBox, Img, Msg, Page, Skeletons, timeAgo, useAction, useAsync } from "../ui/kit";

const STATUS: Record<SessionStatus, [string, string]> = {
  signedIn: ["Signed in", "badge-ok"],
  expired: ["Session expired", "badge-warn"],
  profileOnly: ["Profile only", "badge-muted"],
};

export function Accounts() {
  const { go } = useApp();
  const list = useAsync(() => backend.accountsList());
  const action = useAction();
  const [username, setUsername] = useState("");

  const add = async (e: FormEvent) => {
    e.preventDefault();
    const r = await action.run(() => backend.accountAddProfile(username), (a) => `Saved ${(a as { displayName: string }).displayName}`);
    if (r) { setUsername(""); await list.reload(); }
  };
  const signIn = async () => {
    const r = await action.run(() => backend.accountSignIn(), (a) => `Signed in as ${(a as { displayName: string }).displayName}`);
    if (r) await list.reload();
  };

  return (
    <Page title="Accounts" subtitle="Your Roblox profiles. Novastrap never asks for passwords or cookies.">
      <div className="grid-2">
        <Card title="Sign in with Roblox">
          <p className="muted">Opens roblox.com in your browser using Roblox's official OAuth sign-in. Tokens are stored in Windows Credential Manager, never in files.</p>
          {list.data && !list.data.oauthConfigured ? (
            <>
              <p className="muted small">
                Sign-in needs an OAuth app from the Roblox Creator Dashboard (free). Register it with the redirect URI{" "}
                <code>{list.data.redirectUri}</code> and the <code>openid profile</code> scopes, then paste its client ID in Settings.
              </p>
              <div className="row">
                <button className="btn" onClick={() => backend.openExternal("https://create.roblox.com/dashboard/credentials?activeTab=OAuthTab")}>Open Creator Dashboard</button>
                <button className="btn" onClick={() => go("settings", { tab: "accounts" })}>Add client ID</button>
              </div>
            </>
          ) : (
            <button className="btn btn-primary" onClick={signIn} disabled={action.busy || !list.data}>{action.busy ? "Waiting for the browser…" : "Sign in with Roblox"}</button>
          )}
        </Card>
        <Card title="Add a profile">
          <p className="muted">Save any public Roblox profile by username to show its avatar and details. No sign-in involved.</p>
          <form className="join" onSubmit={add}>
            <input className="input" value={username} onChange={(e) => setUsername(e.target.value)} placeholder="Roblox username" aria-label="Roblox username" />
            <button className="btn" type="submit" disabled={action.busy || !username.trim()}>Add</button>
          </form>
        </Card>
      </div>
      <Msg msg={action.msg} />

      <Card title="Saved accounts">
        {list.loading && !list.data && <Skeletons n={3} />}
        {list.error && <ErrorBox error={list.error} onRetry={list.reload} />}
        {list.data && !list.data.accounts.length && <Empty>No accounts yet.</Empty>}
        <ul className="account-list">
          {list.data?.accounts.map((a) => (
            <li key={a.userId} className={a.active ? "active" : ""}>
              <Img src={a.avatarUrl} alt="" className="avatar" />
              <div className="grow">
                <b>{a.displayName}</b> <span className="muted">@{a.username}</span>
                <div className="muted small">ID {a.userId}{a.lastUsedAt ? ` · used ${timeAgo(a.lastUsedAt)}` : ""}</div>
              </div>
              <span className={`badge ${STATUS[a.status][1]}`}>{STATUS[a.status][0]}</span>
              {a.active ? <span className="badge badge-info">Active</span> : (
                <button className="btn btn-sm" onClick={() => action.run(async () => { await backend.accountSwitch(a.userId); await list.reload(); }, `Switched to ${a.displayName}`)}>Switch</button>
              )}
              <button className="btn btn-sm" onClick={() => backend.openExternal(`https://www.roblox.com/users/${a.userId}/profile`)}>Profile</button>
              <button className="btn btn-sm danger" onClick={() => {
                if (confirm(`Remove ${a.displayName}${a.linked ? " and sign it out" : ""}?`)) void action.run(async () => { await backend.accountRemove(a.userId); await list.reload(); }, "Removed");
              }}>Remove</button>
            </li>
          ))}
        </ul>
        <p className="muted small">Switching changes the account Novastrap shows and uses for its features. The Roblox client keeps its own login; switch it in Roblox itself.</p>
      </Card>
    </Page>
  );
}
