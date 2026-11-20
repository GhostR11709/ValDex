import type { NextRequest } from "next/server";
import { respond } from "@/lib/http";
import { getHome } from "@/lib/youtube";

export async function GET(request: NextRequest) {
  const mood = request.nextUrl.searchParams.get("mood")?.trim().slice(0, 40);
  return respond(() => getHome(mood || undefined));
}
