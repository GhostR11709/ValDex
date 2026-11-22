import type { NextRequest } from "next/server";
import { parseSource } from "@/lib/catalog";
import { respond } from "@/lib/http";
import { getSoundCloudPlaylist } from "@/lib/soundcloud";
import { getPlaylist } from "@/lib/youtube";

export async function GET(request: NextRequest) {
  const id = request.nextUrl.searchParams.get("id") || "";
  const source = parseSource(request.nextUrl.searchParams.get("source"));
  return respond(() => {
    if (source === "soundcloud") {
      if (!/^[\w:.-]{1,180}$/.test(id) || id.includes("..")) throw new Error("That playlist link doesn't look right.");
      return getSoundCloudPlaylist(id);
    }
    if (!/^[\w-]{6,140}$/.test(id)) throw new Error("That playlist link doesn't look right.");
    return getPlaylist(id);
  });
}
