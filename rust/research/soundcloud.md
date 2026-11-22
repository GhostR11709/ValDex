# SoundCloud

Claim: `soundcloud-v2` 0.2.0 is the crate for the catalog unit. Playback is a separate unit because the crate returns a stream URL, and current tracks are fMP4 HLS.

Sources, read 2026-11-21:

- https://crates.io/crates/soundcloud-v2 — 0.2.0, published 2026-11-14, MIT, a few dozen downloads. `Client::discover()` reads the web player for a client id and does not write it to disk. Coverage listed by the crate: search, users, playlists, charts, and transcoding resolve. No user token.
- https://crates.io/crates/rsoundcloud — 0.2.6, updated 2026-04-24, MIT, a few thousand downloads. Read-only api-v2, also discovers a client id. It does not document charts and transcoding as clearly as `soundcloud-v2`.
- The current app (`lib/soundcloud.ts`) uses `soundcloud.ts` 0.7.4 the same way: no account, skip `snipped` previews when a full transcoding exists, and proxy HLS because Windows playback of those playlists is not a plain audio file.

Traps:

- This PC's DNS does not resolve `soundcloud.com`. The Next.js server falls back to 1.1.1.1. The Rust client has to do the same or every call fails on this machine.
- Some tracks are policy `SNIP` and are 30 seconds. That is the upstream file, not a short read. A full track is policy `ALLOW` and several minutes.
- `soundcloud-v2` is new. If a recorded search or a known track id does not match the current app, fix the call or stay on the Next.js route. Do not scrape a client id into the repo.
- ADR-002: until the HLS decoder exists, the window tells the user the song cannot play. It does not pretend the m3u8 is a file.

Do not port this unit until the window plays through the current server.
