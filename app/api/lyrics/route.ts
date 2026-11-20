import type { NextRequest } from "next/server";
import { connection } from "next/server";
import { jsonError } from "@/lib/http";
import { parseLrc } from "@/lib/lyrics";
import { getPlainLyrics } from "@/lib/youtube";
import type { LyricsPayload } from "@/lib/types";

type LrcRecord = {
  trackName?: string;
  artistName?: string;
  duration?: number;
  instrumental?: boolean;
  plainLyrics?: string | null;
  syncedLyrics?: string | null;
};

export async function GET(request: NextRequest) {
  try {
    await connection();
    const title = request.nextUrl.searchParams.get("title")?.trim().slice(0, 160) || "";
    const artist = request.nextUrl.searchParams.get("artist")?.trim().slice(0, 160) || "";
    const album = request.nextUrl.searchParams.get("album")?.trim().slice(0, 160) || "";
    const videoId = request.nextUrl.searchParams.get("id") || "";
    const duration = Number(request.nextUrl.searchParams.get("duration") || "");
    if (!title) return jsonError(new Error("Missing a song title."), 400);

    const lrclib = await fromLrcLib({ title, artist, album, duration: Number.isFinite(duration) ? duration : undefined });
    if (lrclib) {
      return Response.json(lrclib, { headers: { "cache-control": "no-store" } });
    }

    if (/^[\w-]{11}$/.test(videoId)) {
      const plain = await getPlainLyrics(videoId);
      if (plain?.plain) {
        const payload: LyricsPayload = {
          lines: plain.plain.split(/\n+/).map((text) => ({ time: -1, text: text.trim() })).filter((line) => line.text),
          plain: plain.plain,
          source: plain.source,
          synced: false,
        };
        return Response.json(payload, { headers: { "cache-control": "no-store" } });
      }
    }

    const empty: LyricsPayload = { lines: [], plain: "", source: "", synced: false };
    return Response.json(empty, { headers: { "cache-control": "no-store" } });
  } catch (error) {
    return jsonError(error);
  }
}

async function fromLrcLib(input: { title: string; artist: string; album: string; duration?: number }): Promise<LyricsPayload | null> {
  const headers = { "user-agent": "ViviForWindows/1.0", accept: "application/json" };
  const exact = new URL("https://lrclib.net/api/get");
  exact.searchParams.set("track_name", input.title);
  exact.searchParams.set("artist_name", input.artist);
  if (input.album) exact.searchParams.set("album_name", input.album);
  if (input.duration) exact.searchParams.set("duration", String(Math.round(input.duration)));

  let record: LrcRecord | null = null;
  const exactResponse = await fetch(exact, { headers });
  if (exactResponse.ok) record = (await exactResponse.json()) as LrcRecord;
  if (!record) {
    const search = new URL("https://lrclib.net/api/search");
    search.searchParams.set("track_name", input.title);
    if (input.artist) search.searchParams.set("artist_name", input.artist);
    const searchResponse = await fetch(search, { headers });
    if (searchResponse.ok) {
      const rows = (await searchResponse.json()) as LrcRecord[];
      record = pick(rows, input.duration);
    }
  }
  if (!record || record.instrumental) return record?.instrumental ? { lines: [], plain: "", source: "Marked instrumental on LRCLIB", synced: false } : null;
  if (record.syncedLyrics) {
    const lines = parseLrc(record.syncedLyrics);
    if (lines.length) {
      return { lines, plain: record.plainLyrics || lines.map((line) => line.text).join("\n"), source: "LRCLIB", synced: true };
    }
  }
  if (record.plainLyrics) {
    const lines = record.plainLyrics.split(/\n+/).map((text) => ({ time: -1, text: text.trim() })).filter((line) => line.text);
    return { lines, plain: record.plainLyrics, source: "LRCLIB", synced: false };
  }
  return null;
}

function pick(rows: LrcRecord[], duration?: number) {
  const usable = rows.filter((row) => row.syncedLyrics || row.plainLyrics);
  if (!usable.length) return null;
  if (!duration) return usable.find((row) => row.syncedLyrics) || usable[0];
  return usable
    .slice()
    .sort((a, b) => Math.abs((a.duration || 0) - duration) - Math.abs((b.duration || 0) - duration))[0];
}
