import type { NextRequest } from "next/server";
import { getArchiveCreator } from "@/lib/archive";
import { getAudiusArtist } from "@/lib/audius";
import { parseSource } from "@/lib/catalog";
import { respond } from "@/lib/http";
import { getSoundCloudArtist } from "@/lib/soundcloud";
import { getArtist } from "@/lib/youtube";

export async function GET(request: NextRequest) {
  const id = request.nextUrl.searchParams.get("id") || "";
  const source = parseSource(request.nextUrl.searchParams.get("source"));
  return respond(() => {
    if (source === "soundcloud") {
      if (!/^\d{1,20}$/.test(id)) throw new Error("That artist link doesn't look right.");
      return getSoundCloudArtist(id);
    }
    if (source === "audius") {
      if (!/^[A-Za-z0-9]{4,32}$/.test(id)) throw new Error("That artist link doesn't look right.");
      return getAudiusArtist(id);
    }
    if (source === "archive") {
      if (!/^[\w .,'&()+:-]{1,140}$/.test(id)) throw new Error("That artist link doesn't look right.");
      return getArchiveCreator(id);
    }
    if (!/^[\w-]{6,140}$/.test(id)) throw new Error("That artist link doesn't look right.");
    return getArtist(id);
  });
}
