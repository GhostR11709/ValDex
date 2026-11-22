import type { CardItem, CatalogSource, Track } from "@/lib/types";

export const CATALOGS: { id: CatalogSource; label: string; blurb: string }[] = [
  {
    id: "youtube",
    label: "YouTube Music",
    blurb: "Shelves come from YouTube Music. Songs you save stay in this browser.",
  },
  {
    id: "soundcloud",
    label: "SoundCloud",
    blurb: "Music from SoundCloud. No account, and playback is not capped in this app.",
  },
  {
    id: "audius",
    label: "Audius",
    blurb: "Free music uploaded by artists. No account, and playback is not capped in this app.",
  },
  {
    id: "archive",
    label: "Internet Archive",
    blurb: "Free netlabel releases and old recordings. No account.",
  },
];

export function parseSource(value: string | null | undefined): CatalogSource {
  if (value === "soundcloud" || value === "audius" || value === "archive" || value === "youtube") return value;
  return "youtube";
}

export function catalogOf(track?: { source?: CatalogSource } | null): CatalogSource {
  return track?.source || "youtube";
}

export function catalogLabel(source: CatalogSource) {
  return CATALOGS.find((item) => item.id === source)?.label || "YouTube Music";
}

export function catalogBlurb(source: CatalogSource) {
  return CATALOGS.find((item) => item.id === source)?.blurb || CATALOGS[0].blurb;
}

export function sameTrack(a: { id: string; source?: CatalogSource }, b: { id: string; source?: CatalogSource }) {
  return a.id === b.id && catalogOf(a) === catalogOf(b);
}

export function streamQuery(track: { id: string; source?: CatalogSource }) {
  return new URLSearchParams({ source: catalogOf(track), id: track.id }).toString();
}

export function songCard(track: Track): CardItem {
  return {
    kind: "song",
    id: track.id,
    source: catalogOf(track),
    title: track.title,
    subtitle: track.artists.map((artist) => artist.name).filter(Boolean).join(", "),
    thumbnail: track.thumbnail,
    track,
  };
}
