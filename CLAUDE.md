# CLAUDE.md

## Project Overview

A Rust implementation of the card game **Shed** using the [Bevy](https://bevyengine.org/) ECS game engine (v0.14.2). One human player vs three AI opponents at a 4-seat table. The game plays in **matches** of multiple **rounds**: each round a buff is drafted, scores tick up by finish position, and the first player to the match target wins. Match runs are persistent (buffs stack across rounds Balatro-style); a new match resets everything.

## Build & Run

```bash
./scripts/download-fonts.sh  # one-time: fetch font assets (not tracked in git)
./scripts/download-music.sh  # optional: prints where to drop a lo-fi OGG track
cargo run                    # run the game
cargo build                  # build only
cargo check                  # fast type-check without linking
cargo clippy                 # lint
```

Fonts are gitignored. `scripts/download-fonts.sh` pulls them from the
`google/fonts` GitHub mirror into `assets/fonts/`. Re-run any time those files
go missing — the script is idempotent (skips files that already exist).

Music is also gitignored. `scripts/download-music.sh` does not bundle a track;
it just creates `assets/music/` and prints suggested CC0 sources. Drop a
lo-fi OGG at `assets/music/lofi_loop.ogg` to enable background music — the
game runs silently if the file is missing (Bevy logs a one-line warning).

## Source Structure

```
src/
  main.rs                        # App entry point — 1440x900 window
  lib.rs                         # Re-exports modules so tests/ can drive them
  game_plugin.rs                 # GamePlugin: resource registration, system wiring, setup_game
  audio.rs                       # BackgroundMusic, MusicMuted, setup_music, Ctrl+M toggle
  rules.rs                       # Pure predicates: can_play_card, is_burn (+ unit tests)
  ai.rs                          # Per-personality AI strategy (Rob / Mike / Dave)
  components/
    mod.rs
    card.rs                      # Card component, Suit, Rank
    card_visual.rs               # CardBundle, spawn_card_complete, update_card_visuals
    game.rs                      # GameState + Player; MatchState; Personality;
                                 # BuffKind, ActiveBuff; dealing logic (+ unit tests)
  rendering/
    mod.rs
    card_renderer.rs             # CardRendererPlugin, CardAnimation, layout_cards
    card_constants.rs            # Sizes, z-step, play-pile x, hand fan params
  systems/
    mod.rs
    dealing.rs                   # DealTimer, deal_cards_system, draw_first_card_system
    input.rs                     # Mouse + keyboard: hover, click staging, double-click,
                                 # InvalidCardClicked event, confirm_play (Enter/Escape)
    play.rs                      # play_selection, pickup, refill, check_valid_plays,
                                 # has_valid_play, target_hand_size
    ai_runner.rs                 # AITimer, ai_player_system (dispatches to crate::ai)
    swap.rs                      # SwapState, DoneSwapButton, swap input + AI heuristic
    draft.rs                     # DraftState, DraftScreen overlay, AI draft picks
    consumables.rs               # PeekRevealTimer, Mulligan (M) and Peek (P) handlers
    visuals.rs                   # update_card_face_up_state (per-frame card flags)
  ui/
    mod.rs
    play_button.rs               # PlayButton, click handler, style toggle
    score_hud.rs                 # Top-right round/score widget; display-name helpers
    pile_status.rs               # World-space "Play X or higher" text above the pile
    game_over.rs                 # Full-screen overlay + restart_game_system
tests/
  common/mod.rs                  # App fixtures + helpers shared across integration tests
  play_selection.rs              # Burn paths, rank effects, invalid → pickup
  pickup.rs                      # Full / half-pickup, empty pile, staged clear
  swap_heuristic.rs              # ai_swap_system greedy promotion + idempotency
  phase_transitions.rs           # advance_swap_phase + check_valid_plays_system
  has_valid_play.rs              # Source-priority predicate + face-down phase
assets/fonts/
  NotoSans-Regular.ttf           # Rank text
  NotoSansSymbols2-Regular.ttf   # Suit glyphs (♥♦♣♠)
```

## Key Architecture Notes

- **ECS via Bevy** — game logic lives in systems; data lives in components and resources.
- **`GameState`** ([game.rs:7](src/components/game.rs#L7)) is the round-scoped resource: turn, phase, players, draw/discard piles, `cards_in_play`, `finish_order`, `spectate_mode`, `effective_rank`, `seven_active`, `any_card_playable`, `needs_to_pickup`, `selected_cards`, `pending_refill`/`refill_timer`. Reset on every round restart.
- **`MatchState`** ([game.rs](src/components/game.rs)) is the match-scoped resource that survives round resets: `round`, `target`, `scores`, `personas` (AI lineup), `persistent_modifiers` (drafted buffs per seat), `previous_shed`, `match_winner`. Wiped only on new-match restart.
- **`DraftState`** ([systems/draft.rs](src/systems/draft.rs)) is the transient per-round draft state: `pools` (offered buffs per seat) + `picks` (chosen buff per seat). Populated on entry to `Drafting`, cleared by `apply_picks_system`.
- **Four players**: human at index 0 (bottom centre), three AIs at top-left/centre/right. Each AI has a `Personality` (Rob / Mike / Dave) randomly drawn at match start. Duplicates are disambiguated with numeric suffixes ("Dave", "Dave 2").
- **Z-index layers** ([card_constants.rs](src/rendering/card_constants.rs)):
  - Face-down table cards: 0–2 | Face-up table cards: 100–102 | Hand: 200–202
  - Draw pile: ~400 (descending) | Pickup highlight: 490 | Play pile: 500+ | Discard pile: 450– | Pile-status text: 600
- **Suit rendering**: real Unicode symbols (♥♦♣♠) rendered with `NotoSansSymbols2-Regular.ttf` — rank uses `NotoSans-Regular.ttf`. Two separate `Text2dBundle` children per card so each glyph uses the right font.
- **Table layout**: human at bottom centre; three AIs spaced across the top. Hands are fanned (rotated, arced) at the near window edge. Face-down cards sit further from the play pile, face-up closer.
- **Play pile** is anchored at `PLAY_PILE_X = 150.0`. Only the top *finished-animating* card shows its rank/suit text (`show_text`); the rest are face-up but hidden so the stack reads cleanly.

## Game Phases

```
Dealing → Swap → Drafting → Playing → GameOver → (key press) → Dealing → …
```

- **Dealing** — deals 9 cards per player at `DEAL_INTERVAL = 0.15s` apart.
- **Swap** — standard Shithead pre-play. Human clicks a hand card (highlights green) then clicks a face-up card to swap; repeat as desired; press the "Done Swapping" button to confirm. AIs greedily promote any hand card whose rank exceeds a face-up rank.
- **Drafting** — each seat picks one buff. Human via overlay click, AIs auto-pick randomly. Pool size 3 normally, 5 for the previous round's Shed.
- **Playing** — normal play; ends when one player remains (the "Shed").
- **GameOver** — full-screen overlay with finish-order ranking, +points this round, cumulative scores, and a CTA to either start the next round or a new match.

## Shed Rules — Implemented

**Hand management**
- Deal: 3 face-down, 3 face-up, 3 hand per player.
- While draw pile not empty → play from hand only; refill hand to 3 (or 4 with Big Hand) after playing.
- When draw pile empty: hand → face-up → face-down (strict order).
- Can't play → must pick up the entire `cards_in_play` stack into hand.
- **Face-down phase**: click is a blind flip — the card plays immediately without validation. If it bricks, the player picks up the pile (including that revealed card).
- **Face-up endgame**: any face-up may be staged; an invalid confirmation animates the staged cards onto the pile then triggers pickup, so they travel back to the hand with the rest of the stack.

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

**Finish order + "Shed" condition**
- A player who empties hand, face-up, and face-down is **eliminated** (pushed onto `finish_order`) and skipped on subsequent turns — but the round keeps going.
- The round ends when one player remains; that player is the **Shed**.
- When the *human* is eliminated mid-round, `spectate_mode` flips on and the AI tick rate drops from 1.5 s to 0.3 s so the rest of the round resolves in ~10–15 s.
- The Shed of round N plays first in round N+1 (traditional Shed punishment) and gets a bigger draft pool (5 vs 3).

## Match + Scoring

- Scoring: 1st = 3 pts, 2nd = 2, 3rd = 1, Shed = 0 (general formula: `N-1, N-2, …, 0`).
- `MATCH_TARGET = 10` cumulative points wins the match.
- Game Over overlay shows current finish order + per-player cumulative score; CTA changes between "Press any key for the next round" and "Press any key for a new match" based on `MatchState::is_match_over`.
- Top-right **score HUD** displays round number, target, per-player scores, and active buffs.

## Buff Draft (Phase 4)

Every round (including round 1) routes through `Drafting` between `Dealing` and `Playing`. Buffs are **cumulative across the match** and reset only on new-match restart.

| Buff | Type | Effect |
|---|---|---|
| Wild Twos | passive | Your 2s also burn the pile |
| Hot Hand | passive | Your same-rank triples burn (threshold 4→3) |
| Counter-7 | passive | You ignore the "play ≤ 7" restriction |
| Big Hand | passive | Your hand refills to 4 cards instead of 3 |
| Wild Kings | passive | Your Kings also burn the pile |
| Half Pickup | passive | Pile pickup discards half (oldest), keeps the newer half |
| Mulligan (M) | consumable | Once per round: swap your hand with your face-up cards |
| Peek (P) | consumable | Once per round: reveal your face-down cards for 3 s |

Pool generation excludes buffs the player already has. Consumables refresh each round (`ActiveBuff.used_this_round` resets in `add_match_players`). HUD shows consumables with `*` (ready) or `x` (used).

## UI / Feedback

- **Pile status text** above the play pile: "Play 7 or lower", "Play X or higher", "Play anything".
- **Invalid play**: clicked card flashes red (0.5s `invalid_timer`); pile-status text turns orange (2s `InvalidFeedbackTimer`).
- **Hover**: human's playable cards lift and tint yellow.
- **Selected**: staged cards tint green and raise.
- **Pickup prompt**: pulsing yellow highlight behind the play pile when the human can't play. Click the pile or press **Space** to pick up.
- **Play Cards button**: bottom centre; turns green when at least one card is staged.
- **Score HUD**: top-right widget — round, target, per-player scores, per-player active buffs.
- **Draft overlay**: full-screen between rounds; clickable buff rows with name + description.

## Controls

| Input | Action |
|---|---|
| Click card | Stage / deselect (only on human's turn) |
| Double-click card | Play that card immediately, bypassing staging |
| Enter | Confirm staged play |
| Escape | Clear staged selection |
| Play Cards button | Confirm staged play |
| Click play pile / Space | Pick up cards (when prompt is active) |
| Click hand → click face-up (during Swap) | Swap those two cards |
| Done Swapping button | End Swap phase |
| Click buff row (during draft) | Pick that buff for the round |
| M | Use Mulligan (if drafted, once per round) |
| P | Use Peek (if drafted, once per round) |
| Ctrl+M | Toggle background-music mute |
| Any key on Game Over | Continue to next round / new match |

## AI Behaviour

- `AITimer` ticks every 1.5 s in normal play, 0.3 s in spectate mode (human eliminated).
- Each AI seat has a `Personality` randomly assigned at match start:
  - **Rob** (gung-ho + smart): bundles same-rank plays, proactive 4-of-a-kind burns; otherwise plays the lowest valid normal card.
  - **Mike** (cunning, hoards): always single-card; treats 7 as a reserved weapon; saves 2 / 10 for emergencies.
  - **Dave** (chaotic): random rank choice, 50/50 bundle-or-single, will happily waste specials by bundling them.
- All AIs route through `play_selection` (same path as human plays). Human refills are deferred via `pending_refill`; AI hands refill inline.
- Face-down play is always a single blind card regardless of personality, for AI and human alike — `ai_player_system` skips the playability filter in face-down mode and `play_selection` routes a brick to pickup.
- During the Swap phase, each AI greedily promotes any hand card whose rank exceeds a face-up card's rank, picking the biggest gain each iteration until no improvement remains. Personality-aware swap preferences are a follow-up.

## Known Gaps / Future Work

- **Jack, Queen, Ace** still have no special behaviour (treated as normal high cards). Only the King has anything via the optional Wild Kings buff.
- **Animation polish** — pickup currently teleports cards back to the hand; burn cards teleport to discard; draft picks have no fanfare; pile size has no visual indicator. This is the next iteration focus.
- **AI draft picks are random** — personality-aware buff preferences (Mike hoards consumables, Dave picks chaotically) are a follow-up.
- **AI doesn't display its picks** during draft — the overlay only shows the human's options.
- **Big Hand visual gap** — AI hands rendering only allocates space for 3 cards; with Big Hand the 4th may overlap a sibling.
- **"Any key on Game Over"** is generous — M and P will also dismiss the overlay between rounds.
- **Test coverage**: 28 inline unit tests in `src/rules.rs` and `src/components/game.rs` (pure logic), plus 33 integration tests in `tests/` driving real Bevy `App`s against `play_selection`, `pickup_cards_in_play`, `ai_swap_system`, `advance_swap_phase`, `check_valid_plays_system`, and `has_valid_play`. Helpers in `tests/common/mod.rs`. Total: 61 tests, run via `cargo test`. Deferred: integration tests for `ai_player_system` / `ai_draft_system` (need a seedable RNG resource to be deterministic) and a full-round end-to-end test.
- **Architecture**: `GameState` still mixes game logic with entity references; no event-based system communication yet (most state transitions still happen via direct resource mutation in input/AI/draft systems).
- Suit symbols rely on a non-default font asset (`NotoSansSymbols2-Regular.ttf` — fetched by `scripts/download-fonts.sh`).
- `NotoSans-Regular.ttf` from the script is actually the *variable* `NotoSans[wdth,wght].ttf` renamed; google/fonts doesn't ship the static Regular. Bevy renders the default axis fine, but swap in a static TTF if rendering ever looks off.
