import type { NextRequest } from "next/server";
import { getArchiveHome } from "@/lib/archive";
import { getAudiusHome } from "@/lib/audius";
import { parseSource } from "@/lib/catalog";
import { respond } from "@/lib/http";
import { getSoundCloudHome } from "@/lib/soundcloud";
import { getHome } from "@/lib/youtube";

export async function GET(request: NextRequest) {
  const source = parseSource(request.nextUrl.searchParams.get("source"));
  const mood = request.nextUrl.searchParams.get("mood")?.trim().slice(0, 40);
  return respond(() => {
    if (source === "soundcloud") return getSoundCloudHome(mood || undefined);
    if (source === "audius") return getAudiusHome(mood || undefined);
    if (source === "archive") return getArchiveHome();
    return getHome(mood || undefined);
  });
}
