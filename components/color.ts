export async function sampleAccent(url: string) {
  if (!url) return "#ff4f8b";
  try {
    return await new Promise<string>((resolve, reject) => {
      const image = new Image();
      image.crossOrigin = "anonymous";
      image.onload = () => {
        try {
          const canvas = document.createElement("canvas");
          canvas.width = 24;
          canvas.height = 24;
          const context = canvas.getContext("2d", { willReadFrequently: true });
          if (!context) throw new Error("canvas");
          context.drawImage(image, 0, 0, 24, 24);
          const data = context.getImageData(0, 0, 24, 24).data;
          let best = { score: 0, r: 255, g: 79, b: 139 };
          for (let index = 0; index < data.length; index += 4) {
            const r = data[index];
            const g = data[index + 1];
            const b = data[index + 2];
            const max = Math.max(r, g, b);
            const min = Math.min(r, g, b);
            const saturation = max === 0 ? 0 : (max - min) / max;
            const light = (r + g + b) / (3 * 255);
            if (light < 0.15 || light > 0.9) continue;
            const score = saturation * (1 - Math.abs(light - 0.48));
            if (score > best.score) best = { score, r, g, b };
          }
          resolve(best.score < 0.08 ? "#ff4f8b" : boost(best.r, best.g, best.b));
        } catch (error) {
          reject(error);
        }
      };
      image.onerror = () => reject(new Error("image"));
      image.src = url;
    });
  } catch {
    return hashColor(url);
  }
}

export function companionColor(hex: string) {
  const { r, g, b } = parseHex(hex);
  return rgb(Math.round(g * 0.45 + b * 0.4), Math.round(b * 0.35 + 70), Math.round(r * 0.55 + 40));
}

function boost(r: number, g: number, b: number) {
  const max = Math.max(r, g, b, 1);
  const scale = 255 / max;
  const mix = (channel: number) => Math.max(0, Math.min(255, Math.round(channel * scale * 0.78 + 36)));
  return rgb(mix(r), mix(g), mix(b));
}

function hashColor(seed: string) {
  let hash = 0;
  for (const char of seed) hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
  const hue = hash % 360;
  return `hsl(${hue} 72% 62%)`;
}

function parseHex(hex: string) {
  const match = hex.replace("#", "");
  if (match.length < 6) return { r: 255, g: 79, b: 139 };
  return {
    r: Number.parseInt(match.slice(0, 2), 16),
    g: Number.parseInt(match.slice(2, 4), 16),
    b: Number.parseInt(match.slice(4, 6), 16),
  };
}

function rgb(r: number, g: number, b: number) {
  return `#${[r, g, b].map((channel) => channel.toString(16).padStart(2, "0")).join("")}`;
}
