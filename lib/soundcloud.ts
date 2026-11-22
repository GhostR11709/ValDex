import { Readable } from "node:stream";
import { Soundcloud, type SoundcloudPlaylist, type SoundcloudTrack, type SoundcloudUser } from "soundcloud.ts";
import { songCard } from "@/lib/catalog";
import { installPublicDns } from "@/lib/public-dns";
import { isAllowedExternalHost } from "@/lib/public-media";
import type { CardItem, Collection, HomePayload, Track } from "@/lib/types";

const MEDIA_TTL = 4 * 60 * 1000;
const UA = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

type SelectionItem = {
  id?: number;
  urn?: string;
  title?: string;
  artwork_url?: string | null;
  is_album?: boolean;
  set_type?: string;
  user?: { username?: string; avatar_url?: string | null };
};

type Selection = { title?: string; items?: { collection?: SelectionItem[] } };

type ResolvedMedia =
  | { kind: "file"; url: string }
  | { kind: "hls"; target: number; map?: string; segments: { duration: string; url: string }[] };

let client: Soundcloud | null = null;
const mediaCache = new Map<string, { at: number; media: ResolvedMedia }>();
const mediaPending = new Map<string, Promise<ResolvedMedia>>();

function soundcloud() {
  installPublicDns();
  if (!client) client = new Soundcloud();
  return client;
}

function failed(error: unknown): never {
  const message = error instanceof Error ? error.message : "";
  if (/isn't available|isn't offering|doesn't look right/.test(message)) throw error;
  throw new Error("SoundCloud didn't answer.");
}

function artwork(url?: string | null) {
  if (!url) return "";
  const sized = url.replace("-large.", "-t500x500.").replace("-small.", "-t500x500.").replace("-tiny.", "-t500x500.");
  try {
    const host = new URL(sized).hostname.toLowerCase();
    if (host !== "sndcdn.com" && !host.endsWith(".sndcdn.com")) return "";
  } catch {
    return "";
  }
  return `/api/artwork?url=${encodeURIComponent(sized)}`;
}

function mapTrack(item: Partial<SoundcloudTrack>, album?: { name: string; id?: string }): Track | null {
  if (item.id == null || !item.title || item.policy === "BLOCK") return null;
  const duration = typeof item.duration === "number" && item.duration >= 500 ? Math.max(1, Math.round(item.duration / 1000)) : undefined;
  return {
    id: String(item.id),
    source: "soundcloud",
    title: item.title,
    artists: [{ name: item.user?.username || "SoundCloud", id: item.user?.id != null ? String(item.user.id) : undefined }],
    album: album || (item.publisher_metadata?.album_title ? { name: item.publisher_metadata.album_title } : undefined),
    duration,
    thumbnail: artwork(item.artwork_url || item.user?.avatar_url),
  };
}

function playlistCard(item: SelectionItem | SoundcloudPlaylist): CardItem | null {
  if (item.id == null || !item.title) return null;
  const album = Boolean(item.is_album) || item.set_type === "album" || item.set_type === "ep";
  const user = "user" in item ? item.user : undefined;
  return {
    kind: album ? "album" : "playlist",
    id: String(item.id),
    source: "soundcloud",
    title: item.title,
    subtitle: user?.username || "SoundCloud",
    thumbnail: artwork(item.artwork_url || user?.avatar_url),
  };
}

async function listed(tracks: Partial<SoundcloudTrack>[], album?: { name: string; id: string }) {
  const sc = soundcloud();
  const ready = tracks.filter((track) => track.title);
  const missing = tracks.filter((track) => track.id != null && !track.title).slice(0, 40);
  let hydrated: SoundcloudTrack[] = [];
  if (missing.length) {
    try {
      hydrated = await sc.tracks.getArray(missing.map((track) => Number(track.id)));
    } catch {
      hydrated = [];
    }
  }
  return [...ready, ...hydrated]
    .map((track) => mapTrack(track, album))
    .filter((track): track is Track => Boolean(track))
    .slice(0, 50);
}

export async function getSoundCloudHome(mood?: string): Promise<HomePayload> {
  const sc = soundcloud();
  let charts: { collection?: { track?: SoundcloudTrack }[] } | null = null;
  let mixed: { collection?: Selection[] } | null = null;
  try {
    [charts, mixed] = await Promise.all([
      sc.api.getV2("charts", { kind: "trending", genre: "soundcloud:genres:all-music", limit: 24 }).catch(() => null),
      sc.api.getV2("mixed-selections", { limit: 8 }).catch(() => null),
    ]);
  } catch (error) {
    failed(error);
  }
  const selections = mixed?.collection || [];
  const genres = selections.flatMap((selection) => selection.items?.collection || []).filter((item) => item.urn?.includes(":system-playlists:") && item.title);
  const filters = genres.map((item) => item.title || "");
  const chosen = mood ? genres.find((item) => item.title?.toLowerCase() === mood.toLowerCase()) : undefined;
  if (chosen?.urn) {
    const playlist = await getSoundCloudPlaylist(chosen.urn);
    return { filters, shelves: playlist.tracks.length ? [{ title: playlist.title, items: playlist.tracks.map(songCard) }] : [] };
  }
  const trending = (charts?.collection || []).map((item) => mapTrack(item.track || {})).filter((track): track is Track => Boolean(track));
  const shelves = trending.length ? [{ title: "Trending on SoundCloud", items: trending.map(songCard) }] : [];
  for (const selection of selections) {
    const cards = (selection.items?.collection || [])
      .filter((item) => item.id != null && item.title && !item.urn?.includes(":system-playlists:"))
      .map((item) => playlistCard(item))
      .filter((card): card is CardItem => Boolean(card))
      .slice(0, 12);
    if (selection.title && cards.length) shelves.push({ title: selection.title, items: cards });
    if (shelves.length >= 4) break;
  }
  if (!shelves.length && !charts && !mixed) throw new Error("SoundCloud didn't answer.");
  return { filters, shelves };
}

export async function searchSoundCloud(query: string) {
  const sc = soundcloud();
  try {
    const [tracks, users, playlists] = await Promise.all([
      sc.tracks.search({ q: query, limit: 24 }),
      sc.users.search({ q: query, limit: 8 }).catch(() => ({ collection: [] as SoundcloudUser[] })),
      sc.playlists.search({ q: query, limit: 12 }).catch(() => ({ collection: [] as SoundcloudPlaylist[] })),
    ]);
    const albums: CardItem[] = [];
    const playlistCards: CardItem[] = [];
    for (const item of playlists.collection || []) {
      const card = playlistCard(item);
      if (!card) continue;
      if (item.is_album || item.set_type === "album" || item.set_type === "ep") albums.push({ ...card, kind: "album" });
      else playlistCards.push(card);
    }
    return {
      songs: (tracks.collection || []).map((track) => mapTrack(track)).filter((track): track is Track => Boolean(track)),
      videos: [],
      albums,
      artists: (users.collection || [])
        .filter((user) => user.id != null && user.username)
        .map((user) => ({
          kind: "artist" as const,
          id: String(user.id),
          source: "soundcloud" as const,
          title: user.username,
          subtitle: "Artist on SoundCloud",
          thumbnail: artwork(user.avatar_url),
        })),
      playlists: playlistCards,
    };
  } catch (error) {
    failed(error);
  }
}

export async function getSoundCloudArtist(id: string): Promise<Collection> {
  const sc = soundcloud();
  try {
    const user = await sc.users.get(id);
    if (user?.id == null || !user.username) throw new Error("That SoundCloud artist isn't available.");
    const page = (await sc.api.getV2(`users/${user.id}/tracks`, { limit: 50 })) as { collection?: SoundcloudTrack[] };
    const tracks = await listed(page.collection || []);
    return {
      kind: "artist",
      id: String(user.id),
      title: user.username,
      subtitle: "SoundCloud",
      thumbnail: artwork(user.avatar_url),
      description: user.description ? user.description.slice(0, 400) : undefined,
      tracks,
    };
  } catch (error) {
    failed(error);
  }
}

export async function getSoundCloudPlaylist(id: string): Promise<Collection> {
  const sc = soundcloud();
  try {
    const path = /^\d{1,20}$/.test(id) ? `playlists/${id}` : `system-playlists/${encodeURIComponent(id)}`;
    const playlist = (await sc.api.getV2(path)) as Partial<SoundcloudPlaylist> & { tracks?: Partial<SoundcloudTrack>[] };
    if (!playlist.title) throw new Error("That SoundCloud playlist isn't available.");
    const tracks = await listed(playlist.tracks || [], { name: playlist.title, id });
    const album = Boolean(playlist.is_album) || playlist.set_type === "album" || playlist.set_type === "ep";
    return {
      kind: album ? "album" : "playlist",
      id,
      title: playlist.title,
      subtitle: playlist.user?.username ? `${playlist.user.username} · SoundCloud` : "SoundCloud",
      thumbnail: artwork(playlist.artwork_url || playlist.user?.avatar_url),
      description: playlist.description ? playlist.description.slice(0, 400) : undefined,
      tracks,
    };
  } catch (error) {
    failed(error);
  }
}

function assertMedia(url: string) {
  if (!isAllowedExternalHost(url)) throw new Error("The audio host was not recognized.");
}

async function fetchMedia(url: string, headers: HeadersInit) {
  try {
    return await fetch(url, { headers, redirect: "manual" });
  } catch (error) {
    const cause = error instanceof Error && error.cause && typeof error.cause === "object" ? error.cause : null;
    const code = cause && "code" in cause ? String(cause.code) : "";
    if (!/TIMEOUT|ECONNRESET|ECONNREFUSED|ENOTFOUND|UND_ERR/.test(code)) throw error;
    return fetch(url, { headers, redirect: "manual" });
  }
}

async function openAllowed(url: string, headers: HeadersInit, hops = 0): Promise<Response> {
  assertMedia(url);
  installPublicDns();
  const response = await fetchMedia(url, headers);
  if ([301, 302, 303, 307, 308].includes(response.status)) {
    const location = response.headers.get("location");
    if (!location || hops > 4) throw new Error("The audio host was not recognized.");
    return openAllowed(new URL(location, url).toString(), headers, hops + 1);
  }
  return response;
}

function parseMediaPlaylist(text: string, base: string) {
  let map: string | undefined;
  let target = 10;
  let duration = "10";
  const segments: { duration: string; url: string }[] = [];
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (trimmed.startsWith("#EXT-X-TARGETDURATION:")) target = Number(trimmed.split(":")[1]) || 10;
    else if (trimmed.startsWith("#EXT-X-MAP:")) {
      const uri = /URI="([^"]+)"/.exec(trimmed)?.[1];
      if (uri) map = new URL(uri, base).toString();
    } else if (trimmed.startsWith("#EXTINF:")) duration = trimmed.slice("#EXTINF:".length).replace(/,$/, "") || duration;
    else if (trimmed && !trimmed.startsWith("#")) segments.push({ duration, url: new URL(trimmed, base).toString() });
  }
  return { map, target: Math.max(1, Math.round(target)), segments };
}

function highestVariant(text: string, base: string) {
  const lines = text.split(/\r?\n/);
  let best = -1;
  let uri = "";
  for (let index = 0; index < lines.length; index += 1) {
    if (!lines[index].startsWith("#EXT-X-STREAM-INF")) continue;
    const bandwidth = Number(/BANDWIDTH=(\d+)/.exec(lines[index])?.[1] || 0);
    const next = lines[index + 1]?.trim();
    if (next && !next.startsWith("#") && bandwidth >= best) {
      best = bandwidth;
      uri = new URL(next, base).toString();
    }
  }
  return uri;
}

async function readPlaylist(url: string) {
  const response = await openAllowed(url, { "user-agent": UA, accept: "application/vnd.apple.mpegurl,*/*" });
  if (!response.ok) throw new Error("SoundCloud isn't offering a file for this song.");
  return response.text();
}

async function resolveTranscoding(sc: Soundcloud, transcodingUrl: string) {
  const clientId = await sc.api.getClientId();
  const separator = transcodingUrl.includes("?") ? "&" : "?";
  const response = await fetch(`${transcodingUrl}${separator}client_id=${encodeURIComponent(clientId)}`, { headers: sc.api.headers });
  if (!response.ok) return "";
  const body = (await response.json()) as { url?: string };
  return body.url || "";
}

async function buildMedia(id: string): Promise<ResolvedMedia> {
  const sc = soundcloud();
  const track = await sc.tracks.get(id);
  if (!track?.id) throw new Error("That SoundCloud song isn't available.");
  const transcodings = (track.media?.transcodings || []).filter((item) => item?.url && item.format?.protocol);
  const full = transcodings.filter((item) => !item.snipped);
  const chosen =
    full.find((item) => item.format.protocol === "progressive") ||
    full.find((item) => item.format.protocol === "hls") ||
    transcodings.find((item) => item.format.protocol === "hls");
  if (!chosen) throw new Error("SoundCloud isn't offering a file for this song.");
  const mediaUrl = await resolveTranscoding(sc, chosen.url);
  if (!mediaUrl) throw new Error("SoundCloud isn't offering a file for this song.");
  assertMedia(mediaUrl);
  if (chosen.format.protocol === "progressive") return { kind: "file", url: mediaUrl };
  const playlistUrl = mediaUrl;
  let text = await readPlaylist(playlistUrl);
  let base = playlistUrl;
  if (text.includes("#EXT-X-STREAM-INF")) {
    base = highestVariant(text, playlistUrl);
    if (!base) throw new Error("SoundCloud isn't offering a file for this song.");
    assertMedia(base);
    text = await readPlaylist(base);
  }
  const parsed = parseMediaPlaylist(text, base);
  if (!parsed.segments.length) throw new Error("SoundCloud isn't offering a file for this song.");
  if (parsed.map) assertMedia(parsed.map);
  for (const segment of parsed.segments) assertMedia(segment.url);
  return { kind: "hls", ...parsed };
}

function resolveMedia(id: string) {
  const cached = mediaCache.get(id);
  if (cached && Date.now() - cached.at < MEDIA_TTL) return Promise.resolve(cached.media);
  const pending = mediaPending.get(id);
  if (pending) return pending;
  const promise = buildMedia(id)
    .then((media) => {
      mediaCache.set(id, { at: Date.now(), media });
      return media;
    })
    .finally(() => mediaPending.delete(id));
  mediaPending.set(id, promise);
  return promise;
}

export async function soundcloudPlaybackMode(id: string) {
  const media = await resolveMedia(id);
  return media.kind === "file" ? ("audio" as const) : ("hls" as const);
}

export async function soundcloudPlaylist(id: string) {
  const media = await resolveMedia(id);
  if (media.kind !== "hls") throw new Error("SoundCloud isn't offering a file for this song.");
  const base = `/api/stream?source=soundcloud&id=${encodeURIComponent(id)}`;
  const lines = ["#EXTM3U", "#EXT-X-VERSION:7", `#EXT-X-TARGETDURATION:${media.target}`, "#EXT-X-MEDIA-SEQUENCE:0", "#EXT-X-PLAYLIST-TYPE:VOD"];
  if (media.map) lines.push(`#EXT-X-MAP:URI="${base}&part=map"`);
  media.segments.forEach((segment, index) => {
    lines.push(`#EXTINF:${segment.duration},`, `${base}&part=${index}`);
  });
  lines.push("#EXT-X-ENDLIST");
  return `${lines.join("\n")}\n`;
}

async function partUrl(id: string, part: string) {
  const media = await resolveMedia(id);
  if (media.kind === "file") return part ? "" : media.url;
  if (!part) return "";
  if (part === "map") return media.map || "";
  if (!/^\d{1,4}$/.test(part)) return "";
  return media.segments[Number(part)]?.url || "";
}

export async function openSoundcloudAudio(id: string, part: string | null, range: string | null) {
  const url = await partUrl(id, part || "");
  if (!url) throw new Error("That recording isn't available right now.");
  const headers: Record<string, string> = { "user-agent": UA, "accept-encoding": "identity" };
  if (range) headers.range = range;
  const upstream = await openAllowed(url, headers);
  if (upstream.status >= 400 || !upstream.body) throw new Error("That recording isn't available right now.");
  return upstream;
}

export async function soundcloudPack(id: string) {
  const media = await resolveMedia(id);
  const urls = media.kind === "file" ? [media.url] : [media.map, ...media.segments.map((segment) => segment.url)].filter((url): url is string => Boolean(url));
  async function* chunks() {
    for (const url of urls) {
      const upstream = await openAllowed(url, { "user-agent": UA, "accept-encoding": "identity" });
      if (!upstream.ok || !upstream.body) throw new Error("SoundCloud isn't offering a file for this song.");
      const reader = upstream.body.getReader();
      while (true) {
        const next = await reader.read();
        if (next.done) break;
        if (next.value) yield next.value;
      }
    }
  }
  return Readable.from(chunks());
}
