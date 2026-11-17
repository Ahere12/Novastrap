import { useState, type FormEvent } from "react";
import { backend, errorMessage, type CatalogCategory, type CatalogItem, type CatalogSort } from "../api/backend";
import { useApp } from "../app/context";
import { Card, Empty, ErrorBox, Img, Modal, Page, Skeletons, fmtNum, useAsync } from "../ui/kit";

const CATS: [CatalogCategory, string][] = [["all", "All"], ["accessories", "Accessories"], ["clothing", "Clothing"], ["bodyParts", "Body parts"], ["animations", "Animations"], ["collectibles", "Collectibles"]];
const SORTS: [CatalogSort, string][] = [["relevance", "Relevance"], ["mostFavorited", "Most favorited"], ["bestselling", "Bestselling"], ["recentlyUpdated", "Recently updated"], ["priceLowToHigh", "Price: low to high"], ["priceHighToLow", "Price: high to low"]];

export function price(i: CatalogItem) {
  if (i.isOffSale && i.lowestResalePrice) return `Resale from R$${fmtNum(i.lowestResalePrice)}`;
  if (i.isOffSale) return "Off sale";
  if (i.price === 0) return "Free";
  return i.lowestPrice != null ? `R$${fmtNum(i.lowestPrice)}` : i.price != null ? `R$${fmtNum(i.price)}` : "—";
}

function ItemModal({ item, onClose }: { item: CatalogItem; onClose: () => void }) {
  const { favorites, refreshFavorites, toast } = useApp();
  const fav = favorites.items.some((f) => f.id === item.id && f.itemType === item.itemType);
  const toggle = async () => {
    try {
      await backend.toggleFavoriteItem({ id: item.id, itemType: item.itemType, name: item.name, thumbnailUrl: item.thumbnailUrl, addedAt: 0 });
      await refreshFavorites();
    } catch (e) { toast(false, errorMessage(e)); }
  };
  const url = item.itemType === "Bundle" ? `https://www.roblox.com/bundles/${item.id}` : `https://www.roblox.com/catalog/${item.id}`;
  return (
    <Modal onClose={onClose}>
      <div className="item-detail">
        <Img src={item.thumbnailUrl} alt={item.name} className="item-img" />
        <div>
          <h2>{item.name}</h2>
          <p className="muted">by {item.creatorName}{item.creatorHasVerifiedBadge ? " ✔" : ""} · {item.itemType}</p>
          <div className="stats-strip compact">
            <div><b>{price(item)}</b><span>price</span></div>
            <div><b>{fmtNum(item.favoriteCount)}</b><span>favorites</span></div>
            {item.totalQuantity != null && <div><b>{fmtNum(item.totalQuantity)}</b><span>total stock</span></div>}
            {item.unitsAvailableForConsumption != null && <div><b>{fmtNum(item.unitsAvailableForConsumption)}</b><span>left</span></div>}
          </div>
          {item.itemRestrictions.length > 0 && <p>{item.itemRestrictions.map((r) => <span key={r} className="badge badge-info">{r}</span>)}</p>}
          <p className="description">{item.description || "No description."}</p>
          <div className="row">
            <button className="btn" onClick={toggle}>{fav ? "★ Favorited" : "☆ Favorite"}</button>
            <button className="btn btn-primary" onClick={() => backend.openExternal(url)}>Open on Roblox</button>
          </div>
          <p className="muted small">Buying and wearing items happens on roblox.com, where you're signed in. Novastrap doesn't handle purchases.</p>
        </div>
      </div>
    </Modal>
  );
}

export function Catalog() {
  const { settings, favorites } = useApp();
  const [kw, setKw] = useState("");
  const [q, setQ] = useState({ keyword: "", category: "accessories" as CatalogCategory, sort: "relevance" as CatalogSort });
  const [extra, setExtra] = useState<CatalogItem[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [open, setOpen] = useState<CatalogItem | null>(null);
  const page = useAsync(async () => {
    const p = await backend.catalogSearch(q.keyword, q.category, q.sort, null);
    setExtra([]); setCursor(p.nextPageCursor);
    return p;
  }, [q]);
  const active = settings?.accounts.activeUserId ?? null;
  const avatar = useAsync(() => (active ? backend.avatarRender(active) : Promise.resolve(null)), [active]);
  const more = async () => {
    if (!cursor) return;
    const p = await backend.catalogSearch(q.keyword, q.category, q.sort, cursor);
    setExtra((x) => [...x, ...p.data]); setCursor(p.nextPageCursor);
  };
  const items = [...(page.data?.data ?? []), ...extra];
  const onSubmit = (e: FormEvent) => { e.preventDefault(); setQ({ ...q, keyword: kw }); };

  return (
    <Page title="Catalog" subtitle="Browse the Roblox marketplace. Prices come straight from Roblox.">
      <div className="grid-side">
        <div className="stack">
          <form className="join" onSubmit={onSubmit}>
            <input className="input" value={kw} onChange={(e) => setKw(e.target.value)} placeholder="Search items" aria-label="Search items" />
            <select className="input narrow" value={q.sort} onChange={(e) => setQ({ ...q, sort: e.target.value as CatalogSort })} aria-label="Sort">
              {SORTS.map(([v, l]) => <option key={v} value={v}>{l}</option>)}
            </select>
            <button className="btn" type="submit">Search</button>
          </form>
          <div className="chips">
            {CATS.map(([v, l]) => <button key={v} className={`chip ${q.category === v ? "on" : ""}`} onClick={() => setQ({ ...q, category: v })}>{l}</button>)}
          </div>
          {page.loading && <Skeletons n={3} tall />}
          {page.error && <ErrorBox error={page.error} onRetry={page.reload} />}
          {page.data && !items.length && <Empty>No items found.</Empty>}
          <div className="item-grid">
            {items.map((i) => (
              <button key={`${i.itemType}-${i.id}`} className="item-card" onClick={() => setOpen(i)}>
                <Img src={i.thumbnailUrl} alt="" className="item-thumb" />
                <span className="item-name">{i.name}</span>
                <span className="muted small">{price(i)}</span>
              </button>
            ))}
          </div>
          {cursor && !page.loading && <button className="btn" onClick={more}>Load more</button>}
        </div>
        <div className="stack">
          <Card title="Your avatar">
            {!active && <Empty>Pick an active account to preview its avatar.</Empty>}
            {avatar.loading && active && <Skeletons n={1} tall />}
            {avatar.data && <Img src={avatar.data} alt="Current avatar" className="avatar-render" />}
            {active && <button className="btn btn-sm" onClick={() => backend.openExternal("https://www.roblox.com/my/avatar")}>Edit on Roblox</button>}
          </Card>
          <Card title="Favorite items">
            {favorites.items.length ? (
              <ul className="list compact">
                {favorites.items.map((f) => <li key={`${f.itemType}-${f.id}`}><Img src={f.thumbnailUrl} alt="" className="avatar sm" /><span className="grow">{f.name}</span></li>)}
              </ul>
            ) : <Empty>Favorite items to keep them here.</Empty>}
          </Card>
        </div>
      </div>
      {open && <ItemModal item={open} onClose={() => setOpen(null)} />}
    </Page>
  );
}
