"use client";

import { Play, Shuffle } from "lucide-react";
import { useEffect, useState, useSyncExternalStore } from "react";
import { toast } from "sonner";
import { Cover, EmptyState, LoadingBlock, ShelfRow, TrackRow } from "@/components/media";
import { useLibrary } from "@/components/library-provider";
import { usePlayer } from "@/components/player-provider";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useDownloads } from "@/lib/downloads";
import { catalogBlurb, catalogLabel, sameTrack } from "@/lib/catalog";
import { artistLine, formatBytes, greetingFor, shuffleTracks } from "@/lib/format";
import { getJson } from "@/lib/get-json";
import type { CardItem, CatalogSource, Collection, HomePayload, SearchPayload, Shelf, Track } from "@/lib/types";

function useRemote<T>(path: string | null) {
  const [result, setResult] = useState<{ path: string; data: T | null; error: string } | null>(null);
  useEffect(() => {
    if (!path) return;
    const controller = new AbortController();
    let active = true;
    getJson<T>(path, controller.signal)
      .then((data) => {
        if (active) setResult({ path, data, error: "" });
      })
      .catch((reason: Error) => {
        if (!active || reason.name === "AbortError") return;
        setResult({ path, data: null, error: reason.message });
      });
    return () => {
      active = false;
      controller.abort();
    };
  }, [path]);
  const matched = result?.path === path ? result : null;
  return {
    data: matched?.data ?? null,
    error: matched?.error ?? "",
    loading: Boolean(path) && !matched,
  };
}

function subscribeGreeting() {
  return () => undefined;
}

function useGreeting() {
  return useSyncExternalStore(subscribeGreeting, () => greetingFor(new Date()), () => "Welcome back");
}

type BrowseHandlers = {
  onOpen: (card: CardItem) => void;
  onPlay: (card: CardItem, shelf?: Shelf) => void;
  onAlbum: (id: string, source?: CatalogSource) => void;
  onArtist: (id: string, source?: CatalogSource) => void;
};

export function HomeView({
  source,
  mood,
  onMood,
  browse,
}: {
  source: CatalogSource;
  mood: string;
  onMood: (mood: string) => void;
  browse: BrowseHandlers;
}) {
  const [attempt, setAttempt] = useState(0);
  const greeting = useGreeting();
  const path = `/api/home?source=${source}${mood ? `&mood=${encodeURIComponent(mood)}` : ""}${attempt ? `&retry=${attempt}` : ""}`;
  const { data, error, loading } = useRemote<HomePayload>(path);

  if (loading && !data) return <LoadingBlock label="Loading home" />;
  if (error && !data) {
    return (
      <EmptyState
        title={`${catalogLabel(source)} didn't answer`}
        body={error}
        action={
          <Button type="button" onClick={() => setAttempt((value) => value + 1)}>
            Try again
          </Button>
        }
      />
    );
  }

  return (
    <div className="space-y-8">
      <header className="space-y-3">
        <p className="text-sm text-white/50">Windows 11 edition</p>
        <h1 className="text-4xl font-semibold tracking-tight">{greeting}</h1>
        <p className="max-w-xl text-sm leading-6 text-white/60">{catalogBlurb(source)}</p>
        <div className="flex flex-wrap gap-2 pt-1">
          {(data?.filters || []).map((filter) => (
            <button
              key={filter}
              type="button"
              onClick={() => onMood(filter === mood ? "" : filter)}
              className={`rounded-full px-3 py-1.5 text-sm ${filter === mood ? "bg-[var(--vivi)] text-black" : "bg-white/8 text-white/80 hover:bg-white/12"}`}
              aria-pressed={filter === mood}
            >
              {filter}
            </button>
          ))}
        </div>
      </header>
      {data?.shelves.length ? (
        data.shelves.map((shelf) => (
          <ShelfRow key={shelf.title} shelf={shelf} onOpen={browse.onOpen} onPlay={(card) => browse.onPlay(card, shelf)} />
        ))
      ) : (
        <EmptyState title="Nothing on this shelf" body={`${catalogLabel(source)} didn't send any music. Try another filter, or search for something specific.`} />
      )}
    </div>
  );
}

export function SearchView({
  source,
  query,
  onQuery,
  browse,
}: {
  source: CatalogSource;
  query: string;
  onQuery: (query: string) => void;
  browse: BrowseHandlers;
}) {
  const library = useLibrary();
  const rememberSearch = library.rememberSearch;
  const [attempt, setAttempt] = useState(0);
  const trimmed = query.trim();
  const requestKey = trimmed.length < 2 ? "" : `${source}\n${trimmed}\n${attempt}`;
  const [result, setResult] = useState<{ key: string; data: SearchPayload | null; suggestions: string[]; error: string } | null>(null);

  useEffect(() => {
    if (!requestKey) return;
    const controller = new AbortController();
    let active = true;
    const timer = window.setTimeout(() => {
      rememberSearch(trimmed);
      const encoded = encodeURIComponent(trimmed);
      Promise.all([
        getJson<SearchPayload>(`/api/search?source=${source}&q=${encoded}`, controller.signal),
        source === "youtube"
          ? getJson<string[]>(`/api/suggest?q=${encoded}`, controller.signal).catch(() => [] as string[])
          : Promise.resolve([] as string[]),
      ])
        .then(([data, suggestions]) => {
          if (active) setResult({ key: requestKey, data, suggestions, error: "" });
        })
        .catch((reason: Error) => {
          if (!active || reason.name === "AbortError") return;
          setResult({ key: requestKey, data: null, suggestions: [], error: reason.message });
        });
    }, 280);
    return () => {
      active = false;
      controller.abort();
      window.clearTimeout(timer);
    };
  }, [requestKey, rememberSearch, source, trimmed]);

  const matched = result?.key === requestKey ? result : null;
  const data = matched?.data ?? null;
  const suggestions = matched?.suggestions ?? [];
  const error = matched?.error ?? "";
  const loading = Boolean(requestKey) && !matched;

  if (!query.trim()) {
    return (
      <div className="space-y-6">
        <h1 className="text-3xl font-semibold tracking-tight">Search</h1>
        <p className="max-w-lg text-sm leading-6 text-white/60">Look up a song, an album, an artist, or a playlist on {catalogLabel(source)}.</p>
        {library.searches.length ? (
          <div className="flex flex-wrap gap-2">
            {library.searches.map((item) => (
              <button key={item} type="button" onClick={() => onQuery(item)} className="rounded-full bg-white/8 px-3 py-1.5 text-sm hover:bg-white/12">
                {item}
              </button>
            ))}
          </div>
        ) : null}
      </div>
    );
  }

  const songs = data ? [...data.songs, ...data.videos] : [];
  const empty = data && !songs.length && !data.albums.length && !data.artists.length && !data.playlists.length;

  return (
    <div className="space-y-5">
      <h1 className="text-3xl font-semibold tracking-tight">Results for “{query}”</h1>
      <p className="text-sm text-white/45">{catalogLabel(source)}</p>
      {suggestions.length ? (
        <div className="flex flex-wrap gap-2">
          {suggestions.map((item) => (
            <button key={item} type="button" onClick={() => onQuery(item)} className="rounded-full bg-white/8 px-3 py-1 text-xs text-white/75 hover:bg-white/12">
              {item}
            </button>
          ))}
        </div>
      ) : null}
      {loading && !data ? <LoadingBlock label="Searching" /> : null}
      {error ? <EmptyState title="Search didn't finish" body={error} action={<Button type="button" onClick={() => setAttempt((value) => value + 1)}>Try again</Button>} /> : null}
      {empty ? <EmptyState title="No matches" body={`Nothing came back for “${query}”. Try the artist and the title together.`} /> : null}
      {data && !empty ? (
        <Tabs defaultValue="all">
          <TabsList>
            <TabsTrigger value="all">All</TabsTrigger>
            <TabsTrigger value="songs">Songs</TabsTrigger>
            <TabsTrigger value="albums">Albums</TabsTrigger>
            <TabsTrigger value="artists">Artists</TabsTrigger>
            <TabsTrigger value="playlists">Playlists</TabsTrigger>
          </TabsList>
          <TabsContent value="all" className="space-y-8 pt-4">
            {songs.length ? <TrackList tracks={songs} browse={browse} /> : null}
            {data.albums.length ? <ShelfRow shelf={{ title: "Albums", items: data.albums }} onOpen={browse.onOpen} onPlay={(card) => browse.onPlay(card)} /> : null}
            {data.artists.length ? <ShelfRow shelf={{ title: "Artists", items: data.artists }} onOpen={browse.onOpen} onPlay={(card) => browse.onPlay(card)} /> : null}
            {data.playlists.length ? <ShelfRow shelf={{ title: "Playlists", items: data.playlists }} onOpen={browse.onOpen} onPlay={(card) => browse.onPlay(card)} /> : null}
          </TabsContent>
          <TabsContent value="songs" className="pt-4">
            {songs.length ? <TrackList tracks={songs} browse={browse} /> : <EmptyState title="No songs" body="This search didn't include songs." />}
          </TabsContent>
          <TabsContent value="albums" className="pt-4">
            {data.albums.length ? <ShelfRow shelf={{ title: "Albums", items: data.albums }} onOpen={browse.onOpen} onPlay={(card) => browse.onPlay(card)} /> : <EmptyState title="No albums" body="This search didn't include albums." />}
          </TabsContent>
          <TabsContent value="artists" className="pt-4">
            {data.artists.length ? <ShelfRow shelf={{ title: "Artists", items: data.artists }} onOpen={browse.onOpen} onPlay={(card) => browse.onPlay(card)} /> : <EmptyState title="No artists" body="This search didn't include artists." />}
          </TabsContent>
          <TabsContent value="playlists" className="pt-4">
            {data.playlists.length ? <ShelfRow shelf={{ title: "Playlists", items: data.playlists }} onOpen={browse.onOpen} onPlay={(card) => browse.onPlay(card)} /> : <EmptyState title="No playlists" body="This search didn't include playlists." />}
          </TabsContent>
        </Tabs>
      ) : null}
    </div>
  );
}

export function TrackList({ tracks, browse, onRemove }: { tracks: Track[]; browse: BrowseHandlers; onRemove?: (track: Track) => void }) {
  const player = usePlayer();
  return (
    <div>
      {tracks.map((track, index) => (
        <TrackRow
          key={`${track.id}-${index}`}
          track={track}
          position={index + 1}
          active={player.current ? sameTrack(player.current, track) : false}
          onPlay={() => player.playTracks(tracks, index)}
          onAlbum={track.album?.id ? () => browse.onAlbum(track.album!.id!, track.source) : undefined}
          onArtist={track.artists.find((artist) => artist.id) ? () => browse.onArtist(track.artists.find((artist) => artist.id)!.id!, track.source) : undefined}
          onRemove={onRemove ? () => onRemove(track) : undefined}
        />
      ))}
    </div>
  );
}

export function CollectionRoute({
  kind,
  id,
  source = "youtube",
  browse,
}: {
  kind: "album" | "playlist" | "artist";
  id: string;
  source?: CatalogSource;
  browse: BrowseHandlers;
}) {
  const player = usePlayer();
  const [attempt, setAttempt] = useState(0);
  const { data, error, loading } = useRemote<Collection>(`/api/${kind}?source=${source}&id=${encodeURIComponent(id)}${attempt ? `&retry=${attempt}` : ""}`);

  if (loading && !data) return <LoadingBlock label="Loading" />;
  if (error || !data) {
    return (
      <EmptyState
        title="That page didn't open"
        body={error || `${catalogLabel(source)} sent back an empty page.`}
        action={
          <Button type="button" onClick={() => setAttempt((value) => value + 1)}>
            Try again
          </Button>
        }
      />
    );
  }

  return (
    <div className="space-y-8">
      <header className="flex flex-col gap-5 sm:flex-row sm:items-end">
        <Cover src={data.thumbnail} alt="" className="size-44 rounded-3xl shadow-2xl shadow-black/40 sm:size-56" />
        <div className="min-w-0 space-y-3">
          <p className="text-xs tracking-[0.18em] text-white/45 uppercase">{data.kind}</p>
          <h1 className="text-4xl font-semibold tracking-tight text-balance">{data.title}</h1>
          <p className="text-sm text-white/60">{data.subtitle}</p>
          {data.description ? <p className="line-clamp-3 max-w-2xl text-sm leading-6 text-white/55">{data.description}</p> : null}
          <div className="flex gap-2">
            <Button type="button" className="rounded-full bg-white text-black hover:bg-white/90" onClick={() => player.playTracks(data.tracks, 0)} disabled={!data.tracks.length}>
              <Play className="fill-current" /> Play
            </Button>
            <Button type="button" variant="outline" className="rounded-full" onClick={() => player.playTracks(shuffleTracks(data.tracks), 0)} disabled={data.tracks.length < 2}>
              <Shuffle /> Shuffle
            </Button>
          </div>
        </div>
      </header>
      {data.tracks.length ? <TrackList tracks={data.tracks} browse={browse} /> : <EmptyState title="No songs on this page" body={`The page opened, but ${catalogLabel(source)} didn't list tracks that can be played.`} />}
      {data.shelves?.map((shelf) => (
        <ShelfRow key={shelf.title} shelf={shelf} onOpen={browse.onOpen} onPlay={(card) => browse.onPlay(card, shelf)} />
      ))}
    </div>
  );
}

export function LibraryView({
  onLiked,
  onDownloads,
  onRecents,
  onLocal,
  onCreate,
}: {
  onLiked: () => void;
  onDownloads: () => void;
  onRecents: () => void;
  onLocal: (id: string) => void;
  onCreate: () => void;
}) {
  const library = useLibrary();
  const downloads = useDownloads();
  const downloadBytes = downloads.items.reduce((sum, item) => sum + item.bytes, 0);
  return (
    <div className="space-y-8">
      <header className="flex items-end justify-between gap-4">
        <div>
          <h1 className="text-4xl font-semibold tracking-tight">Library</h1>
          <p className="mt-2 max-w-lg text-sm leading-6 text-white/60">Liked songs, playlists, and downloads stay on this PC, in this browser. Clearing site data removes them.</p>
        </div>
        <Button type="button" className="rounded-full" onClick={onCreate}>
          New playlist
        </Button>
      </header>
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        <button type="button" onClick={onLiked} className="rounded-3xl bg-gradient-to-br from-[var(--vivi)] to-fuchsia-950 p-5 text-left">
          <p className="text-sm text-white/80">Liked songs</p>
          <p className="mt-6 text-2xl font-semibold">{library.liked.length}</p>
        </button>
        <button type="button" onClick={onDownloads} className="rounded-3xl bg-white/8 p-5 text-left hover:bg-white/10">
          <p className="text-sm text-white/60">Downloads</p>
          <p className="mt-6 text-2xl font-semibold">{downloads.items.length}</p>
          <p className="truncate text-sm text-white/50">{downloads.items.length ? formatBytes(downloadBytes) : "Save a song from its menu."}</p>
        </button>
        <button type="button" onClick={onRecents} className="rounded-3xl bg-white/8 p-5 text-left hover:bg-white/10">
          <p className="text-sm text-white/60">Recently played</p>
          <p className="mt-6 text-2xl font-semibold">{library.recents[0]?.title || "Nothing yet"}</p>
          <p className="truncate text-sm text-white/50">{library.recents[0] ? artistLine(library.recents[0]) : "Play a song and it shows up here."}</p>
        </button>
      </div>
      <section className="space-y-2">
        <h2 className="text-lg font-semibold">Playlists</h2>
        {library.playlists.length ? (
          library.playlists.map((playlist) => (
            <button key={playlist.id} type="button" onClick={() => onLocal(playlist.id)} className="flex w-full items-center justify-between rounded-2xl px-3 py-3 text-left hover:bg-white/6">
              <span>
                <span className="block font-medium">{playlist.name}</span>
                <span className="text-xs text-white/45">{playlist.tracks.length} songs</span>
              </span>
            </button>
          ))
        ) : (
          <EmptyState title="No playlists yet" body="Create one here, or use the menu on a song and choose New playlist." />
        )}
      </section>
    </div>
  );
}

export function LocalList({
  title,
  body,
  tracks,
  browse,
  onRemove,
  header,
  emptyTitle,
  emptyBody,
}: {
  title: string;
  body: string;
  tracks: Track[];
  browse: BrowseHandlers;
  onRemove?: (track: Track) => void;
  header?: React.ReactNode;
  emptyTitle?: string;
  emptyBody?: string;
}) {
  const player = usePlayer();
  return (
    <div className="space-y-6">
      <header className="space-y-3">
        <h1 className="text-4xl font-semibold tracking-tight">{title}</h1>
        <p className="max-w-lg text-sm text-white/60">{body}</p>
        {header}
        <Button type="button" className="rounded-full bg-white text-black hover:bg-white/90" disabled={!tracks.length} onClick={() => player.playTracks(tracks, 0)}>
          <Play className="fill-current" /> Play
        </Button>
      </header>
      {tracks.length ? (
        <TrackList tracks={tracks} browse={browse} onRemove={onRemove} />
      ) : (
        <EmptyState title={emptyTitle ?? "Nothing saved here"} body={emptyBody ?? "When you heart a song or add it to a playlist, it stays on this PC."} />
      )}
    </div>
  );
}

export function DownloadsView({ browse }: { browse: BrowseHandlers }) {
  const downloads = useDownloads();
  const tracks = downloads.items.map((item) => item.track);
  const bytes = downloads.items.reduce((sum, item) => sum + item.bytes, 0);
  return (
    <LocalList
      title="Downloads"
      body={tracks.length ? `${tracks.length} saved in this browser · ${formatBytes(bytes)}. Play works without asking YouTube again.` : "Songs you download stay in this browser on this PC."}
      tracks={tracks}
      browse={browse}
      onRemove={(track) => void downloads.remove(track.id)}
      emptyTitle="No downloads yet"
      emptyBody="Open the menu on a song and choose Download. The file stays on this PC."
    />
  );
}

export function LocalPlaylist({ id, browse }: { id: string; browse: BrowseHandlers }) {
  const library = useLibrary();
  const playlist = library.playlists.find((item) => item.id === id);
  if (!playlist) return <EmptyState title="That playlist is gone" body="It was deleted from this browser, or it never existed on this PC." />;
  return <LocalPlaylistEditor key={playlist.id} playlist={playlist} browse={browse} />;
}

function LocalPlaylistEditor({
  playlist,
  browse,
}: {
  playlist: { id: string; name: string; tracks: Track[] };
  browse: BrowseHandlers;
}) {
  const library = useLibrary();
  const [name, setName] = useState(playlist.name);
  const [confirming, setConfirming] = useState(false);
  return (
    <LocalList
      title={playlist.name}
      body="This playlist lives only in this browser."
      tracks={playlist.tracks}
      browse={browse}
      onRemove={(track) => library.removeFromPlaylist(playlist.id, track)}
      header={
        <div className="flex max-w-md flex-col gap-2 sm:flex-row">
          <Input value={name} aria-label="Playlist name" onChange={(event) => setName(event.target.value)} />
          <Button type="button" variant="outline" onClick={() => library.renamePlaylist(playlist.id, name)}>
            Rename
          </Button>
          <Button
            type="button"
            variant="destructive"
            onClick={() => {
              if (!confirming) {
                setConfirming(true);
                toast("Click delete again to remove this playlist.");
                window.setTimeout(() => setConfirming(false), 3000);
                return;
              }
              library.deletePlaylist(playlist.id);
            }}
          >
            {confirming ? "Confirm" : "Delete"}
          </Button>
        </div>
      }
    />
  );
}
