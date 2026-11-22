import { songCard } from "@/lib/catalog";
import type { Collection, HomePayload, Track } from "@/lib/types";

const API = "https://api.audius.co";
const APP = "vivi-windows";

const GENRES = ["Electronic", "Hip-Hop/Rap", "Pop", "Rock", "Jazz", "Ambient", "Experimental", "Classical"];

type AudiusTrack = {
  id?: string;
  title?: string;
  duration?: number;
  artwork?: Record<string, unknown>;
  user?: { id?: string; name?: string; handle?: string };
};

type AudiusUser = {
  id?: string;
  name?: string;
  handle?: string;
  bio?: string;
  profile_picture?: Record<string, unknown>;
  track_count?: number;
};

function picture(value: unknown) {
  if (!value || typeof value !== "object") return "";
  const record = value as Record<string, unknown>;
  for (const key of ["480x480", "150x150", "1000x1000"]) {
    if (typeof record[key] === "string" && record[key]) return record[key];
  }
  return "";
}

function mapAudiusTrack(item: AudiusTrack): Track | null {
  if (!item.id || !item.title) return null;
  const artist = item.user?.name || item.user?.handle || "Audius";
  return {
    id: item.id,
    source: "audius",
    title: item.title,
    artists: [{ name: artist, id: item.user?.id }],
    duration: typeof item.duration === "number" && item.duration > 0 ? item.duration : undefined,
    thumbnail: picture(item.artwork),
  };
}

async function audius<T>(path: string): Promise<T> {
  const url = new URL(path, API);
  if (!url.searchParams.has("app_name")) url.searchParams.set("app_name", APP);
  const response = await fetch(url, { headers: { accept: "application/json", "user-agent": "ViviForWindows/1.0" } });
  if (!response.ok) throw new Error("Audius didn't answer.");
  return (await response.json()) as T;
}

export async function getAudiusHome(genre?: string): Promise<HomePayload> {
  const chosen = genre && GENRES.includes(genre) ? genre : "";
  const path = `/v1/tracks/trending?limit=24${chosen ? `&genre=${encodeURIComponent(chosen)}` : ""}`;
  const body = await audius<{ data?: AudiusTrack[] }>(path);
  const tracks = (body.data || []).map(mapAudiusTrack).filter((track): track is Track => Boolean(track));
  return {
    filters: GENRES,
    shelves: tracks.length ? [{ title: chosen || "Trending on Audius", items: tracks.map(songCard) }] : [],
  };
}

export async function searchAudius(query: string) {
  const body = await audius<{ data?: AudiusTrack[] }>(`/v1/tracks/search?query=${encodeURIComponent(query)}&limit=24`);
  const tracks = (body.data || []).map(mapAudiusTrack).filter((track): track is Track => Boolean(track));
  const artists = new Map<string, { id: string; name: string; thumbnail: string }>();
  for (const item of body.data || []) {
    const id = item.user?.id;
    const name = item.user?.name || item.user?.handle;
    if (!id || !name || artists.has(id)) continue;
    artists.set(id, { id, name, thumbnail: picture(item.artwork) });
  }
  return {
    songs: tracks,
    videos: [],
    albums: [],
    artists: [...artists.values()].map((artist) => ({
      kind: "artist" as const,
      id: artist.id,
      source: "audius" as const,
      title: artist.name,
      subtitle: "Artist on Audius",
      thumbnail: artist.thumbnail,
    })),
    playlists: [],
  };
}

export async function getAudiusArtist(id: string): Promise<Collection> {
  const [userBody, trackBody] = await Promise.all([
    audius<{ data?: AudiusUser }>(`/v1/users/${encodeURIComponent(id)}`),
    audius<{ data?: AudiusTrack[] }>(`/v1/users/${encodeURIComponent(id)}/tracks?limit=50`),
  ]);
  const user = userBody.data;
  if (!user?.id) throw new Error("That Audius artist isn't available.");
  const name = user.name || user.handle || "Artist";
  const tracks = (trackBody.data || []).map(mapAudiusTrack).filter((track): track is Track => Boolean(track));
  return {
    kind: "artist",
    id: user.id,
    title: name,
    subtitle: user.handle ? `@${user.handle} · Audius` : "Audius",
    thumbnail: picture(user.profile_picture),
    description: typeof user.bio === "string" ? user.bio.slice(0, 400) : undefined,
    tracks,
  };
}

export function audiusStreamUrl(id: string) {
  if (!/^[A-Za-z0-9]{4,32}$/.test(id)) throw new Error("That song link doesn't look right.");
  return `${API}/v1/tracks/${id}/stream?app_name=${APP}`;
}
