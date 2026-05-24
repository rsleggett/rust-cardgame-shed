#!/usr/bin/env bash
# Music asset bootstrap. The lo-fi loop is not bundled; drop a CC0 or
# royalty-free OGG into assets/music/lofi_loop.ogg and the game will play
# it on loop at low volume. If the file is missing the game runs silently
# (Bevy logs an asset-load warning, no crash).
#
# Suggested sources:
#   - Pixabay Music   https://pixabay.com/music/search/lofi/        (CC0, no attribution)
#   - OpenGameArt     https://opengameart.org/art-search?keys=lofi  (CC0 / CC-BY)
#
# Download a track, convert to OGG Vorbis if needed (`ffmpeg -i in.mp3 out.ogg`),
# rename to lofi_loop.ogg, and place it in assets/music/. Re-run this script
# any time to confirm the file is in place.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MUSIC_DIR="$SCRIPT_DIR/../assets/music"
mkdir -p "$MUSIC_DIR"

DEST="$MUSIC_DIR/lofi_loop.ogg"
if [[ -f "$DEST" ]]; then
  echo "✓ $DEST present — music will play on startup"
  exit 0
fi

cat <<EOF
No music track found at $DEST

Drop a CC0/royalty-free lo-fi OGG into that path to enable background music.
The game runs fine without it — Bevy will log an asset-load warning and stay
silent.

Suggested sources:
  - Pixabay Music   https://pixabay.com/music/search/lofi/
  - OpenGameArt     https://opengameart.org/art-search?keys=lofi
EOF
