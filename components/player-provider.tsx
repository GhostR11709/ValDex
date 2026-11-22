"use client";

import { createContext, useContext, useEffect, useRef, useState, useSyncExternalStore } from "react";
import { toast } from "sonner";
import { companionColor, sampleAccent } from "@/components/color";
import { useLibrary } from "@/components/library-provider";
import { catalogOf, streamQuery } from "@/lib/catalog";
import { readDownloadAudio } from "@/lib/downloads";
import { artistLine } from "@/lib/format";
import { getJson } from "@/lib/get-json";
import type { PlaybackPayload, Track } from "@/lib/types";

type RepeatMode = "off" | "all" | "one";
type Engine = "audio" | "embed" | null;

type YTPlayer = {
  loadVideoById: (videoId: string | { videoId: string }) => void;
  playVideo: () => void;
  pauseVideo: () => void;
  seekTo: (seconds: number, allowSeekAhead: boolean) => void;
  setVolume: (volume: number) => void;
  getCurrentTime: () => number;
  getDuration: () => number;
};

type PlayerApi = {
  queue: Track[];
  index: number;
  current: Track | null;
  playing: boolean;
  buffering: boolean;
  currentTime: number;
  duration: number;
  volume: number;
  muted: boolean;
  shuffle: boolean;
  repeat: RepeatMode;
  radio: boolean;
  showVideo: boolean;
  engine: Engine;
  error: string | null;
  queueOpen: boolean;
  compact: boolean;
  playTracks: (tracks: Track[], start?: number) => void;
  playNext: (track: Track) => void;
  enqueue: (track: Track) => void;
  removeAt: (position: number) => void;
  jump: (position: number) => void;
  toggle: () => void;
  next: () => void;
  previous: () => void;
  seek: (time: number) => void;
  setVolume: (volume: number) => void;
  toggleMute: () => void;
  cycleRepeat: () => void;
  toggleShuffle: () => void;
  setRadio: (value: boolean) => void;
  setShowVideo: (value: boolean) => void;
  setQueueOpen: (value: boolean | ((open: boolean) => boolean)) => void;
  setCompact: (value: boolean) => void;
  clearUpcoming: () => void;
  getAnalyser: () => AnalyserNode | null;
};

const PlayerContext = createContext<PlayerApi | null>(null);
const SETTINGS_KEY = "vivi-windows-player";

type PlayerSettings = {
  volume: number;
  shuffle: boolean;
  repeat: RepeatMode;
  radio: boolean;
  showVideo: boolean;
};

const defaultSettings: PlayerSettings = {
  volume: 0.9,
  shuffle: false,
  repeat: "off",
  radio: true,
  showVideo: true,
};

let settingsState = defaultSettings;
let settingsHydrated = false;
const settingsListeners = new Set<() => void>();

function readSettings(): PlayerSettings {
  if (typeof window === "undefined") return defaultSettings;
  if (!settingsHydrated) {
    settingsHydrated = true;
    try {
      const saved = JSON.parse(localStorage.getItem(SETTINGS_KEY) || "{}") as Partial<PlayerSettings>;
      const repeat = saved.repeat === "all" || saved.repeat === "one" || saved.repeat === "off" ? saved.repeat : "off";
      settingsState = {
        volume: typeof saved.volume === "number" ? Math.min(1, Math.max(0, saved.volume)) : 0.9,
        shuffle: Boolean(saved.shuffle),
        repeat,
        radio: typeof saved.radio === "boolean" ? saved.radio : true,
        showVideo: typeof saved.showVideo === "boolean" ? saved.showVideo : true,
      };
    } catch {
      settingsState = defaultSettings;
    }
  }
  return settingsState;
}

function writeSettings(patch: Partial<PlayerSettings>) {
  settingsState = { ...readSettings(), ...patch };
  localStorage.setItem(SETTINGS_KEY, JSON.stringify(settingsState));
  settingsListeners.forEach((listener) => listener());
}

function subscribeSettings(listener: () => void) {
  settingsListeners.add(listener);
  return () => settingsListeners.delete(listener);
}

declare global {
  interface Window {
    YT?: { Player: new (element: HTMLElement, options: unknown) => YTPlayer };
    onYouTubeIframeAPIReady?: () => void;
  }
}

export function PlayerProvider({ children }: { children: React.ReactNode }) {
  const library = useLibrary();
  const [queue, setQueue] = useState<Track[]>([]);
  const [index, setIndex] = useState(0);
  const [current, setCurrent] = useState<Track | null>(null);
  const [playing, setPlaying] = useState(false);
  const [buffering, setBuffering] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const settings = useSyncExternalStore(subscribeSettings, readSettings, () => defaultSettings);
  const { volume, shuffle, repeat, radio, showVideo } = settings;
  const [muted, setMuted] = useState(false);
  const [engine, setEngine] = useState<Engine>(null);
  const [error, setError] = useState<string | null>(null);
  const [queueOpen, setQueueOpen] = useState(false);
  const [compact, setCompact] = useState(false);
  const [apiReady, setApiReady] = useState(false);

  const queueRef = useRef(queue);
  const indexRef = useRef(index);
  const currentRef = useRef(current);
  const engineRef = useRef(engine);
  const repeatRef = useRef(repeat);
  const radioRef = useRef(radio);
  const volumeRef = useRef(volume);
  const mutedRef = useRef(muted);
  const showVideoRef = useRef(showVideo);
  const timeRef = useRef(0);
  const durationRef = useRef(0);
  const tokenRef = useRef(0);
  const loadedAt = useRef(0);
  const lastRecent = useRef("");
  const radioLock = useRef(false);
  const pendingRef = useRef<string | null>(null);
  const readyRef = useRef(false);
  const audioRef = useRef<HTMLAudioElement>(null);
  const hlsRef = useRef<{ destroy: () => void } | null>(null);
  const objectUrlRef = useRef<string | null>(null);
  const hostRef = useRef<HTMLDivElement>(null);
  const frameRef = useRef<HTMLDivElement>(null);
  const playerRef = useRef<YTPlayer | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const audioContextRef = useRef<AudioContext | null>(null);
  const recentRef = useRef(library.addRecent);
  const loadRef = useRef<(track: Track) => Promise<void>>(async () => undefined);
  const nextRef = useRef<() => void>(() => undefined);
  const playEmbedRef = useRef<(id: string) => void>(() => undefined);

  queueRef.current = queue;
  indexRef.current = index;
  currentRef.current = current;
  engineRef.current = engine;
  repeatRef.current = repeat;
  radioRef.current = radio;
  volumeRef.current = volume;
  mutedRef.current = muted;
  showVideoRef.current = showVideo;
  recentRef.current = library.addRecent;

  useEffect(() => {
    let cancel = false;
    const ready = () => {
      if (!cancel) setApiReady(true);
    };
    if (window.YT?.Player) {
      queueMicrotask(ready);
      return () => {
        cancel = true;
      };
    }
    window.onYouTubeIframeAPIReady = ready;
    if (!document.querySelector('script[src="https://www.youtube.com/iframe_api"]')) {
      const script = document.createElement("script");
      script.src = "https://www.youtube.com/iframe_api";
      document.body.appendChild(script);
    }
  }, []);

  useEffect(() => {
    if (!apiReady || playerRef.current || !hostRef.current || !window.YT) return;
    playerRef.current = new window.YT.Player(hostRef.current, {
      playerVars: {
        autoplay: 0,
        controls: 0,
        disablekb: 1,
        fs: 0,
        modestbranding: 1,
        rel: 0,
        playsinline: 1,
        origin: window.location.origin,
      },
      events: {
        onReady: () => {
          readyRef.current = true;
          const pending = pendingRef.current;
          if (pending) {
            pendingRef.current = null;
            playerRef.current?.loadVideoById(pending);
          }
        },
        onStateChange: (event: { data: number }) => {
          if (engineRef.current !== "embed") return;
          if (event.data === 1) {
            setPlaying(true);
            setBuffering(false);
            setError(null);
            noteRecent();
            try {
              playerRef.current?.setVolume((mutedRef.current ? 0 : volumeRef.current) * 100);
            } catch {
              /* player may not accept volume yet */
            }
          } else if (event.data === 2) {
            setPlaying(false);
            setBuffering(false);
          } else if (event.data === 3) {
            setBuffering(true);
          } else if (event.data === 0 && Date.now() - loadedAt.current > 1200) {
            nextRef.current();
          }
        },
        onError: (event: { data?: number }) => {
          if (engineRef.current !== "embed") return;
          setPlaying(false);
          setBuffering(false);
          const code = event.data;
          const message =
            code === 100
              ? "YouTube says this song was removed."
              : code === 101 || code === 150
                ? "YouTube blocked embedded playback for this song."
                : "YouTube wouldn't play this track.";
          setError(message);
        },
      },
    });
  }, [apiReady]);

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;
    const onTime = () => {
      timeRef.current = audio.currentTime;
      setCurrentTime(audio.currentTime);
      if (audio.duration && Number.isFinite(audio.duration)) {
        durationRef.current = audio.duration;
        setDuration(audio.duration);
      }
    };
    const onPlay = () => {
      setPlaying(true);
      setBuffering(false);
      noteRecent();
    };
    const onPause = () => setPlaying(false);
    const onWaiting = () => setBuffering(true);
    const onEnded = () => {
      if (engineRef.current !== "audio" || Date.now() - loadedAt.current < 800) return;
      nextRef.current();
    };
    const onError = () => {
      if (engineRef.current !== "audio" || Date.now() - loadedAt.current < 400) return;
      const track = currentRef.current;
      if (hlsRef.current) return;
      if (track && catalogOf(track) === "youtube") playEmbedRef.current(track.id);
      else {
        setPlaying(false);
        setBuffering(false);
        setError("This recording didn't start.");
      }
    };
    audio.addEventListener("timeupdate", onTime);
    audio.addEventListener("play", onPlay);
    audio.addEventListener("pause", onPause);
    audio.addEventListener("waiting", onWaiting);
    audio.addEventListener("ended", onEnded);
    audio.addEventListener("error", onError);
    return () => {
      audio.removeEventListener("timeupdate", onTime);
      audio.removeEventListener("play", onPlay);
      audio.removeEventListener("pause", onPause);
      audio.removeEventListener("waiting", onWaiting);
      audio.removeEventListener("ended", onEnded);
      audio.removeEventListener("error", onError);
    };
  }, []);

  useEffect(() => {
    const timer = window.setInterval(() => {
      if (engineRef.current !== "embed" || !playerRef.current?.getCurrentTime) return;
      const time = playerRef.current.getCurrentTime() || 0;
      const length = playerRef.current.getDuration() || 0;
      timeRef.current = time;
      setCurrentTime(time);
      if (length) {
        durationRef.current = length;
        setDuration(length);
      }
    }, 250);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    const level = muted ? 0 : volume;
    if (audioRef.current) audioRef.current.volume = level;
    try {
      playerRef.current?.setVolume(level * 100);
    } catch {
      /* ignored until the player exists */
    }
  }, [volume, muted, engine]);

  useEffect(() => {
    const place = () => {
      const frame = frameRef.current;
      if (!frame) return;
      const stages = engineRef.current === "embed" ? [...document.querySelectorAll<HTMLElement>("[data-vivi-stage]")] : [];
      const visible = stages
        .filter((element) => showVideoRef.current || element.dataset.viviStage === "bar")
        .map((element) => ({ element, rect: element.getBoundingClientRect() }))
        .filter((item) => item.rect.width > 20 && item.rect.height > 20 && item.rect.bottom > 0 && item.rect.right > 0);
      visible.sort((a, b) => b.rect.width * b.rect.height - a.rect.width * a.rect.height);
      const stage = visible[0]?.element ?? null;
      if (stage) {
        const rect = stage.getBoundingClientRect();
        frame.style.opacity = rect.width > 20 ? "1" : "0";
        frame.style.left = `${rect.left}px`;
        frame.style.top = `${rect.top}px`;
        frame.style.width = `${rect.width}px`;
        frame.style.height = `${rect.height}px`;
        frame.style.pointerEvents = "auto";
      } else {
        frame.style.opacity = "0";
        frame.style.left = "0px";
        frame.style.top = "0px";
        frame.style.width = "1px";
        frame.style.height = "1px";
        frame.style.pointerEvents = "none";
      }
    };
    place();
    const observer = new ResizeObserver(place);
    observer.observe(document.body);
    window.addEventListener("resize", place);
    document.addEventListener("scroll", place, true);
    const timer = window.setInterval(place, 400);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", place);
      document.removeEventListener("scroll", place, true);
      window.clearInterval(timer);
    };
  }, [engine, showVideo, compact]);

  useEffect(() => {
    if (!current) {
      document.title = "Vivi for Windows";
      return;
    }
    document.title = `${current.title} · Vivi`;
    const root = document.documentElement;
    let cancel = false;
    void sampleAccent(current.thumbnail).then((color) => {
      if (cancel) return;
      root.style.setProperty("--vivi", color);
      root.style.setProperty("--vivi-2", companionColor(color));
    });
    if ("mediaSession" in navigator) {
      navigator.mediaSession.metadata = new MediaMetadata({
        title: current.title,
        artist: artistLine(current),
        album: current.album?.name || "Vivi",
        artwork: current.thumbnail ? [{ src: current.thumbnail, sizes: "512x512", type: "image/jpeg" }] : [],
      });
      const setHandler = (action: MediaSessionAction, handler: () => void) => {
        try {
          navigator.mediaSession.setActionHandler(action, handler);
        } catch {
          /* this browser doesn't expose that media key */
        }
      };
      setHandler("play", () => toggleRef.current());
      setHandler("pause", () => toggleRef.current());
      setHandler("previoustrack", () => previousRef.current());
      setHandler("nexttrack", () => nextRef.current());
      try {
        navigator.mediaSession.setActionHandler("seekto", (details) => {
          if (typeof details.seekTime === "number") seek(details.seekTime);
        });
      } catch {
        /* seek-to is optional */
      }
    }
    return () => {
      cancel = true;
    };
  }, [current]);

  useEffect(() => {
    if (!("mediaSession" in navigator) || !duration) return;
    try {
      navigator.mediaSession.playbackState = playing ? "playing" : "paused";
      navigator.mediaSession.setPositionState({ duration, playbackRate: 1, position: Math.min(currentTime, duration) });
    } catch {
      /* position state rejects when duration is still settling */
    }
  }, [playing, duration, currentTime]);

  function noteRecent() {
    const track = currentRef.current;
    if (!track || lastRecent.current === track.id) return;
    lastRecent.current = track.id;
    recentRef.current(track);
  }

  function releaseObjectUrl() {
    if (!objectUrlRef.current) return;
    URL.revokeObjectURL(objectUrlRef.current);
    objectUrlRef.current = null;
  }

  function stopHls() {
    const hls = hlsRef.current;
    hlsRef.current = null;
    if (!hls) return;
    try {
      hls.destroy();
    } catch {
      /* already gone */
    }
  }

  function silenceAudio() {
    stopHls();
    const audio = audioRef.current;
    if (!audio) return;
    audio.pause();
    audio.removeAttribute("src");
    audio.load();
    releaseObjectUrl();
  }

  function silenceEmbed() {
    try {
      playerRef.current?.pauseVideo();
    } catch {
      /* player not ready */
    }
  }

  function ensureAnalyser() {
    const audio = audioRef.current;
    if (!audio || analyserRef.current) return;
    const Context = window.AudioContext || (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
    if (!Context) return;
    const context = new Context();
    const source = context.createMediaElementSource(audio);
    const analyser = context.createAnalyser();
    analyser.fftSize = 128;
    source.connect(analyser);
    analyser.connect(context.destination);
    audioContextRef.current = context;
    analyserRef.current = analyser;
  }

  function playEmbed(id: string, alreadyStarted = false) {
    engineRef.current = "embed";
    setEngine("embed");
    silenceAudio();
    loadedAt.current = Date.now();
    setBuffering(true);
    setPlaying(true);
    if (!readyRef.current || !playerRef.current) {
      pendingRef.current = id;
      return;
    }
    pendingRef.current = null;
    if (!alreadyStarted) playerRef.current.loadVideoById(id);
    else {
      try {
        playerRef.current.playVideo();
      } catch {
        /* already rolling */
      }
    }
    try {
      playerRef.current.setVolume((mutedRef.current ? 0 : volumeRef.current) * 100);
    } catch {
      /* ignored */
    }
  }

  async function load(track: Track) {
    const token = ++tokenRef.current;
    audioRef.current?.pause();
    setError(null);
    setCurrent(track);
    currentRef.current = track;
    setCurrentTime(0);
    setDuration(track.duration || 0);
    timeRef.current = 0;
    durationRef.current = track.duration || 0;
    // YouTube's embed has to start in this click, before any network wait,
    // so Windows treats it as a user gesture. Other catalogs play the file directly.
    const source = catalogOf(track);
    if (source === "youtube") playEmbed(track.id);
    try {
      const local = await readDownloadAudio(track);
      if (tokenRef.current !== token) return;
      if (local) {
        const audio = audioRef.current;
        if (!audio) throw new Error("Audio element missing");
        silenceEmbed();
        ensureAnalyser();
        void audioContextRef.current?.resume();
        engineRef.current = "audio";
        setEngine("audio");
        releaseObjectUrl();
        const url = URL.createObjectURL(local);
        objectUrlRef.current = url;
        audio.src = url;
        try {
          await audio.play();
        } catch {
          if (source === "youtube") playEmbed(track.id);
          else setPlaying(false);
        }
        return;
      }
      if (source === "soundcloud") {
        await playSoundcloud(track, token);
        return;
      }
      if (source !== "youtube") {
        const audio = audioRef.current;
        if (!audio) throw new Error("Audio element missing");
        silenceEmbed();
        ensureAnalyser();
        void audioContextRef.current?.resume();
        engineRef.current = "audio";
        setEngine("audio");
        releaseObjectUrl();
        loadedAt.current = Date.now();
        audio.src = `/api/stream?${streamQuery(track)}`;
        try {
          await audio.play();
        } catch {
          setPlaying(false);
        }
        return;
      }
      const playback = await getJson<PlaybackPayload>(`/api/playback?id=${encodeURIComponent(track.id)}`);
      if (tokenRef.current !== token) return;
      if (playback.mode === "audio") {
        const audio = audioRef.current;
        if (!audio) throw new Error("Audio element missing");
        silenceEmbed();
        ensureAnalyser();
        void audioContextRef.current?.resume();
        engineRef.current = "audio";
        setEngine("audio");
        releaseObjectUrl();
        audio.src = `/api/stream?id=${encodeURIComponent(track.id)}`;
        try {
          await audio.play();
        } catch {
          playEmbed(track.id);
        }
      }
    } catch (loadError) {
      if (tokenRef.current !== token) return;
      const message = loadError instanceof Error ? loadError.message : "";
      if (source === "youtube") {
        if (!/official player|Direct audio/.test(message)) setError("Trying the YouTube player for this track.");
        return;
      }
      setPlaying(false);
      setBuffering(false);
      setError(message && message.length < 160 ? message : "This recording didn't start.");
    }
  }

  async function playSoundcloud(track: Track, token: number) {
    const playback = await getJson<PlaybackPayload>(`/api/playback?${streamQuery(track)}`);
    if (tokenRef.current !== token) return;
    const audio = audioRef.current;
    if (!audio) throw new Error("Audio element missing");
    silenceEmbed();
    stopHls();
    ensureAnalyser();
    void audioContextRef.current?.resume();
    engineRef.current = "audio";
    setEngine("audio");
    releaseObjectUrl();
    loadedAt.current = Date.now();
    setBuffering(true);
    if (playback.mode === "audio") {
      audio.src = `/api/stream?${streamQuery(track)}`;
      try {
        await audio.play();
      } catch {
        setPlaying(false);
      }
      return;
    }
    const { default: Hls } = await import("hls.js");
    if (tokenRef.current !== token) return;
    if (!Hls.isSupported()) throw new Error("This browser can't play SoundCloud.");
    const hls = new Hls({ enableWorker: false });
    hlsRef.current = hls;
    await new Promise<void>((resolve, reject) => {
      hls.on(Hls.Events.MANIFEST_PARSED, () => resolve());
      hls.on(Hls.Events.ERROR, (_event, data) => {
        if (!data.fatal || tokenRef.current !== token) return;
        setPlaying(false);
        setBuffering(false);
        setError("This recording didn't start.");
        reject(new Error("This recording didn't start."));
      });
      hls.loadSource(`/api/stream?${streamQuery(track)}`);
      hls.attachMedia(audio);
    });
    if (tokenRef.current !== token) return;
    try {
      await audio.play();
    } catch {
      setPlaying(false);
    }
  }

  function jump(position: number) {
    const nextTrack = queueRef.current[position];
    if (!nextTrack) return;
    indexRef.current = position;
    setIndex(position);
    void load(nextTrack);
  }

  async function extendRadio() {
    if (radioLock.current) return;
    const track = currentRef.current;
    if (!track || catalogOf(track) !== "youtube") {
      setPlaying(false);
      return;
    }
    radioLock.current = true;
    try {
      const data = await getJson<{ tracks: Track[] }>(`/api/radio?id=${encodeURIComponent(track.id)}`);
      const known = new Set(queueRef.current.map((item) => item.id));
      const fresh = data.tracks.filter((item) => !known.has(item.id));
      if (!fresh.length) {
        setPlaying(false);
        return;
      }
      const position = queueRef.current.length;
      const nextQueue = queueRef.current.concat(fresh);
      queueRef.current = nextQueue;
      indexRef.current = position;
      setQueue(nextQueue);
      setIndex(position);
      await load(nextQueue[position]);
    } catch {
      setPlaying(false);
      toast.error("Couldn't find a radio for this song.");
    } finally {
      radioLock.current = false;
    }
  }

  function next() {
    const items = queueRef.current;
    const position = indexRef.current;
    if (!items.length) return;
    if (repeatRef.current === "one") {
      void load(items[position]);
      return;
    }
    if (position < items.length - 1) {
      jump(position + 1);
      return;
    }
    if (repeatRef.current === "all") {
      jump(0);
      return;
    }
    if (radioRef.current) {
      void extendRadio();
      return;
    }
    setPlaying(false);
  }

  function previous() {
    if (timeRef.current > 3) {
      seek(0);
      return;
    }
    if (indexRef.current > 0) jump(indexRef.current - 1);
    else seek(0);
  }

  function seek(time: number) {
    const cap = durationRef.current || time;
    const nextTime = Math.max(0, Math.min(time, cap || time));
    timeRef.current = nextTime;
    setCurrentTime(nextTime);
    if (engineRef.current === "audio" && audioRef.current) audioRef.current.currentTime = nextTime;
    if (engineRef.current === "embed") {
      try {
        playerRef.current?.seekTo(nextTime, true);
      } catch {
        /* ignored */
      }
    }
  }

  function toggle() {
    if (!currentRef.current) return;
    if (playing) {
      if (engineRef.current === "audio") audioRef.current?.pause();
      else {
        try {
          playerRef.current?.pauseVideo();
        } catch {
          /* ignored */
        }
      }
      setPlaying(false);
      return;
    }
    if (engineRef.current === "audio") {
      void audioContextRef.current?.resume();
      void audioRef.current?.play();
    } else if (engineRef.current === "embed") {
      try {
        playerRef.current?.playVideo();
      } catch {
        /* ignored */
      }
    } else {
      void load(currentRef.current);
    }
    setPlaying(true);
  }

  const toggleRef = useRef(toggle);
  const previousRef = useRef(previous);
  toggleRef.current = toggle;
  previousRef.current = previous;
  playEmbedRef.current = playEmbed;
  nextRef.current = next;
  loadRef.current = load;

  function playTracks(tracks: Track[], start = 0) {
    const playable = tracks.filter((track) => (catalogOf(track) === "youtube" ? /^[\w-]{11}$/.test(track.id) : Boolean(track.id)));
    if (!playable.length) {
      toast.error("Nothing in that list can be played.");
      return;
    }
    const position = Math.min(Math.max(start, 0), playable.length - 1);
    queueRef.current = playable;
    indexRef.current = position;
    setQueue(playable);
    setIndex(position);
    void load(playable[position]);
  }

  function playNext(track: Track) {
    const items = queueRef.current.slice();
    items.splice(indexRef.current + 1, 0, track);
    queueRef.current = items;
    setQueue(items);
    if (!currentRef.current) playTracks([track], 0);
    else toast("Playing next");
  }

  function enqueue(track: Track) {
    const items = queueRef.current.concat(track);
    queueRef.current = items;
    setQueue(items);
    if (!currentRef.current) playTracks([track], 0);
    else toast("Added to the queue");
  }

  function removeAt(position: number) {
    const items = queueRef.current.slice();
    if (!items[position]) return;
    const removingCurrent = position === indexRef.current;
    items.splice(position, 1);
    let nextIndex = indexRef.current;
    if (position < nextIndex) nextIndex -= 1;
    if (removingCurrent) nextIndex = Math.min(position, Math.max(items.length - 1, 0));
    queueRef.current = items;
    indexRef.current = nextIndex;
    setQueue(items);
    setIndex(nextIndex);
    if (removingCurrent) {
      if (items[nextIndex]) void load(items[nextIndex]);
      else {
        silenceAudio();
        silenceEmbed();
        setCurrent(null);
        currentRef.current = null;
        setPlaying(false);
        setEngine(null);
      }
    }
  }

  function clearUpcoming() {
    const items = queueRef.current.slice(0, indexRef.current + 1);
    queueRef.current = items;
    setQueue(items);
  }

  function toggleShuffle() {
    const nextValue = !readSettings().shuffle;
    if (nextValue) {
      const items = queueRef.current.slice();
      const rest = items.slice(indexRef.current + 1);
      for (let cursor = rest.length - 1; cursor > 0; cursor -= 1) {
        const swap = Math.floor(Math.random() * (cursor + 1));
        [rest[cursor], rest[swap]] = [rest[swap], rest[cursor]];
      }
      const merged = items.slice(0, indexRef.current + 1).concat(rest);
      queueRef.current = merged;
      setQueue(merged);
    }
    writeSettings({ shuffle: nextValue });
  }

  const api: PlayerApi = {
    queue,
    index,
    current,
    playing,
    buffering,
    currentTime,
    duration: duration || current?.duration || 0,
    volume,
    muted,
    shuffle,
    repeat,
    radio,
    showVideo,
    engine,
    error,
    queueOpen,
    compact,
    playTracks,
    playNext,
    enqueue,
    removeAt,
    jump,
    toggle,
    next,
    previous,
    seek,
    setVolume: (value) => {
      const nextVolume = Math.max(0, Math.min(1, value));
      volumeRef.current = nextVolume;
      writeSettings({ volume: nextVolume });
      if (nextVolume > 0 && muted) setMuted(false);
    },
    toggleMute: () => setMuted((value) => !value),
    cycleRepeat: () => {
      const currentRepeat = readSettings().repeat;
      writeSettings({ repeat: currentRepeat === "off" ? "all" : currentRepeat === "all" ? "one" : "off" });
    },
    toggleShuffle,
    setRadio: (value) => writeSettings({ radio: value }),
    setShowVideo: (value) => writeSettings({ showVideo: value }),
    setQueueOpen,
    setCompact,
    clearUpcoming,
    getAnalyser: () => analyserRef.current,
  };

  return (
    <PlayerContext.Provider value={api}>
      {children}
      <audio ref={audioRef} preload="none" className="pointer-events-none absolute h-px w-px opacity-0" />
      <div ref={frameRef} className="yt-host fixed top-0 left-0 z-30 overflow-hidden rounded-[28px] opacity-0">
        <div ref={hostRef} className="h-full w-full" />
      </div>
    </PlayerContext.Provider>
  );
}

export function usePlayer() {
  const value = useContext(PlayerContext);
  if (!value) throw new Error("usePlayer must be used inside PlayerProvider");
  return value;
}
