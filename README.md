# Vivi for Windows

A Windows 11 music client inspired by [Vivi Music](https://github.com/vivizzz007/vivi-music), the Material 3 YouTube Music app for Android.

The Android project is a phone app. It does not run as a Windows program. This edition is a desktop player you can open in a browser on Windows 11, or install from Edge or Chrome so it has its own window, taskbar icon, and media keys.

Copyright (C) 2026 Vivi for Windows contributors.

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. See `LICENSE`.

This code does not copy the Android/Kotlin source, and it does not include the Musixmatch or JioSaavn modules from Vivi Music.

## What you can do

- Browse YouTube Music home shelves and mood filters
- Search songs, albums, artists, and playlists
- Play a queue with shuffle, repeat, seek, and volume
- Open a now-playing view with album color and lyrics
- Save liked songs and playlists in this browser, on this PC

Playback uses a direct audio stream when YouTube returns one to the computer running the app. When it does not, Vivi plays through the official YouTube player. Lyrics come from [LRCLIB](https://lrclib.net), then from YouTube Music's own lyrics page.

Vivi is not affiliated with YouTube, Google, or the Vivi Music developers.

## Run it on Windows 11

Install [Node.js 22](https://nodejs.org/) or newer, then in this folder:

```bash
npm install
npm run dev
```

Open the URL printed in the terminal. The dev server uses port **4731**.

To install it as an app: in Microsoft Edge or Chrome, open the browser menu and choose **Install Vivi**, or **Apps → Install this site as an app**. It then opens in its own window and responds to the keyboard media keys.

For a production build:

```bash
npm run build
npm start
```

## Credits

- [vivizzz007/vivi-music](https://github.com/vivizzz007/vivi-music) — the Android app this Windows edition is inspired by, GPL-3.0
- [youtubei.js](https://github.com/LuanRT/YouTube.js) by LuanRT — MIT-licensed YouTube Music client used for search and catalog
- [LRCLIB](https://lrclib.net) — synced lyrics
- Mostafa Alagamy — Metrolist, credited by the original Vivi Music project
