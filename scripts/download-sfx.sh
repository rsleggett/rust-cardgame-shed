#!/usr/bin/env bash
# Sound-effect asset bootstrap. The clips are not tracked in git — run this once
# after cloning (like download-fonts.sh). The game runs fine without them: any
# missing clip is simply silent (Bevy logs an asset-load warning, no crash).
#
# Source: Kenney "Interface Sounds" — Creative Commons Zero (CC0), free for any
# use, mirrored by Calinou. https://kenney.nl/assets/interface-sounds
# We pull a handful and rename them to the canonical names the game loads.
#
# The script is idempotent (skips files already present). Re-run any time the
# clips go missing.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SFX_DIR="$SCRIPT_DIR/../assets/sfx"
mkdir -p "$SFX_DIR"

BASE="https://raw.githubusercontent.com/Calinou/kenney-interface-sounds/master/addons/kenney_interface_sounds"

# game name | source file (Kenney) — chosen to fit each cue
SFX=(
  "card_play|drop_002.wav"        # a card dropping onto the pile
  "burn|glass_006.wav"            # bright sweep as the pile burns
  "pickup|scratch_004.wav"        # cards sliding into the hand
  "deal|scratch_002.wav"          # shuffle/riffle at deal start
  "button|click_001.wav"          # chunky button click
  "score|confirmation_001.wav"    # pleasant ding on points/finish
  "invalid|error_004.wav"         # buzz on an illegal play
)

for entry in "${SFX[@]}"; do
  IFS='|' read -r name src <<< "$entry"
  dest="$SFX_DIR/$name.wav"
  if [[ -f "$dest" ]]; then
    echo "✓ $name.wav already present, skipping"
    continue
  fi
  echo "→ Downloading $src → $name.wav"
  curl -fSL "$BASE/$src" -o "$dest"
done

echo "Sound effects ready in $SFX_DIR"
