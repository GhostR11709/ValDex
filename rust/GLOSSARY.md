# Glossary

Names the Rust app uses for the words already in the Next.js app.

| Term | Meaning | Rust name |
| --- | --- | --- |
| Catalog | YouTube Music, SoundCloud, Audius, or Internet Archive | `Source` |
| Track | One playable song: id, source, title, artists, duration | `Track` |
| Card | A shelf tile. A song card may carry a `Track` | `Card` |
| Shelf | A titled row of cards on Home | `Shelf` |
| Collection | An artist, album, or playlist page and its tracks | `Collection` |
| Playback mode | `audio` (a file), `hls` (a playlist), or `embed` (no file) | `Playback` |
| Library | Likes, playlists, and downloads saved on this PC | `Library` in `library.json` |
| Front door | The iced window. Unported work is an HTTP call to port 4731 | `rust/app` |
