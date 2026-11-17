import { useMemo, useState } from "react";
import { backend, type ItemValue } from "../api/backend";
import { Card, Empty, ErrorBox, Page, Skeletons, fmtNum, useAsync } from "../ui/kit";

const worth = (i: ItemValue) => i.value ?? i.rap ?? 0;
const total = (xs: ItemValue[]) => xs.reduce((s, i) => s + worth(i), 0);

function Side({ title, items, onRemove }: { title: string; items: ItemValue[]; onRemove: (id: number) => void }) {
  return (
    <div className="trade-side">
      <h3>{title}</h3>
      {items.length ? (
        <ul className="list compact">
          {items.map((i) => (
            <li key={i.id}>
              <span className="grow">{i.name}</span>
              <span className="muted">{i.value != null ? `Value ${fmtNum(i.value)}` : `RAP ${fmtNum(i.rap)}`}</span>
              <button className="btn btn-sm" onClick={() => onRemove(i.id)} aria-label={`Remove ${i.name}`}>×</button>
            </li>
          ))}
        </ul>
      ) : <Empty>Add items from the table.</Empty>}
      <p><b>Total: {fmtNum(total(items))}</b></p>
    </div>
  );
}

export function Trading() {
  const table = useAsync(() => backend.tradeValues());
  const [q, setQ] = useState("");
  const [a, setA] = useState<ItemValue[]>([]);
  const [b, setB] = useState<ItemValue[]>([]);
  const rows = useMemo(() => {
    const s = q.trim().toLowerCase();
    const items = table.data?.items ?? [];
    return (s ? items.filter((i) => i.name.toLowerCase().includes(s) || i.acronym?.toLowerCase() === s) : items).slice(0, 100);
  }, [table.data, q]);
  const diff = total(b) - total(a);
  const src = table.data;

  return (
    <Page title="Trading" subtitle="Limited item values and a side-by-side trade calculator.">
      {src && (
        <p className="source-note">
          Values from <button className="link" onClick={() => backend.openExternal("https://www.rolimons.com/")}>{src.sourceName}</button>, a third-party site not affiliated with Roblox,
          fetched {new Date(src.fetchedAt * 1000).toLocaleTimeString()}. “Value” is Rolimons' estimate; “RAP” is Roblox's recent average price.
          Items Rolimons hasn't valued show RAP only.
        </p>
      )}
      <Card title="Compare a trade">
        <div className="trade-grid">
          <Side title="You give" items={a} onRemove={(id) => setA(a.filter((x) => x.id !== id))} />
          <Side title="You get" items={b} onRemove={(id) => setB(b.filter((x) => x.id !== id))} />
        </div>
        {(a.length > 0 || b.length > 0) && (
          <p className={diff >= 0 ? "ok" : "error"}>
            {diff >= 0 ? "You receive" : "You give"} {fmtNum(Math.abs(diff))} more by {src?.sourceName} value{a.length && total(a) ? ` (${Math.round((diff / total(a)) * 100)}%)` : ""}.
          </p>
        )}
        <p className="muted small">Trade offers and history need Roblox's signed-in session, which Novastrap doesn't use. Use roblox.com/trades for those.</p>
      </Card>
      <Card title="Item values" right={<input className="input narrow" value={q} onChange={(e) => setQ(e.target.value)} placeholder="Search items or acronyms" aria-label="Search items" />}>
        {table.loading && !table.data && <Skeletons n={5} />}
        {table.error && <ErrorBox error={table.error} onRetry={table.reload} />}
        {table.data && !rows.length && <Empty>No items match.</Empty>}
        {rows.length > 0 && (
          <table className="table">
            <thead><tr><th>Item</th><th>Value</th><th>RAP</th><th>Demand</th><th>Trend</th><th /></tr></thead>
            <tbody>
              {rows.map((i) => (
                <tr key={i.id}>
                  <td>{i.name} {i.acronym && <span className="muted">({i.acronym})</span>} {i.projected && <span className="badge badge-warn">Projected</span>} {i.rare && <span className="badge badge-info">Rare</span>}</td>
                  <td>{i.value != null ? fmtNum(i.value) : <span className="muted">Not valued</span>}</td>
                  <td>{fmtNum(i.rap)}</td>
                  <td>{i.demand ?? "—"}</td>
                  <td>{i.trend ?? "—"}</td>
                  <td className="actions">
                    <button className="btn btn-sm" onClick={() => !a.some((x) => x.id === i.id) && setA([...a, i])}>Give</button>
                    <button className="btn btn-sm" onClick={() => !b.some((x) => x.id === i.id) && setB([...b, i])}>Get</button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Card>
    </Page>
  );
}
