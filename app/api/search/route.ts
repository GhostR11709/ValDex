import type { NextRequest } from "next/server";
import { searchArchive } from "@/lib/archive";
import { searchAudius } from "@/lib/audius";
import { parseSource } from "@/lib/catalog";
import { respond } from "@/lib/http";
import { searchSoundCloud } from "@/lib/soundcloud";
import { searchCatalog } from "@/lib/youtube";

export async function GET(request: NextRequest) {
  const query = request.nextUrl.searchParams.get("q")?.trim().slice(0, 120) || "";
  const source = parseSource(request.nextUrl.searchParams.get("source"));
  return respond(() => {
    if (!query) throw new Error("Type a song, an artist, or a few words.");
    if (source === "soundcloud") return searchSoundCloud(query);
    if (source === "audius") return searchAudius(query);
    if (source === "archive") return searchArchive(query);
    return searchCatalog(query);
  });
}
