import type { NextRequest } from "next/server";
import { respond } from "@/lib/http";
import { searchCatalog } from "@/lib/youtube";

export async function GET(request: NextRequest) {
  const query = request.nextUrl.searchParams.get("q")?.trim().slice(0, 120) || "";
  return respond(() => {
    if (!query) throw new Error("Type a song, an artist, or a few words.");
    return searchCatalog(query);
  });
}
