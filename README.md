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
- `src/ai.rs` — per-personality AI strategy (`choose_play`)
- `src/game_plugin.rs` — main game plugin (systems, input, draft, scoring, HUD, win/restart)
- `src/main.rs` — application entry point
- `scripts/download-fonts.sh` — fetches the font assets (not tracked in git)

See [CLAUDE.md](CLAUDE.md) for an architecture overview and [DEVELOPMENT.md](DEVELOPMENT.md) for design notes, per-phase decisions, and outstanding work.

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
| Click buff row | Pick that buff during the draft phase |
| M | Use Mulligan (if drafted, once per round) |
| P | Use Peek (if drafted, once per round) |
| Any key on Game Over | Continue to next round / new match |

## Match Loop

1. **Dealing** — 9 cards per player, ~5 s total.
2. **Drafting** — pick a buff from your pool (3 normally, 5 if you were the Shed last round).
3. **Playing** — standard Shed rules; finish your stacks and you're out (in the good way).
4. **Game Over screen** — finish order, points awarded this round, cumulative scores, continue prompt.
5. Repeat. First player to 10 cumulative points wins the match. New match resets buffs and personas; next round keeps them.

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
2. **Active-seat highlight** — show whose turn it is.
3. **Big Hand layout fix** — hand fan currently overlaps the 4th card.
4. **Smarter Game Over key handling** — ignore M / P so consumable hotkeys don't double as advance-round.
5. **Settings** — AI count, AI speed, match target, optional rule variants.

### Quality
1. **Unit tests** — `can_play_card`, the burn-check, `MatchState::score_for_position`, `MatchState::award_round`, `ai::choose_play` per personality, and `roll_pool` are all pure functions and easy to cover.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
