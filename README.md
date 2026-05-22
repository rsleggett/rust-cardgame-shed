# Shed

A Rust + Bevy implementation of the card game **Shed** — one human player vs three AI opponents.

## Overview

Built on the Bevy ECS (Entity Component System) game engine. The game features:

- Standard 52-card deck dealt 3 face-down + 3 face-up + 3 in-hand per player
- Four-seat table: human at bottom, three AIs across the top
- Special card rules: 2 (reset), 3 (transparent), 7 (force ≤ 7), 10 (burn pile)
- 4-of-a-kind burn (across one or multiple plays)
- Multi-card same-rank plays (click to stage, Enter or "Play Cards" to confirm)
- Hover, selection, invalid-play, and pickup-required visual feedback
- Win condition, game-over overlay, and one-key restart

## Project Structure

- `src/components/` — game components and bundles
  - `card.rs` — `Card`, `Suit`, `Rank`
  - `card_visual.rs` — `CardBundle`, spawn helper, visual update system
  - `game.rs` — `GameState` resource, `Player`, `GamePhase`, dealing logic
- `src/rendering/` — rendering systems and plugins
  - `card_renderer.rs` — `CardRendererPlugin`, animation, layout
  - `card_constants.rs` — sizes, z-index layers, hand-fan parameters
- `src/game_plugin.rs` — main game plugin (systems, input, AI, win/restart)
- `src/main.rs` — application entry point
- `scripts/download-fonts.sh` — fetches the font assets (not tracked in git)

See [CLAUDE.md](CLAUDE.md) for an architecture overview and [DEVELOPMENT.md](DEVELOPMENT.md) for design notes and outstanding work.

## How to Run

```bash
./scripts/download-fonts.sh   # one-time: fetch fonts into assets/fonts/
cargo run
```

## Controls

| Input | Action |
|---|---|
| Click card | Stage / deselect (only on your turn) |
| Enter | Confirm staged play |
| Escape | Clear staged selection |
| Play Cards button | Confirm staged play |
| Click play pile / Space | Pick up cards (when prompt is active) |
| Any key on Game Over | Restart |

## Future Improvements

### Gameplay

1. **Remaining special-card rules** — pick a variant for J, Q, K, A (skip, reverse, force-lowest, etc.) and implement.
2. **AI multi-card play** — let the AI bundle same-rank cards.
3. **Scoring** — track who finishes last across rounds.
4. **Pickup / burn animation** — make pile manipulation read visually.

### Architecture

1. **Split `GameState`** into rules state and entity references.
2. **Event-based communication** — `CardPlayedEvent`, `PileBurnedEvent`, `TurnEndedEvent`.
3. **De-duplicate play paths** — single play function shared between AI and human flows.
4. **System ordering** — make implicit ordering explicit via `.before()` / `.after()`.

### UI

1. **Pile size indicator** — count badge or fanned ghost cards.
2. **Active-seat highlight** — show whose turn it is.
3. **Settings** — AI count, AI speed, optional rule variants.

### Quality

1. **Unit tests** — `can_play_card` and the 4-of-a-kind burn check are pure functions and easy to cover.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
