# Window

Claim: iced 0.14 is the native window. Audio output is rodio, which decodes with Symphonia.

Sources, read 2026-11-21:

- https://github.com/iced-rs/iced/blob/master/CHANGELOG.md — 0.14.0 (2025-12-07) adds the `Task` API's follow-on, reactive rendering, and Windows window settings. The 0.14 todos example builds an `Application` with `iced::application(new, update, view).run()`.
- https://docs.rs/iced/latest/iced/ — experimental, Elm-style `update` / `view`, Windows included.
- https://docs.rs/rodio/latest/rodio/ — 0.23. `DeviceSinkBuilder::open_default_sink()` plus `rodio::play`. Default decoders include mp3, mp4, flac, vorbis, and wav. An m3u8 body is not one of those.

Traps:

- iced is experimental. The window follows the React chrome (ADR-008): mica background, pink mark, shelves, and the bottom player. It is not a pixel copy of the CSS.
- rodio's sink must stay alive or playback stops. Decode on the UI thread after the bytes arrive. Do not send the sink across a `Task`.
- HLS (SoundCloud) is outside this decoder. ADR-002.

The React widgets (`components/`, `components/ui/`) stay in JavaScript. They are the reference, not a crate to replace one button at a time.
