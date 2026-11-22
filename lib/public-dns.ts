import { lookup as systemLookup, type LookupOptions } from "node:dns";
import { Resolver } from "node:dns/promises";
import net from "node:net";
import { Agent, setGlobalDispatcher } from "undici";

const REVISION = 2;
const LIVE_TTL = 60_000;

let installedRevision = 0;
const liveCache = new Map<string, { at: number; addresses: string[] }>();
const pending = new Map<string, Promise<string[]>>();

type LookupCallback = (
  error: NodeJS.ErrnoException | null,
  address?: string | { address: string; family: number }[],
  family?: number,
) => void;

function systemAddresses(hostname: string) {
  return new Promise<string[]>((resolve) => {
    systemLookup(hostname, { all: true, family: 4 }, (error, addresses) => {
      if (error || !Array.isArray(addresses)) {
        resolve([]);
        return;
      }
      resolve(addresses.filter((item) => item.family === 4).map((item) => item.address));
    });
  });
}

function canConnect(address: string) {
  return new Promise<boolean>((resolve) => {
    const socket = net.connect({ host: address, port: 443, timeout: 1500 });
    const finish = (open: boolean) => {
      socket.destroy();
      resolve(open);
    };
    socket.once("connect", () => finish(true));
    socket.once("timeout", () => finish(false));
    socket.once("error", () => finish(false));
  });
}

async function reachableAddresses(hostname: string) {
  const cached = liveCache.get(hostname);
  if (cached && Date.now() - cached.at < LIVE_TTL) return cached.addresses;
  const running = pending.get(hostname);
  if (running) return running;

  const promise = (async () => {
    let addresses = await systemAddresses(hostname);
    if (!addresses.length) {
      const resolver = new Resolver();
      resolver.setServers(["1.1.1.1", "1.0.0.1"]);
      addresses = await resolver.resolve4(hostname).catch(() => []);
    }
    const checks = await Promise.all(addresses.map(async (address) => ((await canConnect(address)) ? address : "")));
    const live = checks.filter(Boolean);
    const chosen = live.length ? live : addresses;
    if (chosen.length) liveCache.set(hostname, { at: Date.now(), addresses: chosen });
    return chosen;
  })().finally(() => pending.delete(hostname));

  pending.set(hostname, promise);
  return promise;
}

export function installPublicDns() {
  if (installedRevision === REVISION) return;
  installedRevision = REVISION;
  const lookup = (hostname: string, options: LookupOptions | LookupCallback, callback?: LookupCallback) => {
    const done = typeof options === "function" ? options : callback;
    const lookupOptions = typeof options === "function" ? {} : options || {};
    if (!done) return;
    if (lookupOptions.family === 6) {
      systemLookup(hostname, lookupOptions, done);
      return;
    }
    reachableAddresses(hostname).then(
      (addresses) => {
        if (!addresses[0]) {
          done(Object.assign(new Error("SoundCloud didn't answer."), { code: "ENOTFOUND" }));
          return;
        }
        if (lookupOptions.all) done(null, addresses.map((address) => ({ address, family: 4 })));
        else done(null, addresses[0], 4);
      },
      () => done(Object.assign(new Error("SoundCloud didn't answer."), { code: "ENOTFOUND" })),
    );
  };
  setGlobalDispatcher(
    new Agent({
      connect: {
        lookup: lookup as never,
        timeout: 12_000,
        autoSelectFamily: true,
        autoSelectFamilyAttemptTimeout: 400,
      },
      headersTimeout: 20_000,
      bodyTimeout: 30_000,
    }),
  );
}
