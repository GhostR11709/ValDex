"use client";

import { Heart, ListMusic, Pause, Play, Repeat, Repeat1, Shuffle, SkipBack, SkipForward, Volume1, Volume2, VolumeX, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Cover, EmptyState } from "@/components/media";
import { useLibrary } from "@/components/library-provider";
import { usePlayer } from "@/components/player-provider";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { artistLine, formatTime } from "@/lib/format";
import { getJson } from "@/lib/get-json";
import { cn } from "@/lib/utils";
import type { LyricsPayload, Track } from "@/lib/types";

export function PlayerBar({ onOpenNow }: { onOpenNow: () => void }) {
  const player = usePlayer();
  const library = useLibrary();
  const [seeking, setSeeking] = useState(false);
  const [seekValue, setSeekValue] = useState(0);
  const shown = seeking ? seekValue : player.currentTime;
  const liked = player.current ? library.isLiked(player.current) : false;

  return (
    <footer className="border-t border-white/10 bg-black/55 px-3 pt-1 pb-2 backdrop-blur-xl">
      <Slider
        aria-label="Seek"
        min={0}
        max={Math.max(player.duration, 1)}
        step={0.1}
        value={[Math.min(shown, Math.max(player.duration, 1))]}
        disabled={!player.current}
        onValueChange={([value]) => {
          setSeeking(true);
          setSeekValue(value);
          player.seek(value);
        }}
        onValueCommit={() => setSeeking(false)}
        className="seek-slider"
      />
      <div className="mt-1 grid grid-cols-[1fr_auto] items-center gap-3 md:grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)]">
        <div className="flex min-w-0 items-center gap-3 text-left">
          {player.engine === "embed" && player.current && !player.error ? (
            <div data-vivi-stage="bar" className="relative h-12 w-20 shrink-0 overflow-hidden rounded-lg bg-black">
              <Cover src={player.current.thumbnail} alt="" className="size-full" />
            </div>
          ) : (
            <Cover src={player.current?.thumbnail} alt="" className="size-12 rounded-lg" />
          )}
          <button type="button" onClick={onOpenNow} className="min-w-0 text-left" disabled={!player.current}>
          <span className="min-w-0">
            <span className="block truncate text-sm font-medium">{player.current?.title || "Nothing playing"}</span>
            <span className="block truncate text-xs text-white/50">{player.current ? artistLine(player.current) : "Pick a song from Home or Search"}</span>
            {player.error ? <span className="block truncate text-xs text-amber-200/80">{player.error}</span> : null}
          </span>
          </button>
        </div>
        <div className="flex items-center gap-1 md:justify-center">
          <IconTip label={player.shuffle ? "Shuffle on" : "Shuffle off"}>
            <Button type="button" variant="ghost" size="icon-sm" aria-pressed={player.shuffle} className={cn(player.shuffle && "text-[var(--vivi)]")} onClick={player.toggleShuffle}>
              <Shuffle />
            </Button>
          </IconTip>
          <IconTip label="Previous">
            <Button type="button" variant="ghost" size="icon" aria-label="Previous" onClick={player.previous}>
              <SkipBack className="fill-current" />
            </Button>
          </IconTip>
          <Button type="button" size="icon-lg" className="size-11 rounded-full bg-white text-black hover:bg-white/90" aria-label={player.playing ? "Pause" : "Play"} onClick={player.toggle} disabled={!player.current}>
            {player.playing ? <Pause className="fill-current" /> : <Play className="fill-current" />}
          </Button>
          <IconTip label="Next">
            <Button type="button" variant="ghost" size="icon" aria-label="Next" onClick={player.next}>
              <SkipForward className="fill-current" />
            </Button>
          </IconTip>
          <IconTip label={player.repeat === "one" ? "Repeat one" : player.repeat === "all" ? "Repeat queue" : "Repeat off"}>
            <Button type="button" variant="ghost" size="icon-sm" className={cn(player.repeat !== "off" && "text-[var(--vivi)]")} onClick={player.cycleRepeat}>
              {player.repeat === "one" ? <Repeat1 /> : <Repeat />}
            </Button>
          </IconTip>
        </div>
        <div className="col-span-2 flex items-center justify-end gap-1 md:col-span-1">
          <span className="hidden w-24 text-right text-xs text-white/45 tabular-nums md:inline">
            {formatTime(shown)} / {formatTime(player.duration)}
          </span>
          {player.current ? (
            <Button type="button" variant="ghost" size="icon-sm" aria-label={liked ? "Remove from liked" : "Save to liked"} onClick={() => library.toggleLike(player.current!)}>
              <Heart className={cn(liked && "fill-[var(--vivi)] text-[var(--vivi)]")} />
            </Button>
          ) : null}
          <Button type="button" variant="ghost" size="icon-sm" aria-label="Queue" aria-pressed={player.queueOpen} onClick={() => player.setQueueOpen((open) => !open)}>
            <ListMusic />
          </Button>
          <Button type="button" variant="ghost" size="icon-sm" aria-label={player.muted || player.volume === 0 ? "Unmute" : "Mute"} onClick={player.toggleMute}>
            {player.muted || player.volume === 0 ? <VolumeX /> : player.volume < 0.45 ? <Volume1 /> : <Volume2 />}
          </Button>
          <Slider
            aria-label="Volume"
            min={0}
            max={1}
            step={0.01}
            value={[player.muted ? 0 : player.volume]}
            onValueChange={([value]) => player.setVolume(value)}
            className="hidden w-24 md:flex"
          />
        </div>
      </div>
    </footer>
  );
}

function IconTip({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}

export function NowPlaying({ onAlbum, onArtist }: { onAlbum: (id: string) => void; onArtist: (id: string) => void }) {
  const player = usePlayer();
  const track = player.current;
  if (!track) return <EmptyState title="Nothing in the player" body="Go back and start a song. The art, lyrics, and queue will land here." />;
  return (
    <div className="grid items-start gap-8 xl:grid-cols-[minmax(0,1.05fr)_minmax(280px,0.95fr)]">
      <div className="space-y-5">
        <div className="relative mx-auto w-full max-w-xl">
          <div className="pointer-events-none absolute inset-6 -z-10 rounded-full bg-[var(--vivi)] opacity-30 blur-3xl" />
          <div data-vivi-stage="feature" className="relative aspect-square overflow-hidden rounded-[28px] bg-black shadow-2xl shadow-black/50">
            {player.engine === "embed" && player.showVideo ? null : <Cover src={track.thumbnail} alt="" className="size-full" />}
            <Visualizer />
          </div>
        </div>
        <div>
          <h1 className="text-3xl font-semibold tracking-tight text-balance">{track.title}</h1>
          <p className="mt-1 text-sm text-white/65">
            {track.artists.map((artist, index) => (
              <span key={`${artist.name}-${index}`}>
                {index > 0 ? ", " : null}
                {artist.id ? (
                  <button type="button" className="hover:underline" onClick={() => onArtist(artist.id!)}>
                    {artist.name}
                  </button>
                ) : (
                  artist.name
                )}
              </span>
            ))}
            {track.album?.id ? (
              <>
                {" · "}
                <button type="button" className="hover:underline" onClick={() => onAlbum(track.album!.id!)}>
                  {track.album.name}
                </button>
              </>
            ) : null}
          </p>
          <p className="mt-2 text-xs text-white/40">
            {player.engine === "audio" ? "Direct audio from this PC" : "Playing through the YouTube player"}
            {player.buffering ? " · Buffering" : ""}
          </p>
        </div>
      </div>
      <Lyrics key={track.id} track={track} />
    </div>
  );
}

function Visualizer() {
  const player = usePlayer();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const playerRef = useRef(player);
  useEffect(() => {
    playerRef.current = player;
  });
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const context = canvas.getContext("2d");
    if (!context) return;
    let frame = 0;
    const draw = () => {
      frame = requestAnimationFrame(draw);
      const current = playerRef.current;
      const width = canvas.clientWidth;
      const height = canvas.clientHeight;
      const ratio = window.devicePixelRatio || 1;
      if (canvas.width !== Math.floor(width * ratio) || canvas.height !== Math.floor(height * ratio)) {
        canvas.width = Math.floor(width * ratio);
        canvas.height = Math.floor(height * ratio);
      }
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      context.clearRect(0, 0, width, height);
      const analyser = current.engine === "audio" ? current.getAnalyser() : null;
      const count = analyser ? analyser.frequencyBinCount : 28;
      const bins = new Uint8Array(count);
      if (analyser && current.playing) analyser.getByteFrequencyData(bins);
      else if (current.playing) {
        const time = Date.now() / 170;
        for (let index = 0; index < bins.length; index += 1) bins[index] = 30 + Math.abs(Math.sin(time + index * 0.45)) * 170;
      }
      const color = getComputedStyle(document.documentElement).getPropertyValue("--vivi").trim() || "#ff4f8b";
      const gap = 3;
      const barWidth = width / bins.length - gap;
      bins.forEach((value, index) => {
        const barHeight = Math.max(2, (value / 255) * height * 0.72);
        context.fillStyle = color;
        context.globalAlpha = 0.9;
        context.fillRect(index * (barWidth + gap), height - barHeight, Math.max(barWidth, 1), barHeight);
      });
    };
    frame = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(frame);
  }, []);
  return <canvas ref={canvasRef} className="pointer-events-none absolute inset-x-4 bottom-4 h-16" />;
}

function Lyrics({ track }: { track: Track }) {
  const player = usePlayer();
  const [lyrics, setLyrics] = useState<LyricsPayload | null>(null);
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");
  const [message, setMessage] = useState("");
  const activeRef = useRef<HTMLParagraphElement>(null);

  useEffect(() => {
    const controller = new AbortController();
    let active = true;
    const params = new URLSearchParams({
      title: track.title,
      artist: track.artists[0]?.name || "",
      album: track.album?.name || "",
      id: track.id,
      duration: String(track.duration || ""),
    });
    getJson<LyricsPayload>(`/api/lyrics?${params}`, controller.signal)
      .then((payload) => {
        if (!active) return;
        setLyrics(payload);
        setStatus("ready");
      })
      .catch((reason: Error) => {
        if (!active || reason.name === "AbortError") return;
        setMessage(reason.message);
        setStatus("error");
      });
    return () => {
      active = false;
      controller.abort();
    };
  }, [track]);

  const active = lyrics?.synced ? lyrics.lines.reduce((found, line, index) => (line.time >= 0 && line.time <= player.currentTime + 0.2 ? index : found), -1) : -1;

  useEffect(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    activeRef.current?.scrollIntoView({ block: "center", behavior: "smooth" });
  }, [active]);

  return (
    <section className="flex h-[min(68vh,640px)] flex-col rounded-3xl border border-white/10 bg-black/25">
      <header className="flex items-center justify-between px-5 py-4">
        <h2 className="text-sm font-medium">Lyrics</h2>
        <span className="text-xs text-white/40">{lyrics?.source || (status === "loading" ? "Looking…" : "")}</span>
      </header>
      <div className="min-h-0 flex-1 space-y-3 overflow-y-auto px-5 pb-6">
        {status === "loading" ? <p className="text-sm text-white/50">Looking up lyrics…</p> : null}
        {status === "error" ? <p className="text-sm text-white/60">{message}</p> : null}
        {status === "ready" && !lyrics?.lines.length ? (
          <p className="text-sm leading-6 text-white/60">No lyrics turned up for this song. LRCLIB and YouTube Music both came back empty.</p>
        ) : null}
        {lyrics?.lines.map((line, index) => (
          <p
            key={`${line.time}-${index}`}
            ref={index === active ? activeRef : undefined}
            className={cn("text-lg leading-8 text-white/35 transition-colors", lyrics.synced && index === active && "text-2xl text-white")}
          >
            {line.text}
          </p>
        ))}
      </div>
    </section>
  );
}

export function QueueDrawer() {
  const player = usePlayer();
  if (!player.queueOpen) return null;
  return (
    <aside className="fixed inset-x-0 top-12 bottom-24 z-40 flex flex-col border-l border-white/10 bg-[#121216]/95 shadow-2xl backdrop-blur-xl sm:inset-x-auto sm:right-0 sm:w-[360px]">
      <header className="flex items-center justify-between px-4 py-3">
        <div>
          <h2 className="font-semibold">Queue</h2>
          <p className="text-xs text-white/45">{player.queue.length} songs</p>
        </div>
        <div className="flex gap-1">
          <Button type="button" variant="ghost" size="sm" onClick={player.clearUpcoming}>
            Clear upcoming
          </Button>
          <Button type="button" variant="ghost" size="icon-sm" aria-label="Close queue" onClick={() => player.setQueueOpen(false)}>
            <X />
          </Button>
        </div>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-4">
        {player.queue.length ? (
          player.queue.map((track, index) => (
            <div
              key={`${track.id}-${index}`}
              className={cn("flex w-full items-center gap-1 rounded-xl px-2 py-1 hover:bg-white/6", index === player.index && "bg-white/8")}
            >
              <button type="button" onClick={() => player.jump(index)} className="flex min-w-0 flex-1 items-center gap-3 py-1 text-left">
                <Cover src={track.thumbnail} alt="" className="size-10 rounded-md" />
                <span className="min-w-0 flex-1">
                  <span className={cn("block truncate text-sm", index === player.index && "text-[var(--vivi)]")}>{track.title}</span>
                  <span className="block truncate text-xs text-white/45">{artistLine(track)}</span>
                </span>
              </button>
              <button type="button" className="rounded-md p-2 text-white/35 hover:bg-white/10 hover:text-white" aria-label={`Remove ${track.title}`} onClick={() => player.removeAt(index)}>
                <X className="size-3.5" />
              </button>
            </div>
          ))
        ) : (
          <p className="px-3 text-sm text-white/50">The queue is empty.</p>
        )}
      </div>
    </aside>
  );
}

export function CompactStage() {
  const player = usePlayer();
  const track = player.current;
  if (!track) return <p className="m-auto text-sm text-white/50">Nothing is playing. Restore the window and pick a song.</p>;
  return (
    <div className="m-auto flex w-full max-w-3xl items-center gap-6 px-6">
      <div data-vivi-stage="feature" className="relative size-36 overflow-hidden rounded-3xl bg-black sm:size-48">
        {player.engine === "embed" && player.showVideo ? null : <Cover src={track.thumbnail} alt="" className="size-full" />}
      </div>
      <div className="min-w-0">
        <p className="text-xs tracking-[0.16em] text-white/40 uppercase">Mini player</p>
        <h1 className="mt-2 truncate text-2xl font-semibold">{track.title}</h1>
        <p className="truncate text-sm text-white/60">{artistLine(track)}</p>
      </div>
    </div>
  );
}
