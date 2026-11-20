import type { NextRequest } from "next/server";
import { respond } from "@/lib/http";
import { suggest } from "@/lib/youtube";

export async function GET(request: NextRequest) {
  const query = request.nextUrl.searchParams.get("q")?.trim().slice(0, 80) || "";
  if (query.length < 2) return Response.json([], { headers: { "cache-control": "no-store" } });
  return respond(() => suggest(query));
}
