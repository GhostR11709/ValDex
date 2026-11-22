import { connection } from "next/server";

export async function respond(work: () => Promise<unknown>) {
  try {
    await connection();
    const data = await withTimeout(work(), 22000);
    return Response.json(data, { headers: { "cache-control": "no-store" } });
  } catch (error) {
    return jsonError(error);
  }
}

function withTimeout<T>(promise: Promise<T>, ms: number) {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("The catalog took too long to answer.")), Math.max(ms, 30000));
    promise.then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      (error) => {
        clearTimeout(timer);
        reject(error);
      },
    );
  });
}

export function jsonError(error: unknown, status?: number) {
  console.error(error);
  const message = error instanceof Error ? error.message : "";
  const friendly =
    message && message.length < 160 && !/innertube|https?:\/\//i.test(message) && !message.includes("\n")
      ? message
      : "That didn't work. Try again in a moment.";
  const code = status ?? (/doesn't look right|Type a|Missing/.test(friendly) ? 400 : 502);
  return Response.json({ error: friendly }, { status: code, headers: { "cache-control": "no-store" } });
}
