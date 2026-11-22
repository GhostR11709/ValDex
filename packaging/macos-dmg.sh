#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
stage="$root/dist/ValDex.app/Contents"
server="$stage/MacOS/server"
standalone="$root/.next/standalone"

mkdir -p "$stage/MacOS" "$stage/Resources" "$server"

cp "$root/rust/app/target/aarch64-apple-darwin/release/vivi" "$stage/MacOS/vivi-player"
cp "$root/rust/launch/target/aarch64-apple-darwin/release/vivi-start" "$stage/MacOS/vivi-start"
chmod +x "$stage/MacOS/vivi-player" "$stage/MacOS/vivi-start"

cp -R "$standalone/." "$server/"
mkdir -p "$server/.next"
cp -R "$root/.next/static" "$server/.next/static"
if [ -d "$root/public" ]; then
  cp -R "$root/public" "$server/public"
fi

node_version=$(node -p "process.version")
archive="node-${node_version}-darwin-arm64"
curl -fsSL "https://nodejs.org/dist/${node_version}/${archive}.tar.gz" | tar -xz -C "$root/dist"
cp "$root/dist/${archive}/bin/node" "$stage/MacOS/node"
chmod +x "$stage/MacOS/node"

iconset="$root/dist/ValDex.iconset"
rm -rf "$iconset"
mkdir -p "$iconset"
sips -z 16 16 "$root/valdex-icon.png" --out "$iconset/icon_16x16.png" >/dev/null
sips -z 32 32 "$root/valdex-icon.png" --out "$iconset/icon_16x16@2x.png" >/dev/null
sips -z 32 32 "$root/valdex-icon.png" --out "$iconset/icon_32x32.png" >/dev/null
sips -z 64 64 "$root/valdex-icon.png" --out "$iconset/icon_32x32@2x.png" >/dev/null
sips -z 128 128 "$root/valdex-icon.png" --out "$iconset/icon_128x128.png" >/dev/null
sips -z 256 256 "$root/valdex-icon.png" --out "$iconset/icon_128x128@2x.png" >/dev/null
sips -z 256 256 "$root/valdex-icon.png" --out "$iconset/icon_256x256.png" >/dev/null
sips -z 512 512 "$root/valdex-icon.png" --out "$iconset/icon_256x256@2x.png" >/dev/null
sips -z 512 512 "$root/valdex-icon.png" --out "$iconset/icon_512x512.png" >/dev/null
iconutil -c icns "$iconset" -o "$stage/Resources/ValDex.icns"
cp "$root/packaging/Info.plist" "$stage/Info.plist"

rm -f "$root/dist/ValDex-arm64.dmg"
hdiutil create -volname "ValDex" -srcfolder "$root/dist/ValDex.app" -ov -format UDZO "$root/dist/ValDex-arm64.dmg"
echo "built $root/dist/ValDex-arm64.dmg"
