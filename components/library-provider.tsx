"use client";

import { createContext, useCallback, useContext, useMemo, useState, useSyncExternalStore } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import type { Track, UserPlaylist } from "@/lib/types";

type LibraryState = {
  liked: Track[];
  recents: Track[];
  playlists: UserPlaylist[];
  searches: string[];
};

type LibraryApi = LibraryState & {
  ready: boolean;
  isLiked: (id: string) => boolean;
  toggleLike: (track: Track) => void;
  addRecent: (track: Track) => void;
  rememberSearch: (query: string) => void;
  requestPlaylist: (track?: Track) => void;
  renamePlaylist: (id: string, name: string) => void;
  deletePlaylist: (id: string) => void;
  addToPlaylist: (id: string, track: Track) => void;
  removeFromPlaylist: (id: string, trackId: string) => void;
};

const empty: LibraryState = { liked: [], recents: [], playlists: [], searches: [] };
const KEY = "vivi-windows-library-v1";
const LibraryContext = createContext<LibraryApi | null>(null);

function isTrack(value: unknown): value is Track {
  if (!value || typeof value !== "object") return false;
  const track = value as Track;
  return typeof track.id === "string" && typeof track.title === "string" && Array.isArray(track.artists);
}

let libraryState: LibraryState = empty;
let libraryHydrated = false;
const libraryListeners = new Set<() => void>();

function snapshotLibrary(): LibraryState {
  if (typeof window === "undefined") return empty;
  if (!libraryHydrated) {
    libraryHydrated = true;
    libraryState = loadState();
  }
  return libraryState;
}

function commitLibrary(update: (current: LibraryState) => LibraryState) {
  libraryState = update(snapshotLibrary());
  localStorage.setItem(KEY, JSON.stringify(libraryState));
  libraryListeners.forEach((listener) => listener());
}

function subscribeLibrary(listener: () => void) {
  libraryListeners.add(listener);
  return () => libraryListeners.delete(listener);
}

function loadState(): LibraryState {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return empty;
    const parsed = JSON.parse(raw) as Partial<LibraryState>;
    return {
      liked: Array.isArray(parsed.liked) ? parsed.liked.filter(isTrack).slice(0, 500) : [],
      recents: Array.isArray(parsed.recents) ? parsed.recents.filter(isTrack).slice(0, 40) : [],
      searches: Array.isArray(parsed.searches) ? parsed.searches.filter((item) => typeof item === "string").slice(0, 8) : [],
      playlists: Array.isArray(parsed.playlists)
        ? parsed.playlists
            .filter((playlist) => playlist && typeof playlist.id === "string" && typeof playlist.name === "string")
            .map((playlist) => ({
              id: playlist.id,
              name: playlist.name,
              tracks: Array.isArray(playlist.tracks) ? playlist.tracks.filter(isTrack) : [],
            }))
        : [],
    };
  } catch {
    return empty;
  }
}

export function LibraryProvider({ children }: { children: React.ReactNode }) {
  const state = useSyncExternalStore(subscribeLibrary, snapshotLibrary, () => empty);
  const ready = true;
  const [composer, setComposer] = useState<{ track?: Track } | null>(null);
  const [name, setName] = useState("");

  const isLiked = useCallback((id: string) => state.liked.some((track) => track.id === id), [state.liked]);

  const toggleLike = useCallback((track: Track) => {
    const exists = snapshotLibrary().liked.some((item) => item.id === track.id);
    toast(exists ? "Removed from liked songs" : "Saved on this PC");
    commitLibrary((current) => ({
      ...current,
      liked: exists ? current.liked.filter((item) => item.id !== track.id) : [track, ...current.liked].slice(0, 500),
    }));
  }, []);

  const addRecent = useCallback((track: Track) => {
    commitLibrary((current) => ({
      ...current,
      recents: [track, ...current.recents.filter((item) => item.id !== track.id)].slice(0, 40),
    }));
  }, []);

  const rememberSearch = useCallback((query: string) => {
    const trimmed = query.trim();
    if (trimmed.length < 2) return;
    commitLibrary((current) => ({
      ...current,
      searches: [trimmed, ...current.searches.filter((item) => item.toLowerCase() !== trimmed.toLowerCase())].slice(0, 8),
    }));
  }, []);

  const requestPlaylist = useCallback((track?: Track) => {
    setName("");
    setComposer({ track });
  }, []);

  const renamePlaylist = useCallback((id: string, playlistName: string) => {
    const trimmed = playlistName.trim();
    if (!trimmed) return;
    commitLibrary((current) => ({
      ...current,
      playlists: current.playlists.map((playlist) => (playlist.id === id ? { ...playlist, name: trimmed } : playlist)),
    }));
  }, []);

  const deletePlaylist = useCallback((id: string) => {
    commitLibrary((current) => ({ ...current, playlists: current.playlists.filter((playlist) => playlist.id !== id) }));
    toast("Playlist deleted from this PC");
  }, []);

  const addToPlaylist = useCallback((id: string, track: Track) => {
    commitLibrary((current) => ({
      ...current,
      playlists: current.playlists.map((playlist) =>
        playlist.id === id && !playlist.tracks.some((item) => item.id === track.id)
          ? { ...playlist, tracks: [...playlist.tracks, track] }
          : playlist,
      ),
    }));
    toast("Added to the playlist");
  }, []);

  const removeFromPlaylist = useCallback((id: string, trackId: string) => {
    commitLibrary((current) => ({
      ...current,
      playlists: current.playlists.map((playlist) =>
        playlist.id === id ? { ...playlist, tracks: playlist.tracks.filter((track) => track.id !== trackId) } : playlist,
      ),
    }));
  }, []);

  const value = useMemo<LibraryApi>(
    () => ({
      ...state,
      ready,
      isLiked,
      toggleLike,
      addRecent,
      rememberSearch,
      requestPlaylist,
      renamePlaylist,
      deletePlaylist,
      addToPlaylist,
      removeFromPlaylist,
    }),
    [state, ready, isLiked, toggleLike, addRecent, rememberSearch, requestPlaylist, renamePlaylist, deletePlaylist, addToPlaylist, removeFromPlaylist],
  );

  return (
    <LibraryContext.Provider value={value}>
      {children}
      <Dialog open={composer !== null} onOpenChange={(open) => !open && setComposer(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>New playlist</DialogTitle>
            <DialogDescription>Playlists stay in this browser on this PC. They are not uploaded.</DialogDescription>
          </DialogHeader>
          <form
            onSubmit={(event) => {
              event.preventDefault();
              const playlistName = name.trim() || "New playlist";
              const track = composer?.track;
              commitLibrary((current) => ({
                ...current,
                playlists: [
                  { id: crypto.randomUUID(), name: playlistName, tracks: track ? [track] : [] },
                  ...current.playlists,
                ],
              }));
              setComposer(null);
              toast.success(`Created “${playlistName}”`);
            }}
          >
            <Input autoFocus value={name} placeholder="Late night drive" onChange={(event) => setName(event.target.value)} aria-label="Playlist name" />
            <DialogFooter className="mt-4">
              <Button type="button" variant="ghost" onClick={() => setComposer(null)}>
                Cancel
              </Button>
              <Button type="submit">Create</Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </LibraryContext.Provider>
  );
}

export function useLibrary() {
  const value = useContext(LibraryContext);
  if (!value) throw new Error("useLibrary must be used inside LibraryProvider");
  return value;
}
