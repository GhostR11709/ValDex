import { Constants, Innertube, Platform } from "youtubei.js";
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

const CLIENTS = ["ANDROID", "MWEB", "YTMUSIC", "IOS", "WEB"] as const;
const STREAM_USER_AGENT = "Mozilla/5.0";

// The library extracts YouTube's player script and needs a runtime to run that script.
Platform.shim.eval = (data, env) => {
  void env;
  return new Function(data.output)();
};

let ytPromise: Promise<Innertube> | null = null;
let tail: Promise<unknown> = Promise.resolve();
let audioBlockedUntil = 0;
export type ResolvedAudio = { url: string; mime: string; userAgent: string; contentLength: number };

const streamCache = new Map<string, ResolvedAudio & { expires: number }>();

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
    source: "youtube",
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
    source: "youtube",
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
    if (!track) continue;
    const key = `${track.source || "youtube"}:${track.id}`;
    if (seen.has(key)) continue;
    seen.add(key);
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

function personName(value: unknown) {
  if (!value) return "";
  if (typeof value === "string") return value.trim();
  if (typeof value === "object" && "name" in value) {
    const name = (value as { name?: unknown }).name;
    if (typeof name === "string") return name.trim();
    return asText(name);
  }
  return asText(value);
}

function videoCard(item: Loose): CardItem | null {
  const id = item.video_id || (/^[\w-]{11}$/.test(item.id || "") ? item.id : "") || "";
  const title = asText(item.title);
  if (!id || !title) return null;
  const author = personName((item as { author?: unknown }).author);
  const seconds = item.duration?.seconds;
  const thumbnail = bestThumb(item.thumbnails ?? item.thumbnail);
  const track: Track = {
    id,
    source: "youtube",
    title,
    artists: author ? [{ name: author }] : [],
    duration: typeof seconds === "number" && seconds > 0 ? seconds : undefined,
    thumbnail,
  };
  return { kind: "video", id, source: "youtube", title, subtitle: author, thumbnail, track };
}

function songLength(cards: CardItem[]) {
  const songs = cards.filter((card) => {
    const seconds = card.track?.duration;
    return typeof seconds === "number" && seconds >= 45 && seconds <= 480;
  });
  return (songs.length >= 6 ? songs : cards).slice(0, 12);
}

async function youtubeVideoShelf(yt: Innertube, title: string, query: string): Promise<Shelf | null> {
  const found = await yt.search(query, { type: "video", duration: "three_to_twenty_mins" });
  const songs = dedupeCards(list(found.videos).map((item) => videoCard(item))).slice(0, 12);
  if (songs.length >= 4) return { title, items: songs };
  const open = await yt.search(query, { type: "video" });
  const items = songLength(dedupeCards(list(open.videos).map((item) => videoCard(item))));
  if (!items.length) return null;
  return { title, items };
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

const HOME_FALLBACK = [
  { title: "New music", query: "new music" },
  { title: "Top songs", query: "top songs" },
  { title: "Chill", query: "chill music" },
];

function shelfParserMissed(error: unknown) {
  return error instanceof Error && /Expected node of any type/.test(error.message);
}

async function fallbackHome(yt: Innertube): Promise<HomePayload> {
  const shelves = (await Promise.all(HOME_FALLBACK.map((shelf) => youtubeVideoShelf(yt, shelf.title, shelf.query)))).filter((shelf): shelf is Shelf => Boolean(shelf));
  if (!shelves.length) throw new Error("YouTube Music isn't available on this network.");
  return { filters: [], shelves };
}

export async function getHome(mood?: string): Promise<HomePayload> {
  return withYt(async (yt) => {
    try {
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
      if (shelves.length) return { filters, shelves };
    } catch (error) {
      if (!shelfParserMissed(error)) throw error;
    }
    return fallbackHome(yt);
  });
}

export async function searchCatalog(query: string): Promise<SearchPayload> {
  return withYt(async (yt) => {
    let cards: CardItem[] = [];
    try {
      const result = await yt.music.search(query);
      const shelves = [result.songs, result.videos, result.albums, result.artists, result.playlists, ...list(result.contents)];
      cards = dedupeCards(shelves.flatMap((shelf) => list((shelf as Loose | undefined)?.contents).map((item) => mapCard(item))));
    } catch (error) {
      if (!shelfParserMissed(error)) throw error;
    }
    const take = (kind: CardKind) => cards.filter((card) => card.kind === kind);
    const payload: SearchPayload = {
      songs: dedupeTracks(take("song").map((card) => card.track)),
      videos: dedupeTracks(take("video").map((card) => card.track)),
      albums: take("album"),
      artists: take("artist"),
      playlists: take("playlist"),
    };
    if (payload.songs.length || payload.videos.length) return payload;
    const web = await yt.search(query, { type: "video" });
    return { songs: [], videos: dedupeTracks(list(web.videos).map((item) => videoCard(item)?.track)), albums: [], artists: [], playlists: [] };
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

export function youtubeFetchHeaders(userAgent: string, range?: string | null) {
  const headers = new Headers();
  headers.set("user-agent", userAgent || STREAM_USER_AGENT);
  headers.set("accept", "*/*");
  headers.set("origin", "https://www.youtube.com");
  headers.set("referer", "https://www.youtube.com");
  if (range) headers.set("range", range);
  return headers;
}

export async function resolveAudio(videoId: string): Promise<ResolvedAudio | null> {
  const cached = streamCache.get(videoId);
  if (cached && cached.expires > Date.now()) {
    return { url: cached.url, mime: cached.mime, userAgent: cached.userAgent, contentLength: cached.contentLength };
  }
  if (audioBlockedUntil > Date.now()) return null;
  return withYt(async (yt) => {
    let blocked = true;
    for (const client of CLIENTS) {
      try {
        const info = await yt.getBasicInfo(videoId, { client });
        const status = info.playability_status?.status;
        if (status === "LOGIN_REQUIRED") continue;
        blocked = false;
        if (status && status !== "OK") continue;
        const userAgent = clientUserAgent(client);
        for (const format of listedFormats(info)) {
          if (!format.url && !format.signature_cipher && !format.cipher) continue;
          const url = await format.decipher(yt.session.player);
          if (!url || !isAllowedMediaHost(url)) continue;
          let contentLength = format.content_length || 0;
          if (contentLength < 2048) contentLength = await measuredLength(url, userAgent);
          if (contentLength < 2048) continue;
          const mime = (format.mime_type || "audio/mp4").split(";")[0];
          if (!(await servesWholeFile(url, contentLength, userAgent))) continue;
          const audio = { url, mime, userAgent, contentLength };
          streamCache.set(videoId, { ...audio, expires: Date.now() + 2 * 60 * 60 * 1000 });
          return audio;
        }
      } catch (error) {
        blocked = false;
        console.error(client, error instanceof Error ? error.message : error);
      }
    }
    if (blocked) audioBlockedUntil = Date.now() + 10 * 60 * 1000;
    return null;
  });
}

function clientUserAgent(client: (typeof CLIENTS)[number]) {
  const record = Constants.CLIENTS[client] as { USER_AGENT?: string };
  return record.USER_AGENT || STREAM_USER_AGENT;
}

function listedFormats(info: Awaited<ReturnType<Innertube["getBasicInfo"]>>) {
  const audio = [...(info.streaming_data?.adaptive_formats || [])].filter((format) => format.has_audio && !format.has_video && (format.content_length || 0) > 0);
  const progressive = [...(info.streaming_data?.formats || [])].filter((format) => format.has_audio && (format.mime_type || "").startsWith("video/mp4") && ((format.content_length || 0) > 0 || Boolean(format.url || format.signature_cipher || format.cipher)));
  audio.sort((left, right) => (right.bitrate || 0) - (left.bitrate || 0));
  progressive.sort((left, right) => (right.bitrate || 0) - (left.bitrate || 0));
  return [...progressive, ...(audio[0] ? [audio[0]] : [])];
}

async function measuredLength(url: string, userAgent: string) {
  const response = await openMedia(url, youtubeFetchHeaders(userAgent, "bytes=0-1"), 0, AbortSignal.timeout(8000));
  const fromRange = rangeTotal(response.headers.get("content-range"));
  const fromLength = Number(response.headers.get("content-length") || 0);
  await response.body?.cancel().catch(() => undefined);
  if (fromRange >= 2048) return fromRange;
  if (response.status === 200 && fromLength >= 2048) return fromLength;
  return 0;
}

function rangeTotal(header: string | null) {
  const match = header?.match(/\/(\d+)\s*$/);
  return match ? Number(match[1]) : 0;
}

async function servesWholeFile(url: string, contentLength: number, userAgent: string) {
  const start = Math.max(0, contentLength - 1024);
  const response = await openMedia(url, youtubeFetchHeaders(userAgent, `bytes=${start}-${contentLength - 1}`), 0, AbortSignal.timeout(8000));
  if (response.status !== 200 && response.status !== 206) {
    await response.body?.cancel();
    return false;
  }
  const reader = response.body?.getReader();
  if (!reader) return false;
  const first = await reader.read();
  await reader.cancel();
  return Boolean(first.value && first.value.byteLength > 0);
}

async function openMedia(url: string, headers: Headers, hops = 0, signal?: AbortSignal): Promise<Response> {
  if (!isAllowedMediaHost(url)) throw new Error("The audio host was not recognized.");
  const response = await fetch(url, { headers, redirect: "manual", signal });
  if ([301, 302, 303, 307, 308].includes(response.status)) {
    const location = response.headers.get("location");
    if (!location || hops > 3) throw new Error("The audio host was not recognized.");
    return openMedia(new URL(location, url).toString(), headers, hops + 1, signal);
  }
  return response;
}

export function dropCachedAudio(videoId: string) {
  streamCache.delete(videoId);
}
