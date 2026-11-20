"use client";

import { ChevronLeft, Home, Info, Keyboard, Library, Maximize2, Menu, Minimize2, Minus, Search, Square, X } from "lucide-react";
import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import { toast } from "sonner";
import { CompactStage, NowPlaying, PlayerBar, QueueDrawer } from "@/components/player-chrome";
import { useLibrary } from "@/components/library-provider";
import { usePlayer } from "@/components/player-provider";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { CollectionRoute, HomeView, LibraryView, LocalList, LocalPlaylist, SearchView } from "@/components/views";
import { getJson } from "@/lib/get-json";
import { cn } from "@/lib/utils";
import type { CardItem, Collection, Shelf, Track } from "@/lib/types";

const INSTALL_EVENT = "vivi-install-hint";

function subscribeInstall(callback: () => void) {
  window.addEventListener(INSTALL_EVENT, callback);
  return () => window.removeEventListener(INSTALL_EVENT, callback);
}

function useInstallHidden() {
  return useSyncExternalStore(subscribeInstall, () => localStorage.getItem("vivi-windows-install-hint") === "hidden", () => true);
}

function hideInstallHint() {
  localStorage.setItem("vivi-windows-install-hint", "hidden");
  window.dispatchEvent(new Event(INSTALL_EVENT));
}

type Screen =
  | { id: "home" }
  | { id: "search" }
  | { id: "library" }
  | { id: "album"; browseId: string }
  | { id: "artist"; browseId: string }
  | { id: "playlist"; browseId: string }
  | { id: "liked" }
  | { id: "recents" }
  | { id: "local"; playlistId: string }
  | { id: "now" };

export function ViviApp() {
  const player = usePlayer();
  const library = useLibrary();
  const [stack, setStack] = useState<Screen[]>([{ id: "home" }]);
  const [query, setQuery] = useState("");
  const [mood, setMood] = useState("");
  const [menuOpen, setMenuOpen] = useState(false);
  const [helpOpen, setHelpOpen] = useState(false);
  const installHidden = useInstallHidden();
  const searchRef = useRef<HTMLInputElement>(null);
  const screen = stack[stack.length - 1] ?? { id: "home" };

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const typing = Boolean(target?.closest("input, textarea, select, [contenteditable='true']"));
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f") {
        event.preventDefault();
        goPrimary("search");
        return;
      }
      if (typing) return;
      if (event.key === "?") {
        setHelpOpen(true);
        return;
      }
      if (event.key === "Escape") {
        if (player.queueOpen) player.setQueueOpen(false);
        else if (stack.length > 1) setStack((current) => current.slice(0, -1));
        return;
      }
      if (target?.closest("button, a, [data-slot='slider']")) return;
      if (event.code === "Space") {
        event.preventDefault();
        player.toggle();
      } else if (event.key === "ArrowRight") {
        player.seek(player.currentTime + 5);
      } else if (event.key === "ArrowLeft") {
        player.seek(player.currentTime - 5);
      } else if (event.key === "ArrowUp") {
        event.preventDefault();
        player.setVolume(player.volume + 0.05);
      } else if (event.key === "ArrowDown") {
        event.preventDefault();
        player.setVolume(player.volume - 0.05);
      } else if (event.key.toLowerCase() === "l" && player.current) {
        library.toggleLike(player.current);
      } else if (event.key.toLowerCase() === "q") {
        player.setQueueOpen((open) => !open);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [library, player, stack.length]);

  function goPrimary(id: "home" | "search" | "library") {
    setStack([{ id }]);
    setMenuOpen(false);
    if (id === "search") requestAnimationFrame(() => searchRef.current?.focus());
  }

  function push(next: Screen) {
    setStack((current) => {
      const top = current[current.length - 1];
      if (top && JSON.stringify(top) === JSON.stringify(next)) return current;
      return [...current, next];
    });
    setMenuOpen(false);
  }

  function back() {
    setStack((current) => (current.length > 1 ? current.slice(0, -1) : current));
  }

  async function playCard(card: CardItem, shelf?: Shelf) {
    if (card.track && (card.kind === "song" || card.kind === "video")) {
      const tracks = (shelf?.items.map((item) => item.track).filter(Boolean) as Track[]) || [];
      const list = tracks.length ? tracks : [card.track];
      const start = Math.max(0, list.findIndex((item) => item.id === card.track?.id));
      player.playTracks(list, start);
      return;
    }
    if (card.kind === "song" || card.kind === "video") return;
    try {
      const data = await getJson<Collection>(`/api/${card.kind}?id=${encodeURIComponent(card.id)}`);
      if (!data.tracks.length) {
        toast.error("That page has no playable songs.");
        return;
      }
      player.playTracks(data.tracks, 0);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Couldn't start playback.");
    }
  }

  function openCard(card: CardItem) {
    if (card.kind === "album") push({ id: "album", browseId: card.id });
    else if (card.kind === "artist") push({ id: "artist", browseId: card.id });
    else if (card.kind === "playlist") push({ id: "playlist", browseId: card.id });
    else void playCard(card);
  }

  const browse = {
    onOpen: openCard,
    onPlay: playCard,
    onAlbum: (id: string) => push({ id: "album", browseId: id }),
    onArtist: (id: string) => push({ id: "artist", browseId: id }),
  };

  async function toggleFullScreen() {
    player.setCompact(false);
    if (document.fullscreenElement) await document.exitFullscreen();
    else await document.documentElement.requestFullscreen();
  }

  return (
    <div className="mica flex h-dvh flex-col text-white">
      <header className="titlebar flex h-12 shrink-0 items-center gap-2 border-b border-white/8 px-2">
        <div className="no-drag flex items-center gap-1">
          <Button type="button" variant="ghost" size="icon-sm" className="md:hidden" aria-label="Menu" onClick={() => setMenuOpen((open) => !open)}>
            <Menu />
          </Button>
          <Button type="button" variant="ghost" size="icon-sm" aria-label="Back" onClick={back} disabled={stack.length < 2}>
            <ChevronLeft />
          </Button>
          <span className="hidden items-center gap-2 pr-2 sm:flex">
            <ViviMark />
            <span className="text-sm font-semibold tracking-tight">Vivi</span>
          </span>
        </div>
        <form
          className="no-drag mx-auto w-full max-w-xl"
          onSubmit={(event) => {
            event.preventDefault();
            goPrimary("search");
          }}
        >
          <div className="relative">
            <Search className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-white/40" />
            <Input
              ref={searchRef}
              value={query}
              onChange={(event) => {
                setQuery(event.target.value);
                setStack((current) => (current[current.length - 1]?.id === "search" ? current : [...current, { id: "search" }]));
              }}
              onFocus={() => {
                if (screen.id !== "search") push({ id: "search" });
              }}
              placeholder="Search YouTube Music"
              aria-label="Search YouTube Music"
              className="h-9 rounded-full border-white/10 bg-white/8 pl-9"
            />
          </div>
        </form>
        <div className="no-drag flex items-center">
          <Button type="button" variant="ghost" size="icon-sm" aria-label="Keyboard shortcuts and about" onClick={() => setHelpOpen(true)}>
            <Info />
          </Button>
          <div className="caption-buttons ml-1 hidden items-stretch sm:flex">
            <button type="button" className="caption" aria-label="Mini player" onClick={() => player.setCompact(true)}>
              <Minus className="size-3.5" />
            </button>
            <button type="button" className="caption" aria-label="Full screen" onClick={() => void toggleFullScreen()}>
              {player.compact ? <Maximize2 className="size-3.5" /> : <Square className="size-3" />}
            </button>
            <button
              type="button"
              className="caption caption-close"
              aria-label="Close"
              onClick={() => {
                window.close();
                window.setTimeout(() => {
                  toast("This tab was opened by the browser, so Windows won't let the page close itself. Install Vivi as an app, or close the tab.");
                }, 200);
              }}
            >
              <X className="size-3.5" />
            </button>
          </div>
        </div>
      </header>

      {!installHidden && !player.compact ? (
        <div className="flex items-start justify-between gap-3 border-b border-white/8 bg-white/5 px-4 py-2 text-sm text-white/75">
          <p>On Windows 11, use Edge or Chrome and choose Install Vivi, or Apps, then Install this site as an app. It gets its own window and media keys.</p>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={hideInstallHint}
          >
            Dismiss
          </Button>
        </div>
      ) : null}

      <div className="flex min-h-0 flex-1">
        {menuOpen ? <button type="button" className="fixed inset-0 z-30 bg-black/50 md:hidden" aria-label="Close menu" onClick={() => setMenuOpen(false)} /> : null}
        <nav
          className={cn(
            "z-40 w-60 shrink-0 flex-col gap-1 border-r border-white/8 p-3 md:flex",
            menuOpen ? "fixed inset-y-12 left-0 flex bg-[#101014]" : "hidden",
            player.compact && "md:hidden",
          )}
          aria-label="Primary"
        >
          <NavButton icon={<Home />} label="Home" active={screen.id === "home"} onClick={() => goPrimary("home")} />
          <NavButton icon={<Search />} label="Search" active={screen.id === "search"} onClick={() => goPrimary("search")} />
          <NavButton icon={<Library />} label="Library" active={screen.id === "library" || screen.id === "liked" || screen.id === "local"} onClick={() => goPrimary("library")} />
          <div className="mt-4 px-3 text-xs tracking-wide text-white/35 uppercase">On this PC</div>
          <button type="button" onClick={() => push({ id: "liked" })} className="rounded-xl px-3 py-2 text-left text-sm text-white/75 hover:bg-white/6">
            Liked songs
          </button>
          <button type="button" onClick={() => push({ id: "recents" })} className="rounded-xl px-3 py-2 text-left text-sm text-white/75 hover:bg-white/6">
            Recently played
          </button>
          {library.playlists.map((playlist) => (
            <button key={playlist.id} type="button" onClick={() => push({ id: "local", playlistId: playlist.id })} className="truncate rounded-xl px-3 py-2 text-left text-sm text-white/75 hover:bg-white/6">
              {playlist.name}
            </button>
          ))}
        </nav>
        <main className="min-w-0 flex-1 overflow-y-auto px-4 py-6 sm:px-6">
          {player.compact ? (
            <CompactStage />
          ) : screen.id === "home" ? (
            <HomeView mood={mood} onMood={setMood} browse={browse} />
          ) : screen.id === "search" ? (
            <SearchView query={query} onQuery={setQuery} browse={browse} />
          ) : screen.id === "library" ? (
            <LibraryView onLiked={() => push({ id: "liked" })} onRecents={() => push({ id: "recents" })} onLocal={(id) => push({ id: "local", playlistId: id })} onCreate={() => library.requestPlaylist()} />
          ) : screen.id === "album" ? (
            <CollectionRoute kind="album" id={screen.browseId} browse={browse} />
          ) : screen.id === "artist" ? (
            <CollectionRoute kind="artist" id={screen.browseId} browse={browse} />
          ) : screen.id === "playlist" ? (
            <CollectionRoute kind="playlist" id={screen.browseId} browse={browse} />
          ) : screen.id === "liked" ? (
            <LocalList title="Liked songs" body="Saved in this browser. Nothing is sent to a Vivi account." tracks={library.liked} browse={browse} onRemove={(track) => library.toggleLike(track)} />
          ) : screen.id === "recents" ? (
            <LocalList title="Recently played" body="The last songs started on this PC." tracks={library.recents} browse={browse} />
          ) : screen.id === "local" ? (
            <LocalPlaylist id={screen.playlistId} browse={browse} />
          ) : (
            <NowPlaying onAlbum={(id) => push({ id: "album", browseId: id })} onArtist={(id) => push({ id: "artist", browseId: id })} />
          )}
        </main>
      </div>

      {player.compact ? (
        <div className="no-drag flex justify-center pb-1">
          <Button type="button" variant="outline" className="rounded-full" onClick={() => player.setCompact(false)}>
            <Minimize2 /> Restore window
          </Button>
        </div>
      ) : (
        <nav className="grid grid-cols-3 border-t border-white/8 md:hidden" aria-label="Mobile">
          <NavButton icon={<Home />} label="Home" active={screen.id === "home"} onClick={() => goPrimary("home")} />
          <NavButton icon={<Search />} label="Search" active={screen.id === "search"} onClick={() => goPrimary("search")} />
          <NavButton icon={<Library />} label="Library" active={screen.id === "library"} onClick={() => goPrimary("library")} />
        </nav>
      )}

      <PlayerBar onOpenNow={() => push({ id: "now" })} />
      <QueueDrawer />

      <Dialog open={helpOpen} onOpenChange={setHelpOpen}>
        <DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-lg">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Keyboard className="size-4" /> Vivi for Windows
            </DialogTitle>
            <DialogDescription>An independent Windows 11 client inspired by Vivi Music. Not affiliated with YouTube or Google.</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 text-sm leading-6 text-white/75">
            <p>Space plays and pauses. Arrow keys seek and change volume. L likes the song. Q opens the queue. Ctrl+F jumps to search. Question mark opens this panel.</p>
            <label className="flex items-center justify-between gap-3 rounded-xl bg-white/5 px-3 py-2">
              <span>Continue with a radio when the queue ends</span>
              <input type="checkbox" checked={player.radio} onChange={(event) => player.setRadio(event.target.checked)} />
            </label>
            <label className="flex items-center justify-between gap-3 rounded-xl bg-white/5 px-3 py-2">
              <span>Show the YouTube video on Now playing</span>
              <input type="checkbox" checked={player.showVideo} onChange={(event) => player.setShowVideo(event.target.checked)} />
            </label>
            <p>
              Direct audio plays when YouTube gives this PC a stream. Otherwise Vivi uses the official YouTube player. Lyrics come from LRCLIB, then YouTube Music. This app does not include the Musixmatch or JioSaavn code from the Android project.
            </p>
            <p>
              Inspired by <a className="underline" href="https://github.com/vivizzz007/vivi-music" target="_blank" rel="noreferrer">Vivi Music</a> by vivizzz007, which is GPL-3.0. This Windows edition is original code under the same license. Mostafa Alagamy&apos;s Metrolist and the youtubei.js library by LuanRT sit underneath that family of apps.
            </p>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function NavButton({ icon, label, active, onClick }: { icon: React.ReactNode; label: string; active: boolean; onClick: () => void }) {
  return (
    <button type="button" onClick={onClick} aria-current={active ? "page" : undefined} className={cn("flex items-center justify-center gap-2 rounded-xl px-3 py-2 text-sm md:justify-start", active ? "bg-white/10 text-white" : "text-white/70 hover:bg-white/6")}>
      {icon}
      {label}
    </button>
  );
}

function ViviMark() {
  return (
    <svg viewBox="0 0 32 32" className="size-7" aria-hidden>
      <rect width="32" height="32" rx="8" fill="#ff4f8b" />
      <path d="M9 10.5 16 22l7-11.5" fill="none" stroke="#1a0b12" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}
