# Shed

A Rust + Bevy implementation of the card game **Shed** — one human player vs three AI opponents, played in matches of multiple rounds with a Balatro-style buff draft between each round.

## Overview

Built on the Bevy ECS (Entity Component System) game engine. The game features:

- Standard 52-card deck dealt 3 face-down + 3 face-up + 3 in-hand per player.
- Four-seat table: human at bottom, three AIs across the top.
- Special card rules: 2 (reset), 3 (transparent), 7 (force ≤ 7), 10 (burn pile), 4-of-a-kind burn.
- Multi-card same-rank plays for both human (click to stage) and AI (personality-driven bundling).
- **Finish-order ranking, not first-out-wins.** The round ends when one player remains — that player is the "Shed". When the human is eliminated early, AI ticks speed up so the round resolves in seconds.
- **Match-level scoring.** 3/2/1/0 points per finish position; first to 10 wins the match. The previous round's Shed plays first next round (Shed punishment).
- **Three named AI personalities** (Rob, Mike, Dave) randomly drawn per match. Duplicates allowed and disambiguated ("Dave 2").
  - **Rob** is gung-ho and smart — bundles multi-card plays, proactive with burns.
  - **Mike** is cunning — always plays single cards, hoards specials.
  - **Dave** is chaotic — random rank, often wastes specials by playing multiples of them.
- **Per-round buff draft.** Every round (including round 1) each seat picks one perk from a private pool. Buffs stack across the whole match. The previous Shed gets a bigger pool (5 vs 3) as a rubber-band.
- Buff catalogue: Wild Twos, Hot Hand, Counter-7, Big Hand, Wild Kings, Half Pickup (passives); Mulligan, Peek (consumables refreshed each round).
- Hover, selection, invalid-play, pickup-required, and active-buffs HUD feedback.
- Round-end overlay with finish-order, +points this round, cumulative scores; one-key continue.

## Project Structure

- `src/components/` — game components and bundles
  - `card.rs` — `Card`, `Suit`, `Rank`
  - `card_visual.rs` — `CardBundle`, spawn helper, visual update system
  - `game.rs` — `GameState`, `Player`, `MatchState`, `Personality`, `BuffKind`, `ActiveBuff`, dealing logic
- `src/rendering/` — rendering systems and plugins
  - `card_renderer.rs` — `CardRendererPlugin`, animation, layout
  - `card_constants.rs` — sizes, z-index layers, hand-fan parameters
- `src/systems/` — gameplay systems (dealing, input, play, swap, draft, consumables, AI runner, visuals)
- `src/ui/` — interactive widgets and overlays (play button, score HUD, rules panel, pile status, game over)
- `src/rules.rs` — pure rule predicates (`can_play_card`, `is_burn`) with unit tests
- `src/ai.rs` — per-personality AI strategy (`choose_play`)
- `src/audio.rs` — background music + Ctrl+M mute toggle
- `src/game_plugin.rs` — `GamePlugin`: resource registration, system wiring, one-time setup
- `src/lib.rs` — re-exports modules so the integration tests can drive them
- `src/main.rs` — application entry point (1440×900 window)
- `tests/` — integration tests driving real Bevy `App`s
- `scripts/download-fonts.sh` — fetches the font assets (not tracked in git)
- `scripts/download-music.sh` — sets up `assets/music/` and prints CC0 track sources (optional)

See [CLAUDE.md](CLAUDE.md) for an architecture overview and [DEVELOPMENT.md](DEVELOPMENT.md) for design notes, per-phase decisions, and outstanding work.

## How to Run

```bash
./scripts/download-fonts.sh   # one-time: fetch fonts into assets/fonts/
./scripts/download-music.sh   # optional: set up assets/music/ for background music
cargo run
cargo test                    # run the unit + integration test suite
```

### Play in a browser (WebAssembly)

The game also builds to WebAssembly and is auto-deployed to GitHub Pages on every
merge to `master`: **https://rsleggett.github.io/rust-cardgame-shed/**

To run the web build locally:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk            # one-time: the wasm bundler
./scripts/download-fonts.sh    # fonts are gitignored — fetch them first
trunk serve                    # serves at http://localhost:8080
```

The web build ships silent (no bundled music track). See
[.github/workflows/deploy-web.yml](.github/workflows/deploy-web.yml) for the deploy
pipeline.

It also works on phones, in both orientations and by touch. The layout is
orientation-aware: landscape keeps the desktop table, while portrait packs the
three AIs into a compact strip across the top and gives the bottom of the screen
to your hand and the pile at a large, readable scale. The rules panel is hidden
on narrow screens to avoid overflow, and an active-seat glow shows whose turn it
is. Staging cards, picking up the pile, swapping, and continuing past Game Over
all work by tap as well as mouse/keyboard.

## Controls

| Input | Action |
|---|---|
| Click card | Stage / deselect (only on your turn) |
| Double-click card | Play that card immediately, bypassing staging |
| Enter | Confirm staged play |
| Escape | Clear staged selection |
| Play Cards button | Confirm staged play |
| Click play pile / Space | Pick up cards (when prompt is active) |
| Click hand → click face-up (during Swap) | Swap those two cards |
| Done Swapping button | End the Swap phase |
| Click buff row | Pick that buff during the draft phase |
| M | Use Mulligan (if drafted, once per round) |
| P | Use Peek (if drafted, once per round) |
| Ctrl+M | Toggle background-music mute |
| Any key on Game Over | Continue to next round / new match |

## Match Loop

1. **Dealing** — 9 cards per player, ~5 s total.
2. **Swap** — optionally promote hand cards into your face-up row before play; press "Done Swapping" to confirm.
3. **Drafting** — pick a buff from your pool (3 normally, 5 if you were the Shed last round).
4. **Playing** — standard Shed rules; finish your stacks and you're out (in the good way).
5. **Game Over screen** — finish order, points awarded this round, cumulative scores, continue prompt.
6. Repeat. First player to 10 cumulative points wins the match. New match resets buffs and personas; next round keeps them.

## Future Improvements

### Next iteration focus
1. **Animation polish** — animate pile pickups, burns, and draft picks; add an active-seat indicator; show pile size visually.
2. **Core gameplay loop** — finish the special-card set (J, Q, A still vanilla), add persona-aware AI draft picks, expand the buff catalogue, surface what each AI drafted at round-end.

### Gameplay
1. **Remaining special-card rules** — pick a variant for J, Q, A (skip, reverse, force-lowest, etc.) and implement.
2. **Anti-buffs and rare-tier rolls** — Balatro-style risk/reward and rarity gradient.
3. **Targeted buffs** — perks that interact with a chosen opponent.

### Architecture
1. **Split `GameState`** into rules state and entity references.
2. **Event-based communication** — `CardPlayedEvent`, `PileBurnedEvent`, `TurnEndedEvent`, `BuffPickedEvent`.
3. **System ordering** — make implicit ordering explicit via `.before()` / `.after()`.

### UI / Polish
1. **Pile size indicator** — count badge or fanned ghost cards.
2. **Big Hand layout fix** — hand fan currently overlaps the 4th card.
3. **Smarter Game Over key handling** — ignore M / P so consumable hotkeys don't double as advance-round.
4. **Settings** — AI count, AI speed, match target, optional rule variants.
5. **Deeper mobile polish** — width-scaled HUD/buttons, on-screen buttons for the keyboard-only actions (M / P / Enter / Escape) so phones can use consumables, and an on-demand rules toggle. (The layout is already orientation-aware with a portrait seat strip, an active-seat glow, and full touch support; tap-tuning the portrait anchors is ongoing.)

### Quality
The suite currently stands at **61 tests** — 28 inline unit tests over the pure logic in `src/rules.rs` and `src/components/game.rs`, plus 33 integration tests in `tests/` driving real Bevy `App`s. Still uncovered:
1. **`ai_player_system` / `ai_draft_system`** — need a seedable RNG resource to be deterministic.
2. **A full-round end-to-end test** — deal through to a Shed across all phases.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
