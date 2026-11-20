import { connection } from "next/server";
import type { NextRequest } from "next/server";
import { jsonError } from "@/lib/http";
import { dropCachedAudio, isAllowedMediaHost, resolveAudio } from "@/lib/youtube";

export async function GET(request: NextRequest) {
  try {
    await connection();
    const id = request.nextUrl.searchParams.get("id") || "";
    if (!/^[\w-]{11}$/.test(id)) return jsonError(new Error("Missing a song id."), 400);

    let audio = await resolveAudio(id);
    if (!audio) return jsonError(new Error("Direct audio isn't available on this network."), 404);

    const headers = new Headers();
    const range = request.headers.get("range");
    if (range) headers.set("range", range);
    headers.set("user-agent", "Mozilla/5.0");

    let upstream = await fetchAllowed(audio.url, headers);
    if (upstream.status === 403 || upstream.status === 410) {
      dropCachedAudio(id);
      audio = await resolveAudio(id);
      if (!audio) return jsonError(new Error("The audio link expired."), 404);
      upstream = await fetchAllowed(audio.url, headers);
    }
    if (!upstream.body) return jsonError(new Error("The audio stream was empty."));

    const out = new Headers();
    out.set("content-type", upstream.headers.get("content-type") || audio.mime);
    out.set("cache-control", "no-store");
    out.set("accept-ranges", upstream.headers.get("accept-ranges") || "bytes");
    for (const name of ["content-length", "content-range"]) {
      const value = upstream.headers.get(name);
      if (value) out.set(name, value);
    }
    return new Response(upstream.body, { status: upstream.status, headers: out });
  } catch (error) {
    return jsonError(error);
  }
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
