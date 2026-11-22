"use client";

import { useSyncExternalStore } from "react";
import { toast } from "sonner";
import { catalogOf, sameTrack, streamQuery } from "@/lib/catalog";
import type { CatalogSource, Track } from "@/lib/types";

export type DownloadRecord = {
  id: string;
  track: Track;
  mime: string;
  bytes: number;
  savedAt: number;
};

type DownloadState = {
  items: DownloadRecord[];
  busyIds: string[];
};

const DB_NAME = "vivi-windows-downloads";
const META = "meta";
const AUDIO = "audio";
const empty: DownloadState = { items: [], busyIds: [] };

let state: DownloadState = empty;
let started = false;
const listeners = new Set<() => void>();

function isTrack(value: unknown): value is Track {
  if (!value || typeof value !== "object") return false;
  const track = value as Track;
  return typeof track.id === "string" && typeof track.title === "string" && Array.isArray(track.artists);
}

function isRecord(value: unknown): value is DownloadRecord {
  if (!value || typeof value !== "object") return false;
  const record = value as DownloadRecord;
  return typeof record.id === "string" && isTrack(record.track) && typeof record.bytes === "number";
}

function emit(next: DownloadState) {
  state = next;
  listeners.forEach((listener) => listener());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  ensure();
  return () => listeners.delete(listener);
}

function snapshot() {
  ensure();
  return state;
}

function ensure() {
  if (started || typeof indexedDB === "undefined") return;
  started = true;
  void refresh().catch(() => {
    started = false;
  });
}

function openDb() {
  return new Promise<IDBDatabase>((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, 1);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(META)) db.createObjectStore(META, { keyPath: "id" });
      if (!db.objectStoreNames.contains(AUDIO)) db.createObjectStore(AUDIO);
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

function requestToPromise<T>(request: IDBRequest<T>) {
  return new Promise<T>((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

function transactionDone(transaction: IDBTransaction) {
  return new Promise<void>((resolve, reject) => {
    transaction.oncomplete = () => resolve();
    transaction.onerror = () => reject(transaction.error);
    transaction.onabort = () => reject(transaction.error ?? new Error("The download was interrupted."));
  });
}

async function refresh() {
  const db = await openDb();
  const rows = await requestToPromise(db.transaction(META).objectStore(META).getAll());
  const items = (Array.isArray(rows) ? rows : []).filter(isRecord).sort((a, b) => b.savedAt - a.savedAt);
  emit({ ...state, items });
}

export function downloadKey(track: { id: string; source?: CatalogSource }) {
  return `${catalogOf(track)}:${track.id}`;
}

function storedKeys(track: { id: string; source?: CatalogSource }) {
  const key = downloadKey(track);
  return catalogOf(track) === "youtube" ? [key, track.id] : [key];
}

export async function readDownloadAudio(track: { id: string; source?: CatalogSource }) {
  if (typeof indexedDB === "undefined") return null;
  const db = await openDb();
  for (const key of storedKeys(track)) {
    const audio = await requestToPromise(db.transaction(AUDIO).objectStore(AUDIO).get(key));
    if (audio instanceof Blob) return audio;
  }
  return null;
}

function setBusy(id: string, busy: boolean) {
  const busyIds = busy ? [...state.busyIds, id] : state.busyIds.filter((item) => item !== id);
  emit({ ...state, busyIds });
}

export async function downloadTrack(track: Track) {
  if (!track.id || track.id.length > 500) {
    toast.error("This item doesn't have a song file to save.");
    return;
  }
  const key = downloadKey(track);
  if (state.items.some((item) => storedKeys(track).includes(item.id))) {
    toast("Already in Downloads");
    return;
  }
  if (state.busyIds.includes(key)) return;

  const toastId = toast.loading(`Downloading “${track.title}”`);
  setBusy(key, true);
  try {
    const response = await fetch(`/api/stream?${streamQuery(track)}${catalogOf(track) === "soundcloud" ? "&pack=1" : ""}`);
    const type = response.headers.get("content-type") || "";
    if (!response.ok || type.includes("json") || !response.body) {
      let message = "YouTube didn't give this PC a file to save.";
      if (type.includes("json")) {
        const body = (await response.json()) as { error?: string };
        if (body.error) message = body.error;
      }
      throw new Error(message);
    }

    const total = Number(response.headers.get("content-length") || 0);
    const reader = response.body.getReader();
    const chunks: Uint8Array[] = [];
    let received = 0;
    let lastPercent = 0;
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      if (!value) continue;
      const copy = new Uint8Array(value.byteLength);
      copy.set(value);
      chunks.push(copy);
      received += copy.byteLength;
      if (total > 0) {
        const percent = Math.floor((received / total) * 100);
        if (percent >= lastPercent + 8) {
          lastPercent = percent;
          toast.loading(`Downloading “${track.title}” ${percent}%`, { id: toastId });
        }
      }
    }
    if (received < 1024) throw new Error("The audio file came back empty.");

    const bytes = new Uint8Array(received);
    let offset = 0;
    for (const chunk of chunks) {
      bytes.set(chunk, offset);
      offset += chunk.byteLength;
    }
    const mime = type.split(";")[0] || "audio/webm";
    const record: DownloadRecord = {
      id: key,
      track,
      mime,
      bytes: received,
      savedAt: Date.now(),
    };
    const db = await openDb();
    const transaction = db.transaction([META, AUDIO], "readwrite");
    transaction.objectStore(META).put(record);
    transaction.objectStore(AUDIO).put(new Blob([bytes], { type: mime }), key);
    await transactionDone(transaction);
    emit({
      ...state,
      items: [record, ...state.items.filter((item) => !storedKeys(track).includes(item.id))],
    });
    toast.success(`Saved “${track.title}”`, { id: toastId });
  } catch (error) {
    const message = error instanceof Error && error.message ? error.message : "Download failed.";
    toast.error(message, { id: toastId });
  } finally {
    setBusy(key, false);
  }
}

export async function removeDownload(id: string) {
  const current = state.items.find((item) => item.id === id || item.track.id === id);
  const key = current?.id || id;
  try {
    const db = await openDb();
    const transaction = db.transaction([META, AUDIO], "readwrite");
    transaction.objectStore(META).delete(key);
    transaction.objectStore(AUDIO).delete(key);
    await transactionDone(transaction);
    emit({ ...state, items: state.items.filter((item) => item.id !== key) });
    toast(current ? `Removed “${current.track.title}”` : "Removed from Downloads");
  } catch {
    toast.error("Couldn't remove that download.");
  }
}

export function useDownloads() {
  const current = useSyncExternalStore(subscribe, snapshot, () => empty);
  return {
    items: current.items,
    busyIds: current.busyIds,
    isDownloaded: (track: { id: string; source?: CatalogSource }) => current.items.some((item) => storedKeys(track).includes(item.id) || sameTrack(item.track, track)),
    isBusy: (track: { id: string; source?: CatalogSource }) => current.busyIds.includes(downloadKey(track)),
    download: downloadTrack,
    remove: removeDownload,
  };
}
