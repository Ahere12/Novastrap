import { backend } from "../api/backend";
import { useApp } from "../app/context";
import { Card, ErrorBox, Field, Msg, Page, Skeletons, Toggle, useAction, useAsync } from "../ui/kit";

export const ACCENTS = ["#8b7bff", "#3dd6f5", "#3ecf8e", "#f5b84a", "#ff6b7a", "#ff7ad9", "#e8e9f1"];
export const FONTS: { id: string; label: string }[] = [
  { id: "system", label: "Theme default" },
  { id: "verdana", label: "Verdana" },
  { id: "segoe", label: "Segoe UI" },
  { id: "bahnschrift", label: "Bahnschrift" },
  { id: "consolas", label: "Consolas" },
  { id: "georgia", label: "Georgia" },
];

export const THEMES: { id: string; label: string; hint: string }[] = [
  { id: "legacy", label: "Legacy 2019", hint: "Flat light style of the 2017–2019 Roblox website: blue header, white cards, green buttons" },
  { id: "classic", label: "Classic", hint: "Light, early-web look: blue header, glossy buttons, boxed panels" },
  { id: "launcher", label: "Launcher", hint: "Dark game-launcher look" },
];

export function Customize() {
  const { settings, saveSettings } = useApp();
  const slots = useAsync(() => backend.modSlots());
  const action = useAction();
  if (!settings) return <Skeletons n={4} />;
  const ap = settings.appearance;
  const setAp = (patch: Partial<typeof ap>) => saveSettings({ ...settings, appearance: { ...ap, ...patch } });
  const refreshBg = () => window.dispatchEvent(new Event("novastrap-background"));

  const exportTheme = () => {
    const a = document.createElement("a");
    const { background: _bg, ...theme } = ap;
    a.href = URL.createObjectURL(new Blob([JSON.stringify({ format: "novastrap-theme-1", ...theme }, null, 2)], { type: "application/json" }));
    a.download = "novastrap-theme.json";
    a.click();
  };
  const importTheme = async (f: File) => {
    try {
      const t = JSON.parse(await f.text());
      if (t.format !== "novastrap-theme-1") throw new Error();
      await setAp({ theme: t.theme === "launcher" || t.theme === "classic" ? t.theme : "legacy", accent: t.accent, font: t.font, backgroundDim: t.backgroundDim, reduceMotion: !!t.reduceMotion, uiSounds: !!t.uiSounds });
      action.setMsg({ ok: true, text: "Theme imported" });
    } catch { action.setMsg({ ok: false, text: "That isn't a Novastrap theme file" }); }
  };

  return (
    <Page title="Customize" subtitle="Launcher appearance and supported Roblox client file mods. Every change can be undone.">
      <Msg msg={action.msg} />
      <div className="grid-2">
        <Card title="Theme" right={
          <div className="row">
            <button className="btn btn-sm" onClick={exportTheme}>Export</button>
            <label className="btn btn-sm">Import<input type="file" accept=".json" hidden onChange={(e) => e.target.files?.[0] && importTheme(e.target.files[0])} /></label>
          </div>}>
          <Field label="Style">
            <div className="theme-picks">
              {THEMES.map((t) => {
                const on = (ap.theme === "launcher" || ap.theme === "classic" ? ap.theme : "legacy") === t.id;
                return (
                  <button key={t.id} className={`theme-pick ${t.id} ${on ? "on" : ""}`} aria-pressed={on} onClick={() => setAp({ theme: t.id })}>
                    <span className="theme-swatch" aria-hidden="true"><i /><i /><i /></span>
                    <b>{t.label}</b>
                    <span className="small muted">{t.hint}</span>
                  </button>
                );
              })}
            </div>
          </Field>
          <Field label="Accent color">
            <div className="swatches">
              {ACCENTS.map((c) => (
                <button key={c} className={`swatch ${ap.accent === c ? "on" : ""}`} style={{ background: c }} onClick={() => setAp({ accent: c })} aria-label={`Accent ${c}`} />
              ))}
              <input type="color" value={ap.accent} onChange={(e) => setAp({ accent: e.target.value })} aria-label="Custom accent" />
            </div>
          </Field>
          <Field label="Font">
            <select className="input" value={ap.font} onChange={(e) => setAp({ font: e.target.value })}>
              {FONTS.map((f) => <option key={f.id} value={f.id}>{f.label}</option>)}
            </select>
          </Field>
          <Toggle label="Reduce motion" hint="Turns off page and hover animations." checked={ap.reduceMotion} onChange={(v) => setAp({ reduceMotion: v })} />
          <Toggle label="Interface sounds" hint="A soft click when you navigate or launch." checked={ap.uiSounds} onChange={(v) => setAp({ uiSounds: v })} />
        </Card>
        <Card title="Background">
          <p className="muted small">A PNG, JPEG or WebP image under 8 MB, stored in Novastrap's own folder.</p>
          <div className="row">
            <button className="btn" onClick={() => action.run(async () => { const r = await backend.backgroundChoose(); if (r) { await saveSettings({ ...settings, appearance: { ...ap, background: r } }); refreshBg(); } }, undefined)}>Choose image</button>
            {ap.background && <button className="btn" onClick={() => action.run(async () => { await backend.backgroundClear(); await saveSettings({ ...settings, appearance: { ...ap, background: null } }); refreshBg(); })}>Remove</button>}
          </div>
          <Field label={`Dim background: ${ap.backgroundDim}%`}>
            <input type="range" min={0} max={95} value={ap.backgroundDim} onChange={(e) => setAp({ backgroundDim: Number(e.target.value) })} disabled={!ap.background} />
          </Field>
        </Card>
      </div>

      <Card title="Roblox client mods">
        <p className="muted small">
          Replaces specific files in the installed Roblox version's content folder, like Bloxstrap mods. Originals are backed up and restored when you revert.
          Mods are re-applied automatically after Roblox updates. Executables are never touched.
        </p>
        {slots.loading && !slots.data && <Skeletons n={3} />}
        {slots.error && <ErrorBox error={slots.error} onRetry={slots.reload} />}
        <ul className="list">
          {slots.data?.map((s) => (
            <li key={s.slot.id}>
              <div className="grow">
                <b>{s.slot.name}</b> <span className="badge badge-muted">{s.slot.group}</span> <span className="badge badge-muted">.{s.slot.format}</span>
                <div className="muted small">{s.slot.description}</div>
                {!s.compatible && <div className="error small">Not compatible: {s.reason}</div>}
              </div>
              {s.enabled && <span className="badge badge-ok">Applied</span>}
              <button className="btn btn-sm" disabled={!s.compatible || action.busy}
                onClick={() => action.run(async () => { const ok = await backend.modChooseFile(s.slot.id); await slots.reload(); return ok; }, (ok) => (ok ? `${s.slot.name} applied` : "No file chosen"))}>
                {s.enabled ? "Replace file" : "Choose file"}
              </button>
              {s.enabled && <button className="btn btn-sm" onClick={() => action.run(async () => { await backend.modRevert(s.slot.id); await slots.reload(); }, "Original restored")}>Revert</button>}
            </li>
          ))}
        </ul>
      </Card>
    </Page>
  );
}
