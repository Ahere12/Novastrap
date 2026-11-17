import { useMemo, useState, type FormEvent } from "react";
import { backend, type GameCard } from "../api/backend";
import { GameRow } from "../ui/games";
import { ErrorBox, Page, Skeletons, useAsync } from "../ui/kit";

export function Discover() {
  const home = useAsync(() => backend.discoverHome());
  const [query, setQuery] = useState("");
  const [submitted, setSubmitted] = useState("");
  const [genre, setGenre] = useState("all");
  const search = useAsync<GameCard[] | null>(
    () => (submitted ? backend.discoverSearch(submitted) : Promise.resolve(null)),
    [submitted],
  );

  const genres = useMemo(() => {
    const s = new Set<string>();
    home.data?.sorts.forEach((x) => x.games.forEach((g) => g.genre && s.add(g.genre)));
    return [...s].sort();
  }, [home.data]);

  const filter = (games: GameCard[]) => (genre === "all" ? games : games.filter((g) => g.genre === genre));
  const onSubmit = (e: FormEvent) => { e.preventDefault(); setSubmitted(query.trim()); };

  return (
    <Page title="Discover" subtitle="Experiences from Roblox's own charts, with search and categories.">
      <form className="join" onSubmit={onSubmit}>
        <input className="input" value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search experiences" aria-label="Search experiences" />
        <button className="btn" type="submit" disabled={!query.trim()}>Search</button>
        {submitted && <button className="btn" type="button" onClick={() => { setSubmitted(""); setQuery(""); }}>Clear</button>}
      </form>

      {genres.length > 0 && (
        <div className="chips" role="radiogroup" aria-label="Category">
          {["all", ...genres].map((g) => (
            <button key={g} role="radio" aria-checked={genre === g} className={`chip ${genre === g ? "on" : ""}`} onClick={() => setGenre(g)}>
              {g === "all" ? "All categories" : g}
            </button>
          ))}
        </div>
      )}

      {submitted && (
        <>
          {search.loading && <Skeletons n={2} tall />}
          {search.error && <ErrorBox error={search.error} onRetry={search.reload} />}
          {search.data && <GameRow title={`Results for “${submitted}”`} games={filter(search.data)} />}
        </>
      )}

      {home.loading && !home.data && <Skeletons n={4} tall />}
      {home.error && <ErrorBox error={home.error} onRetry={home.reload} />}
      {home.data && !submitted && (
        <>
          <GameRow title="Recommended for you" hint={home.data.recommendedHint ?? "Based on genres you play and favorite"} games={filter(home.data.recommended)} />
          {home.data.sorts.map((s) => <GameRow key={s.id} title={s.name} hint={s.description} games={filter(s.games)} />)}
        </>
      )}
    </Page>
  );
}
