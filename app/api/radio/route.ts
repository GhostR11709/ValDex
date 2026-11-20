import type { NextRequest } from "next/server";
import { respond } from "@/lib/http";
import { getRadio } from "@/lib/youtube";

export async function GET(request: NextRequest) {
  const id = request.nextUrl.searchParams.get("id") || "";
  return respond(async () => {
    if (!/^[\w-]{11}$/.test(id)) throw new Error("Missing a song id.");
    return { tracks: await getRadio(id) };
  });
}
