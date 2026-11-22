const SUFFIXES = ["audius.co", "cloudflarestorage.com", "archive.org", "soundcloud.com", "sndcdn.com", "soundcloud.cloud"];

function blockedHost(hostname: string) {
  const host = hostname.toLowerCase().replace(/\.$/, "");
  if (!host || host === "localhost" || host.endsWith(".localhost") || host.endsWith(".local") || host.endsWith(".internal")) return true;
  const ip = /^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/.exec(host);
  if (!ip) return host.includes(":");
  const parts = ip.slice(1).map(Number);
  if (parts.some((part) => part > 255)) return true;
  const [a, b] = parts;
  if (a === 0 || a === 10 || a === 127) return true;
  if (a === 169 && b === 254) return true;
  if (a === 172 && b >= 16 && b <= 31) return true;
  if (a === 192 && b === 168) return true;
  return false;
}

export function isAllowedExternalHost(url: string) {
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== "https:") return false;
    const host = parsed.hostname.toLowerCase();
    if (blockedHost(host)) return false;
    return SUFFIXES.some((suffix) => host === suffix || host.endsWith(`.${suffix}`));
  } catch {
    return false;
  }
}

export async function fetchExternalAudio(url: string, headers: Headers, hops = 0): Promise<Response> {
  if (!isAllowedExternalHost(url)) throw new Error("The audio host was not recognized.");
  const response = await fetch(url, { headers, redirect: "manual" });
  if ([301, 302, 303, 307, 308].includes(response.status)) {
    const location = response.headers.get("location");
    if (!location || hops > 4) throw new Error("The audio host was not recognized.");
    return fetchExternalAudio(new URL(location, url).toString(), headers, hops + 1);
  }
  return response;
}
