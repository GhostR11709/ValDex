# YouTube Music

Claim: `rustypipe` 0.11.4 is the crate for the YouTube Music unit.

Sources, read 2026-11-21:

- https://crates.io/crates/rustypipe — version 0.11.4, published 2025-04-23, GPL-3.0. The readme lists YouTube Music playlist, album, artist, search, suggestions, radio, lyrics, and moods.
- https://codeberg.org/ThetaDev/rustypipe/pulls — repository fixes still landing in 2026 (shorts thumbnails, empty grid continuations). The crates.io release is older than those commits.
- https://crates.io/crates/innertube-rs — 0.12.0, published 2026-11-12, MIT, about 2,250 downloads. It is a port of YouTube.js and can resolve an audio-only stream. It is younger and does not document the Music mood and radio surface `rustypipe` does.

Traps:

- YouTube often refuses a direct file on this network. The React app then embeds the player. ADR-002 does not. The port must return the same "no file" outcome, not a webview.
- Innertube responses change. A recording of Home will not be byte-identical on the next run. Gate a known video id and a search fixture, not the trending shelf.
- `rustypipe`'s GPL-3.0 matches this repo. `innertube-rs` is MIT. Mixing the MIT crate in is allowed, but the chosen crate is `rustypipe` until a recorded case fails and a later ADR switches.

Do not port this unit until the window plays through the current server.
