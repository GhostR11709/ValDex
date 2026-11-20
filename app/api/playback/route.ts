import type { NextRequest } from "next/server";
import { respond } from "@/lib/http";
import { resolveAudio } from "@/lib/youtube";

export async function GET(request: NextRequest) {
  const id = request.nextUrl.searchParams.get("id") || "";
  return respond(async () => {
    if (!/^[\w-]{11}$/.test(id)) throw new Error("Missing a song id.");
    const audio = await resolveAudio(id);
    if (!audio) {
      return { mode: "embed" as const, reason: "YouTube asked this network to use the official player." };
    }
    return { mode: "audio" as const, mime: audio.mime };
  });
}
