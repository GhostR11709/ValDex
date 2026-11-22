"use client";

import { Heart, MoreHorizontal, Music, Play } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useLibrary } from "@/components/library-provider";
import { usePlayer } from "@/components/player-provider";
import { useDownloads } from "@/lib/downloads";
import { catalogLabel, catalogOf } from "@/lib/catalog";
import { downloadKey } from "@/lib/downloads";
import { artistLine, formatTime } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { CardItem, Shelf, Track } from "@/lib/types";

export function Cover({ src, alt, className }: { src?: string; alt: string; className?: string }) {
  const [failed, setFailed] = useState(false);
  return (
    <div className={cn("relative overflow-hidden bg-white/8", className)}>
      {!src || failed ? (
        <div className="grid h-full w-full place-items-center text-white/40">
          <Music className="size-7" aria-hidden />
          <span className="sr-only">{alt}</span>
        </div>
      ) : (
        // Album art comes from YouTube's CDN, which changes hostnames.
        // eslint-disable-next-line @next/next/no-img-element
        <img src={src} alt={alt} referrerPolicy="no-referrer" className="absolute inset-0 h-full w-full object-cover" onError={() => setFailed(true)} />
      )}
    </div>
  );
}

export function ShelfRow({
  shelf,
  onOpen,
  onPlay,
}: {
  shelf: Shelf;
  onOpen: (card: CardItem) => void;
  onPlay: (card: CardItem) => void;
}) {
  return (
    <section className="space-y-3">
      <h2 className="px-1 text-xl font-semibold tracking-tight">{shelf.title}</h2>
      <div className="no-scrollbar -mx-1 flex gap-3 overflow-x-auto px-1 pb-2">
        {shelf.items.map((card) => (
          <article key={`${card.kind}-${card.id}`} className="w-40 shrink-0 sm:w-44">
            <div className="group relative">
              <button type="button" onClick={() => onOpen(card)} className="block w-full text-left">
                <Cover src={card.thumbnail} alt="" className="aspect-square w-full rounded-2xl shadow-lg shadow-black/30" />
              </button>
              <Button
                type="button"
                size="icon"
                className="absolute right-2 bottom-2 size-10 rounded-full bg-white text-black opacity-0 shadow-lg transition group-hover:opacity-100 focus-visible:opacity-100"
                aria-label={`Play ${card.title}`}
                onClick={() => onPlay(card)}
              >
                <Play className="fill-current" />
              </Button>
            </div>
            <button type="button" onClick={() => onOpen(card)} className="mt-2 line-clamp-2 text-left text-sm font-medium">
              {card.title}
            </button>
            <p className="line-clamp-1 text-xs text-white/50">{card.subtitle || labelFor(card.kind)}</p>
          </article>
        ))}
      </div>
    </section>
  );
}

function labelFor(kind: CardItem["kind"]) {
  if (kind === "album") return "Album";
  if (kind === "playlist") return "Playlist";
  if (kind === "artist") return "Artist";
  if (kind === "video") return "Video";
  return "Song";
}

export function TrackRow({
  track,
  position,
  active,
  onPlay,
  onAlbum,
  onArtist,
  onRemove,
}: {
  track: Track;
  position: number;
  active?: boolean;
  onPlay: () => void;
  onAlbum?: () => void;
  onArtist?: () => void;
  onRemove?: () => void;
}) {
  const library = useLibrary();
  const liked = library.isLiked(track);
  return (
    <div
      className={cn(
        "group grid grid-cols-[auto_1fr_auto] items-center gap-3 rounded-xl px-2 py-2 hover:bg-white/6 sm:grid-cols-[2rem_1fr_minmax(0,180px)_auto]",
        active && "bg-white/8",
      )}
    >
      <button type="button" onClick={onPlay} className="text-center text-sm text-white/40 tabular-nums" aria-label={`Play ${track.title}`}>
        <span className="group-hover:hidden">{position}</span>
        <Play className="mx-auto hidden size-3.5 fill-current group-hover:block" />
      </button>
      <button type="button" onClick={onPlay} className="flex min-w-0 items-center gap-3 text-left">
        <Cover src={track.thumbnail} alt="" className="size-11 rounded-lg" />
        <span className="min-w-0">
          <span className={cn("block truncate text-sm font-medium", active && "text-[var(--vivi)]")}>{track.title}</span>
          <span className="block truncate text-xs text-white/50">
            {artistLine(track)}
            {catalogOf(track) === "youtube" ? "" : ` · ${catalogLabel(catalogOf(track))}`}
          </span>
        </span>
      </button>
      <button type="button" onClick={onAlbum} className="hidden truncate text-left text-sm text-white/45 hover:text-white sm:block" disabled={!onAlbum}>
        {track.album?.name || ""}
      </button>
      <div className="flex items-center gap-1">
        <span className="w-10 text-right text-xs text-white/40 tabular-nums">{track.duration ? formatTime(track.duration) : ""}</span>
        <Button type="button" variant="ghost" size="icon-sm" aria-label={liked ? "Remove from liked" : "Save to liked"} onClick={() => library.toggleLike(track)}>
          <Heart className={cn(liked && "fill-[var(--vivi)] text-[var(--vivi)]")} />
        </Button>
        <TrackMenu track={track} onAlbum={onAlbum} onArtist={onArtist} onRemove={onRemove} />
      </div>
    </div>
  );
}

function TrackMenu({
  track,
  onAlbum,
  onArtist,
  onRemove,
}: {
  track: Track;
  onAlbum?: () => void;
  onArtist?: () => void;
  onRemove?: () => void;
}) {
  const player = usePlayer();
  const library = useLibrary();
  const downloads = useDownloads();
  const saved = downloads.isDownloaded(track);
  const busy = downloads.isBusy(track);
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button type="button" variant="ghost" size="icon-sm" aria-label={`More options for ${track.title}`}>
          <MoreHorizontal />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuItem onSelect={() => player.playNext(track)}>Play next</DropdownMenuItem>
        <DropdownMenuItem onSelect={() => player.enqueue(track)}>Add to queue</DropdownMenuItem>
        <DropdownMenuItem onSelect={() => library.toggleLike(track)}>
          {library.isLiked(track) ? "Remove from liked" : "Save to liked songs"}
        </DropdownMenuItem>
        <DropdownMenuItem
          disabled={busy}
          onSelect={() => {
            if (saved) {
              const item = downloads.items.find((entry) => entry.id === downloadKey(track) || entry.track.id === track.id);
              void downloads.remove(item?.id || downloadKey(track));
            }
            else void downloads.download(track);
          }}
        >
          {busy ? "Downloading…" : saved ? "Remove download" : "Download"}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuSub>
          <DropdownMenuSubTrigger>Add to playlist</DropdownMenuSubTrigger>
          <DropdownMenuSubContent>
            <DropdownMenuItem onSelect={() => library.requestPlaylist(track)}>New playlist</DropdownMenuItem>
            {library.playlists.map((playlist) => (
              <DropdownMenuItem key={playlist.id} onSelect={() => library.addToPlaylist(playlist.id, track)}>
                {playlist.name}
              </DropdownMenuItem>
            ))}
          </DropdownMenuSubContent>
        </DropdownMenuSub>
        {onAlbum ? <DropdownMenuItem onSelect={onAlbum}>Go to album</DropdownMenuItem> : null}
        {onArtist ? <DropdownMenuItem onSelect={onArtist}>Go to artist</DropdownMenuItem> : null}
        {onRemove ? <DropdownMenuItem onSelect={onRemove}>Remove from this list</DropdownMenuItem> : null}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export function EmptyState({ title, body, action }: { title: string; body: string; action?: React.ReactNode }) {
  return (
    <div className="mx-auto max-w-md rounded-3xl border border-white/10 bg-white/4 px-6 py-10 text-center">
      <h2 className="text-lg font-semibold">{title}</h2>
      <p className="mt-2 text-sm leading-6 text-white/60">{body}</p>
      {action ? <div className="mt-5 flex justify-center">{action}</div> : null}
    </div>
  );
}

export function LoadingBlock({ label }: { label: string }) {
  return (
    <div className="space-y-6" aria-busy="true" aria-label={label}>
      <div className="h-8 w-56 animate-pulse rounded-lg bg-white/10" />
      <div className="flex gap-3 overflow-hidden">
        {Array.from({ length: 5 }, (_, index) => (
          <div key={index} className="h-52 w-40 shrink-0 animate-pulse rounded-2xl bg-white/6" />
        ))}
      </div>
      <div className="space-y-2">
        {Array.from({ length: 4 }, (_, index) => (
          <div key={index} className="h-14 animate-pulse rounded-xl bg-white/5" />
        ))}
      </div>
    </div>
  );
}
