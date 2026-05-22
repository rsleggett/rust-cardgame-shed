# CLAUDE.md

## Project Overview

A Rust implementation of the card game "Shit Head" (Shed) using the [Bevy](https://bevyengine.org/) ECS game engine (v0.14.2). One human player vs three AI opponents at a 4-seat table.

## Build & Run

```bash
./scripts/download-fonts.sh  # one-time: fetch font assets (not tracked in git)
cargo run                    # run the game
cargo build                  # build only
cargo check                  # fast type-check without linking
cargo clippy                 # lint
```

Fonts are gitignored. `scripts/download-fonts.sh` pulls them from the
`google/fonts` GitHub mirror into `assets/fonts/`. Re-run any time those files
go missing — the script is idempotent (skips files that already exist).

## Source Structure

```
src/
  main.rs                        # App entry point — 1440x900 window
  game_plugin.rs                 # GamePlugin: systems, input, AI, win/restart
  components/
    mod.rs
    card.rs                      # Card component, Suit, Rank
    card_visual.rs               # CardBundle, spawn_card_complete, update_card_visuals
    game.rs                      # GameState resource, Player, GamePhase, dealing logic
  rendering/
    mod.rs
    card_renderer.rs             # CardRendererPlugin, CardAnimation, layout_cards
    card_constants.rs            # Sizes, z-step, play-pile x, hand fan params
assets/fonts/
  NotoSans-Regular.ttf           # Rank text
  NotoSansSymbols2-Regular.ttf   # Suit glyphs (♥♦♣♠)
```

## Key Architecture Notes

- **ECS via Bevy** — game logic lives in systems; data lives in components and resources.
- **`GameState`** ([game.rs:7](src/components/game.rs#L7)) is the central resource. Tracks players, draw/discard piles, `cards_in_play`, `current_player`, `phase`, `effective_rank`, `seven_active`, `any_card_playable`, `needs_to_pickup`, `selected_cards`, `pending_refill`/`refill_timer`, `winner`.
- **Four players**: human at index 0 (bottom centre), three AIs at top-left/centre/right. Each player has `face_down_cards`, `face_up_cards`, and `hand` (3 each at deal).
- **Z-index layers** ([card_constants.rs](src/rendering/card_constants.rs)):
  - Face-down table cards: 0–2 | Face-up table cards: 100–102 | Hand: 200–202
  - Draw pile: ~400 (descending) | Pickup highlight: 490 | Play pile: 500+ | Discard pile: 450– | Pile-status text: 600
- **Suit rendering**: real Unicode symbols (♥♦♣♠) rendered with `NotoSansSymbols2-Regular.ttf` — rank uses `NotoSans-Regular.ttf`. Two separate `Text2dBundle` children per card so each glyph uses the right font.
- **Table layout**: human at bottom centre; three AIs spaced across the top. Hands are fanned (rotated, arced) at the near window edge. Face-down cards sit further from the play pile, face-up closer.
- **Play pile** is anchored at `PLAY_PILE_X = 150.0`. Only the top *finished-animating* card shows its rank/suit text (`show_text`); the rest are face-up but hidden so the stack reads cleanly.

## Shithead Rules — Implemented

**Hand management**
- Deal: 3 face-down, 3 face-up, 3 hand per player.
- While draw pile not empty → play from hand only; refill hand to 3 after playing.
- When draw pile empty: hand → face-up → face-down (strict order).
- Can't play → must pick up the entire `cards_in_play` stack into hand.

**Special cards**
- **2** — resets the pile; any card valid next (`any_card_playable = true`).
- **3** — transparent; effective rank and special flags unchanged.
- **7** — next player must play ≤ 7 (`seven_active = true`).
- **10** — burns the pile (moves to `discard_pile`); same player goes again.
- **4-of-a-kind** at the top of the pile (across one or multiple plays) — burns the pile; same player goes again.
- Otherwise: next player must play `>= effective_rank`.

**Multi-card play**
- Click multiple same-rank cards to stage them (green tint, raised).
- Click again to deselect. Selecting a different rank starts fresh.
- Confirm with **Enter** or the "Play Cards" button. Cancel with **Escape**.
- Max 4 staged cards.

**Win condition**
- A player wins when their hand, face-up, and face-down are all empty.
- `phase = GameOver`, full-screen overlay shows "You Win!" / "You Lose!".
- Press any key to restart — all cards despawned and a fresh deal begins.

## UI / Feedback

- **Pile status text** ([game_plugin.rs:858](src/game_plugin.rs#L858)) above the play pile: "Play 7 or lower", "Play X or higher", "Play anything".
- **Invalid play**: clicked card flashes red (0.5s `invalid_timer`); pile-status text turns orange (2s `InvalidFeedbackTimer`).
- **Hover**: human's playable cards lift and tint yellow.
- **Selected**: staged cards tint green and raise.
- **Pickup prompt**: pulsing yellow highlight behind the play pile when the human can't play. Click the pile or press **Space** to pick up.
- **Play Cards button**: bottom centre; turns green when at least one card is staged.

## Controls

| Input | Action |
|---|---|
| Click card | Stage / deselect (only on human's turn) |
| Enter | Confirm staged play |
| Escape | Clear staged selection |
| Play Cards button | Confirm staged play |
| Click play pile | Pick up cards (when prompt is active) |
| Space | Pick up cards (when prompt is active) |
| Any key (on Game Over) | Restart |

## AI Behaviour

- One `AITimer` (1.5s) gates all AI turns.
- On its turn, an AI picks up if it has no valid play, otherwise plays the **lowest valid normal card**, falling back to 3 → 10 → 2 in that priority. Specials are saved as escape hatches.
- AI plays one card per tick (no multi-card play).
- AI hands refill immediately on play. The human refill is *deferred* (`pending_refill` + `refill_timer`) so the replacement cards animate in after the played card lands.

## Known Gaps / Future Work

- **Jack, Queen, King, Ace** — no special behaviour; treated as normal high cards (J=11, Q=12, K=13, A=14).
- AI doesn't bundle same-rank multi-card plays.
- No scoring / streaks / "who is the shithead" tracking across rounds.
- Architecture: `GameState` still mixes game logic with entity references; no event-based system communication yet (most state transitions still happen via direct resource mutation in input/AI systems).
- Suit symbols rely on a non-default font asset (`NotoSansSymbols2-Regular.ttf` — fetched by `scripts/download-fonts.sh`).
- `NotoSans-Regular.ttf` from the script is actually the *variable* `NotoSans[wdth,wght].ttf` renamed; google/fonts doesn't ship the static Regular. Bevy renders the default axis fine, but swap in a static TTF if rendering ever looks off.
