# Decisions

Accepted 2026-11-21. A later change gets a new ADR that supersedes the old one.

## ADR-001: Native window

Status: accepted (2026-11-21)

Context: rust-anything read Vivi as a library because Next.js has no `main`. The app is a Next.js server on port 4731 plus a React window. The user chose a native desktop app over a Rust API behind the React window, and over keeping a browser view.

Decision: `rust/app` is an iced window. It is the entry point. Routes that are not ported yet are answered by the Next.js server at `http://127.0.0.1:4731`. `rust-anything app front-door` scaffolds a CLI, so it is not used for this shape.

Consequences: The React window stays in the repo as the reference. It is not the running UI once the Rust app can play. Pixel layout is not the equivalence bar.

## ADR-002: Direct audio only

Status: accepted (2026-11-21)

Context: The React app embeds the YouTube player when YouTube will not give a file. A native window cannot show that player.

Decision: Playback is a decoded audio file. The window does not embed the YouTube player. SoundCloud HLS is downloaded as one file (`pack=1`) and the AAC frames are played. YouTube uses a direct file when the link serves the whole song; otherwise the window says it cannot play.

Consequences: Songs that only exist inside the YouTube embed still will not play. The browser app keeps its own HLS player for SoundCloud.

## ADR-003: New library on disk

Status: accepted (2026-11-21)

Context: Likes, playlists, and downloads live in the browser's IndexedDB. A native app does not open that database.

Decision: The Rust app stores its library under the Windows user data directory (`Vivi/library.json`). The browser library is left where it is.

Consequences: Saves made in the React app do not appear in the native app.

## ADR-004: Full current scope, one piece at a time

Status: accepted (2026-11-21)

Context: Vivi plays YouTube Music, SoundCloud, Audius, and Internet Archive, with search, a queue, lyrics, radio, likes, playlists, and downloads.

Decision: Every one of those stays in scope. Each piece moves only after it matches the current app. The window ships first and uses the Next.js server for anything not moved yet.

Consequences: The first build does not contain rustypipe or soundcloud-v2 yet. Those land as their own units.

## ADR-005: Catalog crates

Status: accepted (2026-11-21)

Context: `youtubei.js` and `soundcloud.ts` have no line-for-line Rust port inside this repo. Maintained crates cover the same catalogs.

Decision: YouTube Music uses `rustypipe` (GPL-3.0, same license as Vivi). SoundCloud uses `soundcloud-v2`. Audius and Internet Archive stay plain HTTPS. Each crate is checked against the current app before that catalog stops calling the Next.js server.

Consequences: A crate that does not match is not a reason to invent a second client in the same step. The mismatch is fixed or the unit stays on the server.

## ADR-006: Cutover

Status: accepted (2026-11-21)

Context: `Play Vivi.bat` starts the Next.js server and opens the React window.

Decision: The shortcut launches the Rust app once that app can play a direct audio file. The Next.js app stays in the repo as the reference until a later decision says otherwise.

Consequences: This ADR is not done by the front door alone. The shortcut stays on Next.js until playback from the window is verified.

## ADR-007: Equivalence, platform, and checkpoints

Status: accepted (2026-11-21)

Context: Live shelves change between two requests. The product is the Windows 11 edition.

Decision: A port matches the current app's job: the same kind of result, the same error when audio is missing, the same ids and titles for a stable recording. Shelf order and trending rows are noise. The window is Windows 11. Work stops for review after the window plays, then after each catalog.

Consequences: `app record` of the live home page is not a byte-for-byte gate. Stable calls (a known track's playback mode, a parsed fixture) are.

## ADR-008: Same chrome as the React app

Status: accepted (2026-11-21)

Context: ADR-007 says a match is the job, not the pixels. The user then asked for the Vivi style.

Decision: The iced window uses the React app's chrome. Dark mica background, pink mark, pill search, catalog chips, sidebar, shelves of cards, track rows, and the bottom player. Each control does the same job. Spacing can differ where the toolkit cannot match CSS.

Consequences: The first plain list is replaced. Downloads stay a note on the Library page until that unit is ported.
