import type { NextRequest } from "next/server";
import { parseSource } from "@/lib/catalog";
import { respond } from "@/lib/http";
import { soundcloudPlaybackMode } from "@/lib/soundcloud";
import { resolveAudio } from "@/lib/youtube";

export async function GET(request: NextRequest) {
  const id = request.nextUrl.searchParams.get("id") || "";
  const source = parseSource(request.nextUrl.searchParams.get("source"));
  return respond(async () => {
    if (source === "soundcloud") {
      if (!/^\d{1,20}$/.test(id)) throw new Error("Missing a song id.");
      return { mode: await soundcloudPlaybackMode(id) };
    }
    if (source !== "youtube") return { mode: "audio" as const };
    if (!/^[\w-]{11}$/.test(id)) throw new Error("Missing a song id.");
    const audio = await resolveAudio(id);
    if (!audio) {
      return { mode: "embed" as const, reason: "YouTube asked this network to use the official player." };
    }
    return { mode: "audio" as const, mime: audio.mime };
  });
}
