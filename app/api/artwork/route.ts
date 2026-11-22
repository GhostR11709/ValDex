import { connection } from "next/server";
import type { NextRequest } from "next/server";
import { jsonError } from "@/lib/http";
import { installPublicDns } from "@/lib/public-dns";
import { isAllowedExternalHost } from "@/lib/public-media";

export async function GET(request: NextRequest) {
  try {
    await connection();
    installPublicDns();
    const target = request.nextUrl.searchParams.get("url") || "";
    const parsed = new URL(target);
    const host = parsed.hostname.toLowerCase();
    if (parsed.protocol !== "https:" || parsed.username || parsed.password || !isAllowedExternalHost(target)) {
      return jsonError(new Error("That artwork link doesn't look right."), 400);
    }
    if (host !== "sndcdn.com" && !host.endsWith(".sndcdn.com")) {
      return jsonError(new Error("That artwork link doesn't look right."), 400);
    }
    const upstream = await fetch(target, { redirect: "follow", headers: { "user-agent": "Mozilla/5.0", accept: "image/*" } });
    if (!upstream.ok || !upstream.body) return jsonError(new Error("That artwork isn't available."), 404);
    return new Response(upstream.body, {
      headers: {
        "content-type": upstream.headers.get("content-type") || "image/jpeg",
        "cache-control": "public, max-age=86400",
      },
    });
  } catch (error) {
    return jsonError(error);
  }
}
