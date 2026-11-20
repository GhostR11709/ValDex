import type { LyricLine } from "./types";

export function parseLrc(source: string): LyricLine[] {
  const lines: LyricLine[] = [];
  const stamp = /\[(\d{1,2}):(\d{2})(?:[.:](\d{1,3}))?\]/g;
  for (const raw of source.split(/\r?\n/)) {
    const times: number[] = [];
    stamp.lastIndex = 0;
    let match: RegExpExecArray | null;
    while ((match = stamp.exec(raw))) {
      const minutes = Number(match[1]);
      const seconds = Number(match[2]);
      const fraction = match[3] ? Number(match[3].padEnd(3, "0").slice(0, 3)) / 1000 : 0;
      times.push(minutes * 60 + seconds + fraction);
    }
    const text = raw.replace(/\[(\d{1,2}):(\d{2})(?:[.:](\d{1,3}))?\]/g, "").trim();
    if (!times.length || !text) continue;
    for (const time of times) lines.push({ time, text });
  }
  lines.sort((a, b) => a.time - b.time);
  return lines;
}
