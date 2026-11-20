export type Person = {
  name: string;
  id?: string;
};

export type Track = {
  id: string;
  title: string;
  artists: Person[];
  album?: { name: string; id?: string };
  duration?: number;
  thumbnail: string;
};

export type CardKind = "song" | "video" | "album" | "playlist" | "artist";

export type CardItem = {
  kind: CardKind;
  id: string;
  title: string;
  subtitle: string;
  thumbnail: string;
  track?: Track;
};

export type Shelf = {
  title: string;
  items: CardItem[];
};

export type HomePayload = {
  filters: string[];
  shelves: Shelf[];
};

export type SearchPayload = {
  songs: Track[];
  videos: Track[];
  albums: CardItem[];
  artists: CardItem[];
  playlists: CardItem[];
};

export type Collection = {
  kind: "album" | "playlist" | "artist";
  id: string;
  title: string;
  subtitle: string;
  thumbnail: string;
  description?: string;
  tracks: Track[];
  shelves?: Shelf[];
};

export type LyricLine = {
  time: number;
  text: string;
};

export type LyricsPayload = {
  lines: LyricLine[];
  plain: string;
  source: string;
  synced: boolean;
};

export type PlaybackPayload = {
  mode: "audio" | "embed";
  mime?: string;
  reason?: string;
};

export type UserPlaylist = {
  id: string;
  name: string;
  tracks: Track[];
};
