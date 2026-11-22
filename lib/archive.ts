import type { CardItem, Collection, HomePayload, Track } from "@/lib/types";

const IDENTIFIER = /^[A-Za-z0-9][A-Za-z0-9._-]{0,119}$/;
const AUDIO = /\.(mp3|ogg|flac|wav|m4a)$/i;

type ArchiveFile = {
  name?: string;
  format?: string;
  title?: string;
  length?: string;
  size?: string;
};

type ArchiveDoc = {
  identifier?: string;
  title?: unknown;
  creator?: unknown;
  description?: unknown;
};

type ArchiveMeta = {
  metadata?: ArchiveDoc;
  files?: ArchiveFile[];
};

const metaCache = new Map<string, { at: number; meta: ArchiveMeta }>();

function text(value: unknown) {
  if (typeof value === "string") return value.trim();
  if (Array.isArray(value)) return text(value[0]);
  return "";
}

function plain(value: string) {
  return value.replace(/<[^>]+>/g, " ").replace(/\s+/g, " ").trim().slice(0, 400);
}

function seconds(value?: string) {
  if (!value) return undefined;
  if (/^\d+(\.\d+)?$/.test(value)) {
    const parsed = Number(value);
    return parsed > 0 ? Math.round(parsed) : undefined;
  }
  const clock = /^(\d+):(\d{2})(?::(\d{2}))?$/.exec(value);
  if (!clock) return undefined;
  if (clock[3]) return Number(clock[1]) * 3600 + Number(clock[2]) * 60 + Number(clock[3]);
  return Number(clock[1]) * 60 + Number(clock[2]);
}

export function archiveStreamId(identifier: string, file: string) {
  return `${identifier}~${Buffer.from(file, "utf8").toString("base64url")}`;
}

export function parseArchiveStreamId(id: string) {
  const split = id.indexOf("~");
  if (split < 1) throw new Error("That recording doesn't look right.");
  const identifier = id.slice(0, split);
  const file = Buffer.from(id.slice(split + 1), "base64url").toString("utf8");
  if (!IDENTIFIER.test(identifier) || !file || file.includes("..") || file.includes("\\") || file.startsWith("/")) {
    throw new Error("That recording doesn't look right.");
  }
  return { identifier, file };
}

function audioFiles(meta: ArchiveMeta) {
  return (meta.files || []).filter((file) => file.name && AUDIO.test(file.name) && !file.name.includes(".."));
}

async function metadata(identifier: string) {
  if (!IDENTIFIER.test(identifier)) throw new Error("That recording doesn't look right.");
  const cached = metaCache.get(identifier);
  if (cached && Date.now() - cached.at < 10 * 60 * 1000) return cached.meta;
  const response = await fetch(`https://archive.org/metadata/${encodeURIComponent(identifier)}`, {
    headers: { accept: "application/json", "user-agent": "ViviForWindows/1.0" },
  });
  if (!response.ok) throw new Error("The Internet Archive didn't answer.");
  const meta = (await response.json()) as ArchiveMeta;
  if (!meta.metadata?.identifier) throw new Error("That recording isn't on the Internet Archive.");
  metaCache.set(identifier, { at: Date.now(), meta });
  return meta;
}

function itemCard(doc: ArchiveDoc): CardItem | null {
  const id = doc.identifier || "";
  const title = text(doc.title);
  if (!IDENTIFIER.test(id) || !title) return null;
  const creator = text(doc.creator);
  return {
    kind: "album",
    id,
    source: "archive",
    title,
    subtitle: creator || "Internet Archive",
    thumbnail: `https://archive.org/services/img/${encodeURIComponent(id)}`,
  };
}

async function searchDocs(query: string, rows: number) {
  const url = new URL("https://archive.org/advancedsearch.php");
  url.searchParams.set("q", query);
  url.searchParams.append("fl[]", "identifier");
  url.searchParams.append("fl[]", "title");
  url.searchParams.append("fl[]", "creator");
  url.searchParams.set("rows", String(rows));
  url.searchParams.set("output", "json");
  url.searchParams.append("sort[]", "downloads desc");
  const response = await fetch(url, { headers: { accept: "application/json", "user-agent": "ViviForWindows/1.0" } });
  if (!response.ok) throw new Error("The Internet Archive didn't answer.");
  const body = (await response.json()) as { response?: { docs?: ArchiveDoc[] } };
  return (body.response?.docs || []).map(itemCard).filter((card): card is CardItem => Boolean(card));
}

function safeTerms(query: string) {
  return query.replace(/[+\-!()[\]{}^"~*?:\\/&|]/g, " ").replace(/\s+/g, " ").trim().slice(0, 80);
}

export async function getArchiveHome(): Promise<HomePayload> {
  const [netlabels, live] = await Promise.all([
    searchDocs("collection:netlabels AND mediatype:audio AND format:MP3", 16),
    searchDocs("collection:78rpm AND mediatype:audio AND format:MP3", 16),
  ]);
  const shelves = [
    netlabels.length ? { title: "Netlabels", items: netlabels } : null,
    live.length ? { title: "78 rpm records", items: live } : null,
  ].filter((shelf): shelf is HomePayload["shelves"][number] => Boolean(shelf));
  return { filters: [], shelves };
}

export async function searchArchive(query: string) {
  const terms = safeTerms(query);
  if (!terms) return { songs: [], videos: [], albums: [], artists: [], playlists: [] };
  const albums = await searchDocs(`(${terms}) AND mediatype:audio AND format:MP3`, 24);
  return { songs: [], videos: [], albums, artists: [], playlists: [] };
}

export async function getArchiveAlbum(id: string): Promise<Collection> {
  const meta = await metadata(id);
  const info = meta.metadata || {};
  const creator = text(info.creator) || "Internet Archive";
  const title = text(info.title) || id;
  const thumbnail = `https://archive.org/services/img/${encodeURIComponent(id)}`;
  const tracks: Track[] = audioFiles(meta).map((file) => ({
    id: archiveStreamId(id, file.name || ""),
    source: "archive",
    title: file.title || (file.name || "").replace(/\.[^.]+$/, "").replace(/[_-]+/g, " "),
    artists: [{ name: creator, id: creator }],
    album: { name: title, id },
    duration: seconds(file.length),
    thumbnail,
  }));
  if (!tracks.length) throw new Error("That page has no audio files.");
  return {
    kind: "album",
    id,
    title,
    subtitle: `${creator} · Internet Archive`,
    thumbnail,
    description: plain(text(info.description)),
    tracks,
  };
}

export async function getArchiveCreator(name: string): Promise<Collection> {
  const clean = name.trim().slice(0, 140);
  if (!clean) throw new Error("That artist link doesn't look right.");
  const albums = await searchDocs(`creator:"${clean.replace(/"/g, "")}" AND mediatype:audio AND format:MP3`, 24);
  return {
    kind: "artist",
    id: clean,
    title: clean,
    subtitle: "Internet Archive",
    thumbnail: albums[0]?.thumbnail || "",
    tracks: [],
    shelves: albums.length ? [{ title: "Recordings", items: albums }] : [],
  };
}

export function archiveFileUrl(id: string) {
  const { identifier, file } = parseArchiveStreamId(id);
  const encoded = file.split("/").map((part) => encodeURIComponent(part)).join("/");
  return `https://archive.org/download/${encodeURIComponent(identifier)}/${encoded}`;
}
