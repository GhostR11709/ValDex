# Map

Goal: a Windows 11 iced app that does what Vivi does now. The Next.js app stays as the reference (ADR-001, ADR-006).

Decisions: [DECISIONS.md](DECISIONS.md). Names: [GLOSSARY.md](GLOSSARY.md).

## Units

| Unit | State | Notes |
| --- | --- | --- |
| Window and HTTP fallback | claimed by this session 2026-11-21 | `rust/app`. Vivi chrome (ADR-008). Home, search, and direct-file playback call port 4731. |
| Likes file | claimed by this session 2026-11-21 | `library.json` under the user data dir. Downloads are files next to it. Playlists are not in this claim. |
| YouTube Music (`rustypipe`) | open | ADR-005. Research: [research/youtube.md](research/youtube.md). |
| SoundCloud (`soundcloud-v2`) plus HLS decode | open | ADR-002, ADR-005. Research: [research/soundcloud.md](research/soundcloud.md). |
| Audius | open | Plain HTTPS. No account. |
| Internet Archive | open | Plain HTTPS. |
| Lyrics and radio | open | Still the Next.js routes. |
| Playlists | open | ADR-003. Downloads in the window save files on this PC and do not read the browser library. |

## Out of scope

- The React window, as a thing to recompile. It stays as the reference.
- The YouTube embed player (ADR-002).
- Importing the browser's IndexedDB library (ADR-003).

## Open questions

The window plays a file from the current server, including a SoundCloud song that arrives as HLS and a YouTube song that arrives as a progressive file. Next unit: YouTube Music through rustypipe.
