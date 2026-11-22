import { Readable } from "node:stream";
import { connection } from "next/server";
import type { NextRequest } from "next/server";
import { archiveFileUrl } from "@/lib/archive";
import { audiusStreamUrl } from "@/lib/audius";
import { parseSource } from "@/lib/catalog";
import { jsonError } from "@/lib/http";
import { fetchExternalAudio } from "@/lib/public-media";
import { openSoundcloudAudio, soundcloudPack, soundcloudPlaybackMode, soundcloudPlaylist } from "@/lib/soundcloud";
import { dropCachedAudio, isAllowedMediaHost, resolveAudio, youtubeFetchHeaders } from "@/lib/youtube";

export async function GET(request: NextRequest) {
  try {
    await connection();
    const id = request.nextUrl.searchParams.get("id") || "";
    const source = parseSource(request.nextUrl.searchParams.get("source"));
    if (source === "soundcloud") return proxySoundcloud(id, request);
    if (source === "audius" || source === "archive") return proxyExternal(source, id, request);

    if (!/^[\w-]{11}$/.test(id)) return jsonError(new Error("Missing a song id."), 400);

    let audio = await resolveAudio(id);
    if (!audio) return jsonError(new Error("Direct audio isn't available on this network."), 404);

    const range = request.headers.get("range");
    const headers = youtubeFetchHeaders(audio.userAgent, range || (audio.contentLength > 1 ? `bytes=0-${audio.contentLength - 1}` : null));

    let upstream = await fetchAllowed(audio.url, headers);
    if (upstream.status === 403 || upstream.status === 410) {
      dropCachedAudio(id);
      audio = await resolveAudio(id);
      if (!audio) return jsonError(new Error("The audio link expired."), 404);
      const retryRange = range || (audio.contentLength > 1 ? `bytes=0-${audio.contentLength - 1}` : null);
      upstream = await fetchAllowed(audio.url, youtubeFetchHeaders(audio.userAgent, retryRange));
    }
    if (!upstream.body) return jsonError(new Error("The audio stream was empty."));

    const out = new Headers();
    out.set("content-type", upstream.headers.get("content-type") || audio.mime);
    out.set("cache-control", "no-store");
    out.set("accept-ranges", "bytes");
    const wholeFile = !range && upstream.status === 206;
    if (wholeFile) {
      const total = upstream.headers.get("content-range")?.split("/").pop();
      if (total && total !== "*") out.set("content-length", total);
      return new Response(upstream.body, { status: 200, headers: out });
    }
    for (const name of ["content-length", "content-range"]) {
      const value = upstream.headers.get(name);
      if (value) out.set(name, value);
    }
    return new Response(upstream.body, { status: upstream.status, headers: out });
  } catch (error) {
    return jsonError(error);
  }
}

async function proxySoundcloud(id: string, request: NextRequest) {
  if (!/^\d{1,20}$/.test(id)) return jsonError(new Error("Missing a song id."), 400);
  const part = request.nextUrl.searchParams.get("part");
  const pack = request.nextUrl.searchParams.get("pack") === "1";
  if (pack) {
    const stream = await soundcloudPack(id);
    return new Response(Readable.toWeb(stream) as ReadableStream, {
      headers: { "content-type": "audio/mp4", "cache-control": "no-store" },
    });
  }
  if (!part && (await soundcloudPlaybackMode(id)) === "hls") {
    const playlist = await soundcloudPlaylist(id);
    return new Response(playlist, {
      headers: { "content-type": "application/vnd.apple.mpegurl", "cache-control": "no-store" },
    });
  }
  const upstream = await openSoundcloudAudio(id, part, request.headers.get("range"));
  const headers = new Headers();
  headers.set("content-type", upstream.headers.get("content-type") || "audio/mp4");
  headers.set("cache-control", "no-store");
  headers.set("accept-ranges", upstream.headers.get("accept-ranges") || "bytes");
  for (const name of ["content-length", "content-range"]) {
    const value = upstream.headers.get(name);
    if (value) headers.set(name, value);
  }
  return new Response(upstream.body, { status: upstream.status, headers });
}

async function proxyExternal(source: "audius" | "archive", id: string, request: NextRequest) {
  const headers = new Headers();
  const range = request.headers.get("range");
  if (range) headers.set("range", range);
  headers.set("user-agent", "Mozilla/5.0");
  const url = source === "audius" ? audiusStreamUrl(id) : archiveFileUrl(id);
  const upstream = await fetchExternalAudio(url, headers);
  if (upstream.status >= 400 || !upstream.body) {
    return jsonError(new Error("That recording isn't available right now."), upstream.status === 404 ? 404 : 502);
  }
  const out = new Headers();
  out.set("content-type", upstream.headers.get("content-type") || "audio/mpeg");
  out.set("cache-control", "no-store");
  out.set("accept-ranges", upstream.headers.get("accept-ranges") || "bytes");
  for (const name of ["content-length", "content-range"]) {
    const value = upstream.headers.get(name);
    if (value) out.set(name, value);
  }
  return new Response(upstream.body, { status: upstream.status, headers: out });
}

async function fetchAllowed(url: string, headers: Headers, hops = 0): Promise<Response> {
  if (!isAllowedMediaHost(url)) throw new Error("The audio host was not recognized.");
  const response = await fetch(url, { headers, redirect: "manual" });
  if ([301, 302, 303, 307, 308].includes(response.status)) {
    const location = response.headers.get("location");
    if (!location || hops > 3) throw new Error("The audio host was not recognized.");
    return fetchAllowed(new URL(location, url).toString(), headers, hops + 1);
  }
  return response;
}
