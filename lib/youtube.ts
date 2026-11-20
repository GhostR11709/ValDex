import { Innertube } from "youtubei.js";
import type { CardItem, CardKind, Collection, HomePayload, Person, SearchPayload, Shelf, Track } from "./types";

// Catalog access for the Windows client. Uses youtubei.js (MIT).
// Does not include Vivi Music's Musixmatch or JioSaavn modules.

type Loose = {
  type?: string;
  id?: string;
  video_id?: string;
  title?: unknown;
  name?: unknown;
  subtitle?: unknown;
  description?: unknown;
  strapline_text_one?: unknown;
  item_type?: string;
  artists?: { name?: string; channel_id?: string }[];
  author?: { name?: string; channel_id?: string } | string;
  album?: { id?: string; name?: string };
  duration?: { seconds?: number };
  thumbnails?: unknown;
  thumbnail?: unknown;
  contents?: unknown;
  header?: Loose;
  primary?: Loose;
  endpoint?: { payload?: { videoId?: string; browseId?: string } };
  suggestion?: unknown;
  footer?: unknown;
};

const CLIENTS = ["IOS", "ANDROID", "TV", "WEB"] as const;

let ytPromise: Promise<Innertube> | null = null;
let tail: Promise<unknown> = Promise.resolve();
let audioBlockedUntil = 0;
const streamCache = new Map<string, { url: string; mime: string; expires: number }>();

function getYt() {
  if (!ytPromise) {
    ytPromise = Innertube.create({ generate_session_locally: true }).catch((error: unknown) => {
      ytPromise = null;
      throw error;
    });
  }
  return ytPromise;
}

function withYt<T>(fn: (yt: Innertube) => Promise<T>) {
  const run = tail.then(() => getYt().then(fn));
  tail = run.then(
    () => undefined,
    () => undefined,
  );
  return run;
}

function asText(value: unknown): string {
  if (typeof value === "string") return value.trim();
  if (typeof value === "number") return String(value);
  if (value && typeof value === "object") {
    const record = value as { text?: unknown; description?: unknown; toString?: () => string };
    if (typeof record.text === "string" && record.text.trim()) return record.text.trim();
    if (record.description && record.description !== value) {
      const nested = asText(record.description);
      if (nested) return nested;
    }
    if (typeof record.toString === "function") {
      const text = record.toString();
      if (text && text !== "[object Object]") return text.trim();
    }
  }
  return "";
}

function list(value: unknown): Loose[] {
  if (!value) return [];
  if (Array.isArray(value)) return value as Loose[];
  if (typeof value === "object" && Symbol.iterator in value) return Array.from(value as Iterable<Loose>);
  return [];
}

function walkThumbs(node: unknown, depth: number, found: { url: string; width: number; index: number }[]) {
  if (!node || depth > 5) return;
  if (Array.isArray(node)) {
    for (const entry of node) walkThumbs(entry, depth + 1, found);
    return;
  }
  if (typeof node !== "object") return;
  const record = node as { url?: unknown; width?: unknown; contents?: unknown; thumbnail?: unknown };
  if (typeof record.url === "string") {
    found.push({
      url: record.url,
      width: typeof record.width === "number" ? record.width : 0,
      index: found.length,
    });
  }
  if (record.contents) walkThumbs(record.contents, depth + 1, found);
  if (record.thumbnail && record.thumbnail !== node) walkThumbs(record.thumbnail, depth + 1, found);
}

function bestThumb(source: unknown) {
  const found: { url: string; width: number; index: number }[] = [];
  walkThumbs(source, 0, found);
  found.sort((a, b) => a.width - b.width || a.index - b.index);
  const url = found.at(-1)?.url || "";
  const secure = url.startsWith("http://") ? `https://${url.slice("http://".length)}` : url;
  return secure.replace(/=w\d+-h\d+((?:-[A-Za-z0-9]+)*)/, "=w720-h720$1");
}

function peopleOf(item: Loose): Person[] {
  if (Array.isArray(item.artists) && item.artists.length) {
    return item.artists
      .map((artist) => ({ name: artist.name?.trim() || "", id: artist.channel_id }))
      .filter((artist) => artist.name);
  }
  if (typeof item.author === "string" && item.author.trim()) return [{ name: item.author.trim() }];
  if (item.author && typeof item.author === "object") {
    const name = asText("name" in item.author ? item.author.name : item.author);
    if (name) return [{ name, id: item.author.channel_id }];
  }
  return [];
}

function idOf(item: Loose) {
  if (item.id) return item.id;
  if (item.video_id) return item.video_id;
  return item.endpoint?.payload?.videoId || item.endpoint?.payload?.browseId || "";
}

function kindOf(item: Loose): CardKind | null {
  if (item.item_type === "song" || item.item_type === "non_music_track") return "song";
  if (item.item_type === "video") return "video";
  if (item.item_type === "album") return "album";
  if (item.item_type === "playlist") return "playlist";
  if (item.item_type === "artist" || item.item_type === "library_artist") return "artist";
  const id = idOf(item);
  if (!id) return null;
  if (/^[\w-]{11}$/.test(id)) return "song";
  if (id.startsWith("MPRE")) return "album";
  if (id.startsWith("UC")) return "artist";
  if (/^(VL|PL|RD|OLAK)/.test(id)) return "playlist";
  return null;
}

function unwrap(item: Loose): Loose {
  if (item.type === "PlaylistPanelVideoWrapper" && item.primary) return item.primary;
  return item;
}

function mapTrack(item: Loose, fallback?: Partial<Track>): Track | null {
  const source = unwrap(item);
  const id = source.video_id || (/^[\w-]{11}$/.test(source.id || "") ? source.id : "") || "";
  if (!id) return null;
  const title = asText(source.title) || asText(source.name);
  if (!title) return null;
  const artists = peopleOf(source);
  return {
    id,
    title,
    artists: artists.length ? artists : fallback?.artists || [],
    album: source.album?.name ? { name: source.album.name, id: source.album.id } : fallback?.album,
    duration: typeof source.duration?.seconds === "number" && source.duration.seconds > 0 ? source.duration.seconds : fallback?.duration,
    thumbnail: bestThumb(source.thumbnails ?? source.thumbnail) || fallback?.thumbnail || "",
  };
}

function mapCard(item: Loose): CardItem | null {
  const kind = kindOf(item);
  const id = idOf(item);
  if (!kind || !id) return null;
  const artists = peopleOf(item);
  const track = kind === "song" || kind === "video" ? mapTrack(item) : null;
  return {
    kind,
    id,
    title: asText(item.title) || asText(item.name) || "Untitled",
    subtitle: asText(item.subtitle) || artists.map((artist) => artist.name).join(", "),
    thumbnail: bestThumb(item.thumbnails ?? item.thumbnail),
    track: track || undefined,
  };
}

function dedupeTracks(tracks: Array<Track | null | undefined>) {
  const seen = new Set<string>();
  const out: Track[] = [];
  for (const track of tracks) {
    if (!track || seen.has(track.id)) continue;
    seen.add(track.id);
    out.push(track);
  }
  return out;
}

function dedupeCards(cards: Array<CardItem | null | undefined>) {
  const seen = new Set<string>();
  const out: CardItem[] = [];
  for (const card of cards) {
    if (!card) continue;
    const key = `${card.kind}:${card.id}`;
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(card);
  }
  return out;
}

function shelfOf(section: Loose, titleFrom: "header" | "title"): Shelf | null {
  const title = titleFrom === "header" ? asText(section.header?.title) || asText(section.title) : asText(section.title);
  const items = dedupeCards(list(section.contents).map((item) => mapCard(item)));
  if (!title || !items.length) return null;
  return { title, items };
}

function isReleaseMeta(value: string) {
  return /^(single|album|ep|playlist|song|video|podcast)$/i.test(value) || /^\d{4}$/.test(value) || /^\d+\s+(songs?|minutes?|hours?)$/i.test(value);
}

function headerArtists(header?: Loose): Person[] {
  if (!header) return [];
  const people = peopleOf(header);
  if (people.length) return people;
  const strap = header.strapline_text_one as Loose | undefined;
  const strapName = asText(strap);
  if (strapName && !isReleaseMeta(strapName)) return [{ name: strapName, id: strap?.endpoint?.payload?.browseId }];
  const name = asText(header.subtitle)
    .split(/[•·]/)
    .map((part) => part.trim())
    .find((part) => part && !isReleaseMeta(part));
  return name ? [{ name }] : [];
}

function headerSubtitle(header: Loose | undefined, artists: Person[], fallback: string) {
  const names = artists.map((artist) => artist.name).join(", ");
  const meta = asText(header?.subtitle);
  if (names && meta && meta !== names) return `${names} · ${meta}`;
  return names || meta || fallback;
}

export function isAllowedMediaHost(url: string) {
  try {
    const host = new URL(url).hostname;
    return host === "googlevideo.com" || host.endsWith(".googlevideo.com") || host === "youtube.com" || host.endsWith(".youtube.com");
  } catch {
    return false;
  }
}

export async function getHome(mood?: string): Promise<HomePayload> {
  return withYt(async (yt) => {
    let home = await yt.music.getHomeFeed();
    const filters = [...(home.filters || [])];
    if (mood && filters.includes(mood)) {
      try {
        home = await home.applyFilter(mood);
      } catch (error) {
        console.error(error);
      }
    }
    const shelves = list(home.sections)
      .filter((section) => section.type === "MusicCarouselShelf")
      .map((section) => shelfOf(section, "header"))
      .filter((shelf): shelf is Shelf => Boolean(shelf));
    return { filters, shelves };
  });
}

export async function searchCatalog(query: string): Promise<SearchPayload> {
  return withYt(async (yt) => {
    const result = await yt.music.search(query);
    const shelves = [result.songs, result.videos, result.albums, result.artists, result.playlists, ...list(result.contents)];
    const cards = dedupeCards(shelves.flatMap((shelf) => list((shelf as Loose | undefined)?.contents).map((item) => mapCard(item))));
    const take = (kind: CardKind) => cards.filter((card) => card.kind === kind);
    return {
      songs: dedupeTracks(take("song").map((card) => card.track)),
      videos: dedupeTracks(take("video").map((card) => card.track)),
      albums: take("album"),
      artists: take("artist"),
      playlists: take("playlist"),
    };
  });
}

export async function suggest(input: string) {
  return withYt(async (yt) => {
    const sections = await yt.music.getSearchSuggestions(input);
    const out: string[] = [];
    for (const section of sections || []) {
      for (const item of section.contents || []) {
        const text = asText((item as Loose).suggestion);
        if (text && !out.includes(text)) out.push(text);
      }
    }
    return out.slice(0, 8);
  });
}

export async function getAlbum(id: string): Promise<Collection> {
  return withYt(async (yt) => {
    const album = await yt.music.getAlbum(id);
    const header = album.header as Loose | undefined;
    const title = asText(header?.title) || "Album";
    const artists = headerArtists(header);
    const thumbnail = bestThumb(header?.thumbnails ?? header?.thumbnail ?? album.background);
    const tracks = dedupeTracks(
      list(album.contents).map((item) =>
        mapTrack(item, { artists, album: { name: title, id }, thumbnail }),
      ),
    );
    return {
      kind: "album",
      id,
      title,
      subtitle: headerSubtitle(header, artists, "Album"),
      thumbnail: thumbnail || tracks[0]?.thumbnail || "",
      description: asText(header?.description),
      tracks,
    };
  });
}

export async function getArtist(id: string): Promise<Collection> {
  return withYt(async (yt) => {
    const artist = await yt.music.getArtist(id);
    const header = artist.header as Loose | undefined;
    const title = asText(header?.title) || "Artist";
    const thumbnail = bestThumb(header?.thumbnail ?? header?.thumbnails);
    let tracks: Track[] = [];
    const shelves: Shelf[] = [];
    for (const section of list(artist.sections)) {
      if (section.type === "MusicShelf") {
        const mapped = dedupeTracks(list(section.contents).map((item) => mapTrack(item, { artists: [{ name: title, id }], thumbnail })));
        if (asText(section.title) === "Top songs" || tracks.length === 0) tracks = mapped;
      }
      if (section.type === "MusicCarouselShelf") {
        const shelf = shelfOf(section, "header");
        if (shelf) shelves.push(shelf);
      }
    }
    try {
      const all = await artist.getAllSongs();
      const more = dedupeTracks(list(all?.contents).map((item) => mapTrack(item, { artists: [{ name: title, id }], thumbnail })));
      if (more.length > tracks.length) tracks = more;
    } catch (error) {
      console.error(error);
    }
    return {
      kind: "artist",
      id,
      title,
      subtitle: asText(header?.subtitle) || "Artist",
      thumbnail,
      description: asText(header?.description),
      tracks,
      shelves,
    };
  });
}

export async function getPlaylist(id: string): Promise<Collection> {
  return withYt(async (yt) => {
    let page = await yt.music.getPlaylist(id);
    const header = page.header as Loose | undefined;
    const title = asText(header?.title) || "Playlist";
    const people = headerArtists(header);
    let items = list(page.contents);
    let guard = 0;
    while (page.has_continuation && guard < 3) {
      page = await page.getContinuation();
      items = items.concat(list(page.contents));
      guard += 1;
    }
    const thumbnail = bestThumb(header?.thumbnail ?? header?.thumbnails ?? page.background);
    const tracks = dedupeTracks(items.map((item) => mapTrack(item, { thumbnail })));
    return {
      kind: "playlist",
      id,
      title,
      subtitle: headerSubtitle(header, people, `${tracks.length} songs`),
      thumbnail: thumbnail || tracks[0]?.thumbnail || "",
      description: asText(header?.description),
      tracks,
    };
  });
}

export async function getRadio(videoId: string) {
  return withYt(async (yt) => {
    const panel = await yt.music.getUpNext(videoId);
    return dedupeTracks(list(panel?.contents).map((item) => mapTrack(item))).filter((track) => track.id !== videoId);
  });
}

export async function getPlainLyrics(videoId: string) {
  return withYt(async (yt) => {
    try {
      const shelf = await yt.music.getLyrics(videoId);
      const plain = asText(shelf?.description);
      if (!plain) return null;
      return { plain, source: asText(shelf?.footer) || "YouTube Music" };
    } catch (error) {
      console.error(error);
      return null;
    }
  });
}

export async function resolveAudio(videoId: string) {
  const cached = streamCache.get(videoId);
  if (cached && cached.expires > Date.now()) return { url: cached.url, mime: cached.mime };
  if (audioBlockedUntil > Date.now()) return null;
  return withYt(async (yt) => {
    for (const client of CLIENTS) {
      try {
        const info = await yt.getBasicInfo(videoId, { client });
        if (info.playability_status?.status === "LOGIN_REQUIRED") {
          audioBlockedUntil = Date.now() + 10 * 60 * 1000;
          return null;
        }
        if (info.playability_status?.status && info.playability_status.status !== "OK") continue;
        const format = info.chooseFormat({ type: "audio", quality: "best" });
        const url = await format.decipher(yt.session.player);
        if (!url || !isAllowedMediaHost(url)) continue;
        const mime = format.mime_type || "audio/mp4";
        streamCache.set(videoId, { url, mime, expires: Date.now() + 2 * 60 * 60 * 1000 });
        return { url, mime };
      } catch (error) {
        console.error(client, error instanceof Error ? error.message : error);
      }
    }
    return null;
  });
}

export function dropCachedAudio(videoId: string) {
  streamCache.delete(videoId);
}
