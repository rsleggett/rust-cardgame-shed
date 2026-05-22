#!/usr/bin/env bash
# Downloads the font assets the game needs at runtime.
# Fonts are not tracked in git — run this once after cloning.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FONT_DIR="$SCRIPT_DIR/../assets/fonts"
mkdir -p "$FONT_DIR"

# name | url | output filename
FONTS=(
  "NotoSans|https://raw.githubusercontent.com/google/fonts/main/ofl/notosans/NotoSans%5Bwdth%2Cwght%5D.ttf|NotoSans-Regular.ttf"
  "NotoSansSymbols2|https://raw.githubusercontent.com/google/fonts/main/ofl/notosanssymbols2/NotoSansSymbols2-Regular.ttf|NotoSansSymbols2-Regular.ttf"
)

for entry in "${FONTS[@]}"; do
  IFS='|' read -r name url out <<< "$entry"
  dest="$FONT_DIR/$out"
  if [[ -f "$dest" ]]; then
    echo "✓ $out already present, skipping"
    continue
  fi
  echo "→ Downloading $name → $out"
  curl -fSL "$url" -o "$dest"
done

echo "Fonts ready in $FONT_DIR"
