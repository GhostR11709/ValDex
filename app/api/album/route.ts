import type { NextRequest } from "next/server";
import { getArchiveAlbum } from "@/lib/archive";
import { parseSource } from "@/lib/catalog";
import { respond } from "@/lib/http";
import { getSoundCloudPlaylist } from "@/lib/soundcloud";
import { getAlbum } from "@/lib/youtube";

export async function GET(request: NextRequest) {
  const id = request.nextUrl.searchParams.get("id") || "";
  const source = parseSource(request.nextUrl.searchParams.get("source"));
  return respond(() => {
    if (source === "soundcloud") {
      if (!/^[\w:.-]{1,180}$/.test(id) || id.includes("..")) throw new Error("That album link doesn't look right.");
      return getSoundCloudPlaylist(id);
    }
    if (source === "archive") {
      if (!/^[A-Za-z0-9][A-Za-z0-9._-]{0,119}$/.test(id)) throw new Error("That album link doesn't look right.");
      return getArchiveAlbum(id);
    }
    if (!/^[\w-]{6,140}$/.test(id)) throw new Error("That album link doesn't look right.");
    return getAlbum(id);
  });
}
