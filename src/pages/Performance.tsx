import { useEffect, useMemo, useState } from "react";
import { backend, type FlagSpec, type PerformanceProfile } from "../api/backend";
import { Card, Empty, ErrorBox, Field, Msg, Page, Skeletons, Tabs, fmtBytes, useAction, useAsync } from "../ui/kit";

function FlagInput({ spec, value, onChange }: { spec: FlagSpec; value: boolean | number | undefined; onChange: (v: boolean | number | undefined) => void }) {
  const k = spec.kind;
  if (k.type === "bool")
    return (
      <select className="input" value={value === undefined ? "" : String(value)} onChange={(e) => onChange(e.target.value === "" ? undefined : e.target.value === "true")}>
        <option value="">Roblox default</option><option value="true">On</option><option value="false">Off</option>
      </select>
    );
  if (k.type === "choice")
    return (
      <select className="input" value={value === undefined ? "" : String(value)} onChange={(e) => onChange(e.target.value === "" ? undefined : Number(e.target.value))}>
        <option value="">Roblox default</option>
        {k.options.map((o) => <option key={o} value={o}>{o}</option>)}
      </select>
    );
  return (
    <input className="input" type="number" min={k.min} max={k.max} placeholder={`Default (${k.min}–${k.max})`}
      value={value === undefined ? "" : String(value)}
      onChange={(e) => onChange(e.target.value === "" ? undefined : Math.max(k.min, Math.min(k.max, Math.round(Number(e.target.value)))))} />
  );
}

function download(name: string, text: string) {
  const a = document.createElement("a");
  a.href = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  a.download = name;
  a.click();
  URL.revokeObjectURL(a.href);
}

export function Performance() {
  const st = useAsync(() => backend.performanceState());
  const diag = useAsync(() => backend.diagnostics());
  const action = useAction();
  const [tab, setTab] = useState<"profiles" | "roblox" | "diagnostics">("profiles");
  const [edit, setEdit] = useState<{ p: PerformanceProfile; original: string | null } | null>(null);
  const [fps, setFps] = useState<number | null>(null);
  const [quality, setQuality] = useState<number | null>(null);

  useEffect(() => {
    if (st.data) { setFps(st.data.roblox.framerateCap); setQuality(st.data.roblox.graphicsQuality); }
  }, [st.data]);

  const groups = useMemo(() => {
    const m = new Map<string, FlagSpec[]>();
    st.data?.allowlist.forEach((f) => m.set(f.group, [...(m.get(f.group) ?? []), f]));
    return [...m.entries()];
  }, [st.data]);

  const saveEdit = async () => {
    if (!edit) return;
    const r = await action.run(() => backend.savePerformanceProfile(edit.p, edit.original), (x) => {
      const d = (x as { dropped: string[] }).dropped;
      return d.length ? `Saved. Removed flags Roblox would ignore: ${d.join(", ")}` : "Saved";
    });
    if (r) { setEdit(null); await st.reload(); }
  };

  const importProfile = async (file: File) => {
    try {
      const raw = JSON.parse(await file.text());
      // Accepts a Novastrap profile or a plain Bloxstrap-style flags object.
      const p: PerformanceProfile = raw && typeof raw === "object" && "flags" in raw
        ? { name: String(raw.name ?? file.name.replace(/\.json$/i, "")), flags: raw.flags, framerateCap: raw.framerateCap ?? null, graphicsQuality: raw.graphicsQuality ?? null }
        : { name: file.name.replace(/\.json$/i, "").slice(0, 40), flags: raw, framerateCap: null, graphicsQuality: null };
      setEdit({ p, original: null });
    } catch { action.setMsg({ ok: false, text: "That file isn't valid JSON" }); }
  };

  const all = [...(st.data?.presets ?? []).map((p) => ({ p, preset: true })), ...(st.data?.profiles ?? []).map((p) => ({ p, preset: false }))];

  return (
    <Page title="Performance" subtitle="Roblox's graphics settings and the FastFlags Roblox still allows. Results depend on your PC; nothing here promises more FPS.">
      <Tabs tabs={[{ id: "profiles", label: "Profiles & FastFlags" }, { id: "roblox", label: "Frame rate & graphics" }, { id: "diagnostics", label: "Diagnostics" }]} value={tab} onChange={setTab} />
      <Msg msg={action.msg} />
      {st.error && <ErrorBox error={st.error} onRetry={st.reload} />}
      {st.loading && !st.data && <Skeletons n={4} />}

      {st.data && tab === "profiles" && !edit && (
        <Card title="Profiles" right={
          <div className="row">
            <label className="btn btn-sm">Import<input type="file" accept=".json" hidden onChange={(e) => e.target.files?.[0] && importProfile(e.target.files[0])} /></label>
            <button className="btn btn-sm btn-primary" onClick={() => setEdit({ p: { name: "", flags: {}, framerateCap: null, graphicsQuality: null }, original: null })}>New profile</button>
          </div>}>
          <p className="muted small">The active profile is written before every launch. Only flags on Roblox's allowlist are written; Roblox ignores all others.</p>
          <ul className="list">
            {all.map(({ p, preset }) => {
              const active = st.data!.active === p.name;
              return (
                <li key={p.name} className={active ? "active" : ""}>
                  <div className="grow">
                    <b>{p.name}</b> {preset && <span className="badge badge-muted">Preset</span>}
                    <div className="muted small">
                      {Object.keys(p.flags).length} flags{p.framerateCap ? ` · ${p.framerateCap} FPS cap` : ""}{p.graphicsQuality ? ` · graphics ${p.graphicsQuality}` : ""}
                    </div>
                  </div>
                  <button className="btn btn-sm" onClick={() => download(`${p.name}.json`, JSON.stringify(p, null, 2))}>Export</button>
                  {preset ? (
                    <button className="btn btn-sm" onClick={() => setEdit({ p: { ...p, name: `${p.name} (copy)` }, original: null })}>Duplicate</button>
                  ) : (
                    <>
                      <button className="btn btn-sm" onClick={() => setEdit({ p, original: p.name })}>Edit</button>
                      <button className="btn btn-sm danger" onClick={() => confirm(`Delete ${p.name}?`) && action.run(async () => { await backend.deletePerformanceProfile(p.name); await st.reload(); }, "Deleted")}>Delete</button>
                    </>
                  )}
                  {active ? (
                    <button className="btn btn-sm" onClick={() => action.run(async () => { await backend.setActivePerformanceProfile(null); await st.reload(); }, "Profile turned off; Roblox defaults restored")}>Deactivate</button>
                  ) : (
                    <button className="btn btn-sm btn-primary" onClick={() => action.run(async () => { const w = await backend.setActivePerformanceProfile(p.name); await st.reload(); return w; }, (w) => ((w as string[]).length ? `Active. ${(w as string[]).join(" ")}` : `${p.name} is active`))}>Use</button>
                  )}
                </li>
              );
            })}
          </ul>
        </Card>
      )}

      {st.data && tab === "profiles" && edit && (
        <Card title={edit.original ? `Edit ${edit.original}` : "New profile"}>
          <div className="form-grid">
            <Field label="Name"><input className="input" maxLength={40} value={edit.p.name} onChange={(e) => setEdit({ ...edit, p: { ...edit.p, name: e.target.value } })} /></Field>
            <Field label="Frame rate cap" hint="Roblox's own setting; Roblox doesn't support more than 240.">
              <select className="input" value={edit.p.framerateCap ?? ""} onChange={(e) => setEdit({ ...edit, p: { ...edit.p, framerateCap: e.target.value ? Number(e.target.value) : null } })}>
                <option value="">Don't change</option>
                {st.data.framerateCaps.map((f) => <option key={f} value={f}>{f}</option>)}
              </select>
            </Field>
            <Field label="Graphics quality" hint="1–21; Roblox's in-game slider uses this scale.">
              <input className="input" type="number" min={1} max={21} placeholder="Don't change" value={edit.p.graphicsQuality ?? ""}
                onChange={(e) => setEdit({ ...edit, p: { ...edit.p, graphicsQuality: e.target.value ? Math.max(1, Math.min(21, Number(e.target.value))) : null } })} />
            </Field>
          </div>
          {groups.map(([group, specs]) => (
            <fieldset key={group} className="flag-group">
              <legend>{group}</legend>
              {specs.map((s) => (
                <div key={s.name} className="flag-row">
                  <div className="grow"><code className="small">{s.name}</code><div className="muted small">{s.description}</div></div>
                  <FlagInput spec={s} value={edit.p.flags[s.name]} onChange={(v) => {
                    const flags = { ...edit.p.flags };
                    if (v === undefined) delete flags[s.name]; else flags[s.name] = v;
                    setEdit({ ...edit, p: { ...edit.p, flags } });
                  }} />
                </div>
              ))}
            </fieldset>
          ))}
          <div className="row">
            <button className="btn btn-primary" onClick={saveEdit} disabled={action.busy || !edit.p.name.trim()}>Save profile</button>
            <button className="btn" onClick={() => setEdit(null)}>Cancel</button>
          </div>
        </Card>
      )}

      {st.data && tab === "roblox" && (
        <Card title="Frame rate & graphics">
          <p className="muted small">These are the same values Roblox's in-game settings menu saves. Close Roblox before applying: it overwrites them when it exits.</p>
          <div className="form-grid">
            <Field label="Maximum frame rate">
              <select className="input" value={fps ?? ""} onChange={(e) => setFps(e.target.value ? Number(e.target.value) : null)}>
                <option value="">Unchanged ({st.data.roblox.framerateCap ?? "unknown"})</option>
                {st.data.framerateCaps.map((f) => <option key={f} value={f}>{f} FPS</option>)}
              </select>
            </Field>
            <Field label={`Graphics quality: ${quality ?? "unknown"}`}>
              <input type="range" min={1} max={21} value={quality ?? 10} onChange={(e) => setQuality(Number(e.target.value))} />
            </Field>
          </div>
          <div className="row">
            <button className="btn btn-primary" disabled={action.busy || !st.data.roblox.path}
              onClick={() => action.run(async () => { await backend.applyRobloxSettings(fps, quality); await st.reload(); }, "Applied to Roblox's settings")}>Apply</button>
            {!st.data.roblox.path && <span className="muted small">Start Roblox once so it creates its settings file.</span>}
          </div>
        </Card>
      )}

      {tab === "diagnostics" && (
        <Card title="Launch diagnostics" right={<button className="btn btn-sm" onClick={diag.reload} disabled={diag.loading}>Re-run</button>}>
          {diag.loading && !diag.data && <Skeletons n={4} />}
          {diag.error && <ErrorBox error={diag.error} onRetry={diag.reload} />}
          {diag.data && (
            <>
              <dl className="facts">
                <dt>Install</dt><dd>{diag.data.status.installations.length ? `${diag.data.status.installations.length} version folder(s)` : "Not installed"}</dd>
                <dt>Update state</dt><dd>{diag.data.status.update.state}</dd>
                <dt>Link handler</dt><dd>{diag.data.status.handler.kind}</dd>
                <dt>FastFlags file</dt><dd className="mono path">{diag.data.flagsFile ?? "—"} ({diag.data.flagsWritten} flags)</dd>
                <dt>Frame rate cap</dt><dd>{diag.data.robloxSettings.framerateCap ?? "—"}</dd>
                <dt>Graphics quality</dt><dd>{diag.data.robloxSettings.graphicsQuality ?? "—"}</dd>
                <dt>Running clients</dt><dd>{diag.data.clients.length ? diag.data.clients.map((c) => `${c.pid} (${fmtBytes(c.memoryBytes)})`).join(", ") : "None"}</dd>
                <dt>Mods</dt><dd>{diag.data.mods.length ? diag.data.mods.map((m) => `${m.slotId}: ${m.ok ? "OK" : m.reason}`).join(" · ") : "None enabled"}</dd>
              </dl>
              <h3>Recent client log errors</h3>
              {diag.data.recentErrors.length ? <pre className="log">{diag.data.recentErrors.join("\n")}</pre> : <Empty>No errors in the latest Roblox log.</Empty>}
            </>
          )}
        </Card>
      )}
    </Page>
  );
}
