import type { NextRequest } from "next/server";
import { parseSource } from "@/lib/catalog";
import { respond } from "@/lib/http";
import { suggest } from "@/lib/youtube";

export async function GET(request: NextRequest) {
  const query = request.nextUrl.searchParams.get("q")?.trim().slice(0, 80) || "";
  const source = parseSource(request.nextUrl.searchParams.get("source"));
  if (query.length < 2 || source !== "youtube") return Response.json([], { headers: { "cache-control": "no-store" } });
  return respond(() => suggest(query));
}
