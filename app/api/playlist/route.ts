import type { NextRequest } from "next/server";
import { respond } from "@/lib/http";
import { getPlaylist } from "@/lib/youtube";

export async function GET(request: NextRequest) {
  const id = request.nextUrl.searchParams.get("id") || "";
  return respond(() => {
    if (!/^[\w-]{6,140}$/.test(id)) throw new Error("That playlist link doesn't look right.");
    return getPlaylist(id);
  });
}
