# ValDex

A Windows 11 music player. YouTube Music, SoundCloud, Audius, and Internet Archive are the providers. Likes and downloads stay on this PC.

ValDex is inspired by [Vivi Music](https://github.com/vivizzz007/vivi-music), the GPL-3.0 Android app by vivizzz007. This edition does not copy that Android project, and it does not include the Musixmatch or JioSaavn modules. Those modules have extra credit rules in [Vivi's rules.md](https://github.com/vivizzz007/vivi-music/blob/main/rules.md). Because they are not in this repo, ValDex does not show those labels.

Copyright (C) 2026 ValDex contributors.

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. See `LICENSE` and `NOTICE`.

ValDex is not affiliated with YouTube, Google, SoundCloud, or the Vivi Music developers.

## Install

Download `ValDex-Setup.exe` from [Releases](https://github.com/GhostR11709/ValDex/releases). It installs for the current user and opens ValDex. Windows may say the file is unsigned: choose **More info**, then **Run anyway**.

The next time a newer release is on GitHub, ValDex asks before it installs that version. The installer attached to the release must be named `ValDex-Setup.exe`.

## Play

Search, play, seek, repeat, like, and download. Saved songs stay in `%APPDATA%\Vivi`.

## Develop

Install [Node.js 22](https://nodejs.org/) or newer and Rust. Then:

```bash
npm install
npm run dev
```

The catalog server uses port **4731**. `Play Vivi.bat` opens the React reference. The installed app is the native window in `rust/app`, started by `rust/launch`.

## Publish a new version

1. Set the same version in `rust/launch/Cargo.toml`, `rust/app/Cargo.toml`, `package.json`, and `packaging/vivi.iss`.
2. Build `dist\ValDex-Setup.exe`.
3. Publish a GitHub release tagged `v` plus that version, and attach `ValDex-Setup.exe`.

Installed copies compare their version with that tag and offer the new installer.

## Credits

- [vivizzz007/vivi-music](https://github.com/vivizzz007/vivi-music) — the Android app this Windows edition is inspired by, GPL-3.0
- [mostafaalagamy/Metrolist](https://github.com/mostafaalagamy/Metrolist) — credited by Vivi Music as its inspiration
- [youtubei.js](https://github.com/LuanRT/YouTube.js) by LuanRT — YouTube Music catalog
- [LRCLIB](https://lrclib.net) — synced lyrics
