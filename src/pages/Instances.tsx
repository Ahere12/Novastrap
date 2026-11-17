import { useEffect, useState } from "react";
import { backend, type InstanceProfile } from "../api/backend";
import { useApp } from "../app/context";
import { Card, Empty, ErrorBox, Field, Msg, Page, Skeletons, fmtBytes, fmtDuration, useAction, useAsync } from "../ui/kit";

const blank: InstanceProfile = { id: "", name: "", target: "", performanceProfile: null };

export function Instances() {
  const { toast } = useApp();
  const profiles = useAsync(() => backend.instanceProfiles());
  const perf = useAsync(() => backend.performanceState());
  const clients = useAsync(() => backend.runningClients());
  const action = useAction();
  const [edit, setEdit] = useState<InstanceProfile | null>(null);

  useEffect(() => {
    const t = setInterval(() => void clients.reload(), 3000);
    return () => clearInterval(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const perfNames = [...(perf.data?.presets ?? []), ...(perf.data?.profiles ?? [])].map((p) => p.name);
  const save = async () => {
    if (!edit) return;
    const r = await action.run(() => backend.saveInstanceProfile(edit), "Saved");
    if (r) { profiles.setData(r as InstanceProfile[]); setEdit(null); }
  };
  const launch = async (p: InstanceProfile) => {
    const r = await action.run(() => backend.launchInstance(p.id), `Started ${p.name}`);
    if (r) (r as { warnings: string[] }).warnings.forEach((w) => toast(false, w));
    setTimeout(() => void clients.reload(), 1500);
  };

  return (
    <Page title="Instances" subtitle="Saved launch setups and the Roblox clients running right now."
      actions={<button className="btn btn-primary" onClick={() => setEdit({ ...blank })}>New instance</button>}>
      <Msg msg={action.msg} />
      {edit && (
        <Card title={edit.id ? "Edit instance" : "New instance"}>
          <div className="form-grid">
            <Field label="Name"><input className="input" value={edit.name} maxLength={40} onChange={(e) => setEdit({ ...edit, name: e.target.value })} placeholder="e.g. Main, Building, Trading" /></Field>
            <Field label="Launch into" hint="Place ID or roblox.com link. Leave empty for the Roblox home screen.">
              <input className="input" value={edit.target} onChange={(e) => setEdit({ ...edit, target: e.target.value })} placeholder="Optional" />
            </Field>
            <Field label="Performance profile" hint="Applied before this instance starts.">
              <select className="input" value={edit.performanceProfile ?? ""} onChange={(e) => setEdit({ ...edit, performanceProfile: e.target.value || null })}>
                <option value="">Use the active profile</option>
                {perfNames.map((n) => <option key={n}>{n}</option>)}
              </select>
            </Field>
          </div>
          <div className="row">
            <button className="btn btn-primary" onClick={save} disabled={action.busy || !edit.name.trim()}>Save</button>
            <button className="btn" onClick={() => setEdit(null)}>Cancel</button>
          </div>
        </Card>
      )}

      <Card title="Saved instances">
        {profiles.loading && !profiles.data && <Skeletons n={2} />}
        {profiles.error && <ErrorBox error={profiles.error} onRetry={profiles.reload} />}
        {profiles.data && !profiles.data.length && <Empty>Create an instance to launch Roblox straight into a game with its own performance profile.</Empty>}
        <ul className="list">
          {profiles.data?.map((p) => (
            <li key={p.id}>
              <div className="grow">
                <b>{p.name}</b>
                <div className="muted small">{p.target ? `Joins ${p.target}` : "Roblox home screen"} · {p.performanceProfile ?? "active performance profile"}</div>
              </div>
              <button className="btn btn-sm" onClick={() => setEdit(p)}>Edit</button>
              <button className="btn btn-sm danger" onClick={() => confirm(`Delete ${p.name}?`) && action.run(async () => profiles.setData(await backend.deleteInstanceProfile(p.id)), "Deleted")}>Delete</button>
              <button className="btn btn-sm btn-primary" onClick={() => launch(p)} disabled={action.busy}>Launch</button>
            </li>
          ))}
        </ul>
      </Card>

      <Card title="Running clients" right={<span className="muted small">Updates every 3 s</span>}>
        {clients.error && <ErrorBox error={clients.error} onRetry={clients.reload} />}
        {clients.data && !clients.data.length && <Empty>No Roblox clients are running.</Empty>}
        {clients.data && clients.data.length > 0 && (
          <table className="table">
            <thead><tr><th>Instance</th><th>Process</th><th>Memory</th><th>Running for</th><th /></tr></thead>
            <tbody>
              {clients.data.map((c) => (
                <tr key={c.pid}>
                  <td>{c.instance?.name ?? <span className="muted">Started outside Novastrap</span>}</td>
                  <td className="mono small">{c.pid}</td>
                  <td>{fmtBytes(c.memoryBytes)}</td>
                  <td>{fmtDuration(c.runTimeSecs)}</td>
                  <td className="actions">
                    <button className="btn btn-sm danger" onClick={() => confirm("Close this Roblox client? Unsaved progress in the game may be lost.") && action.run(async () => { await backend.closeClient(c.pid); await clients.reload(); }, "Closed")}>Close</button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        <p className="muted small">
          Running several clients at once isn't available: on the current Roblox build, a second client hands off to the first one.
          Separate accounts per client would also need Roblox session cookies, which Novastrap doesn't handle.
        </p>
      </Card>
    </Page>
  );
}
