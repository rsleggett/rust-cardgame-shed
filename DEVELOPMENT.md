# Development Notes

A working log of design decisions, architecture choices, and known issues. CLAUDE.md is the snapshot of *what is*; this file captures *why* and *what's outstanding*.

---

## Card Rendering

### Suits and Fonts
- Rank text uses `NotoSans-Regular.ttf`.
- Suit glyphs use `NotoSansSymbols2-Regular.ttf`, which covers the Miscellaneous Symbols block (U+2660–U+2667 — ♠♡♢♣♤♥♦♧). The default Bevy font does not.
- Each card spawns two `Text2dBundle` children — one per font — so the rank and suit can be rendered with different typefaces.
- Earlier attempts: NotoColorEmoji (didn't render), `card_suits.png` sprite atlas (abandoned), plain-text placeholders like `<3` `<>` `<^>` `(^)` (replaced once the symbols font was added).

### Font Distribution
- Fonts are **not tracked in git** (`assets/fonts/*.ttf` is gitignored). Run `scripts/download-fonts.sh` after cloning.
- The script fetches from the `google/fonts` GitHub mirror. It's idempotent — re-runs skip files that already exist.
- google/fonts only ships the variable `NotoSans[wdth,wght].ttf`. The script saves it as `NotoSans-Regular.ttf` so `asset_server.load("fonts/NotoSans-Regular.ttf")` keeps working. Bevy 0.14 renders the default axis (Regular weight) without complaint.

### Card Bundle
- `CardBundle` ([card_visual.rs:7](src/components/card_visual.rs#L7)) bundles the `Card` component with its `SpriteBundle`.
- `spawn_card_complete` builds the full card hierarchy: bundle + dark border child + rank text child + suit text child.
- `update_card_visuals` is the single system that paints the card sprite each frame based on `Card` flags (`is_face_up`, `is_selected`, `is_hovered`, `invalid_timer`) and toggles text visibility via `show_text`.

### Z-Bleed Fix
- Stacked cards in the play pile were previously showing rank/suit text from cards below the top, causing visual bleed.
- Fix: `update_card_face_up_state` marks every pile card as `is_face_up = true` but only sets `show_text = true` on the topmost *finished-animating* entry. The text on lower cards is hidden via `Visibility::Hidden` in `update_card_visuals`.
- Using "topmost finished-animating" (not just the last entry) prevents the text from switching to an incoming card before its lerp animation lands.

---

## Game Mechanics

### Cards in Play Stack
- `cards_in_play: Vec<Entity>` holds the entire pile. `current_card` is the most recent push.
- On pick-up, the full vec is drained into the player's hand and `effective_rank` / `seven_active` / `any_card_playable` are reset. With the Half Pickup buff active, the older half goes to `discard_pile` instead — only the newer half (rounded up) reaches the hand.
- On burn (10 or 4-of-a-kind), the vec is drained into `discard_pile` instead.

### Special Cards (implemented)
- **2** — `any_card_playable = true`, `effective_rank = None`, `seven_active = false`. Next player can play anything.
- **3** — transparent; `effective_rank` and special flags are left unchanged. Useful as filler that doesn't reset the trick.
- **7** — `seven_active = true`, `effective_rank = Some(Seven)`. Next must play ≤ 7 (unless they have Counter-7).
- **10** — burns the pile, same player goes again.
- **4-of-a-kind at top** — checked in `play_selection`; if the top 4 cards in the pile share a rank (within a single play or across plays), the pile burns and the player goes again. With Hot Hand the threshold drops to 3.

### Multi-Card Selection (human + AI)
- Human: click toggles `selected_cards`; selecting a different rank replaces the selection. Up to 4 cards staged, confirm with Enter / "Play Cards" / cancel with Escape.
- AI: `ai_player_system` filters legal candidates through `can_play_card`, hands them to `ai::choose_play(personality, …)`, then routes the resulting `Vec<Entity>` through the same `play_selection` path. Personality decides single vs bundle.

### Hand Refill Timing
- AI: refilled synchronously inside `play_selection` when `playing_player != 0`.
- Human: deferred via `pending_refill` + `refill_timer` (0.45s) so the played card's animation completes before new cards animate in from the draw pile centre. Without this delay the refill animation overlaps the play animation and looks chaotic.
- Big Hand bumps the refill target to 4 (in both paths) via `target_hand_size`.

---

## Phase 1 — Finish Order + Spectate Speedup

**Problem.** Originally the game ended the moment any player emptied their hand/face-up/face-down, which collides with how Shed actually works (the *last* player is the Shed, finishing first is *winning*).

**Decisions.**
- `GameState.winner: Option<usize>` replaced by `finish_order: Vec<usize>`. `Player.eliminated: bool` cached for cheap reads in the hot turn-advance loop.
- Two helpers on `GameState`: `check_and_eliminate(player_index)` and `advance_to_next_active()`. The 4 win-check sites and 5 turn-advance sites collapse into these.
- `GameOver` fires only when `finish_order.len() + 1 == players.len()` — at that point the last active seat is also pushed onto `finish_order` and the phase flips.
- Burn paths (10 or 4-of-a-kind) had to thread the "if eliminated, advance instead of going again" branch.
- **Spectate speedup.** `spectate_mode: bool` flips on when seat 0 is eliminated. `ai_player_system` reconciles `AITimer` duration each tick: 1.5 s normally, 0.3 s in spectate. No explicit input gating needed — `current_player` will never return to 0 once the human is out, so all existing `current_player == 0` guards naturally hold.

---

## Phase 2 — MatchState + Scoring + Rounds

**Problem.** Every round was identical; no notion of "best of N" or carrying state across rounds.

**Decisions.**
- New `MatchState` resource separate from `GameState`. Survives round teardown; only reset on new-match restart (`*match_state = MatchState::new(...)`).
- Scoring formula: `N - 1 - position` (3/2/1/0 for 4 players). Shed always 0.
- `award_round` is idempotent (`current_round_scored` flag, reset by `start_next_round`) so it can be safely called from per-frame systems.
- Match winner = first cumulative score to hit `MATCH_TARGET = 10`; tied scores broken by highest cumulative (stable iteration order).
- **Restart split.** `restart_game_system` branches on `match_state.is_match_over()`. New match: fully reset `MatchState`. Next round: bump round, keep scores, and set `current_player = previous_shed` so the previous Shed plays first (Shed punishment).
- Score HUD lives in its own widget at top-right, persists across restarts, just reads state each frame.

---

## Phase 3 — AI Personalities + Multi-Card AI

**Problem.** Three AIs were identical; AI couldn't bundle multi-card plays; dealing animation was too slow.

**Decisions.**
- `Personality { Rob, Mike, Dave }` enum on `Player`. `MatchState.personas: Vec<AiPersona>` (display name + personality) populated by `MatchState::generate_personas` — random with replacement, duplicates suffixed (`"Dave"`, `"Dave 2"`). Personas persist across rounds; reshuffle on new match.
- Personality logic split into `src/ai.rs`. Single entry point `choose_play(personality, candidates, cards, from_face_down) -> Vec<Entity>`; one private function per personality. Face-down play is always blind and personality-agnostic.
- **AI now uses `play_selection`** (the same path as the human). `play_selection`'s refill is branched: defer for `playing_player == 0`, inline for AI. `play_card` was deleted entirely — the rule logic lives in one place now.
- `DEAL_INTERVAL` constant (0.15s) replaces the hardcoded 0.5s. Total deal time dropped from ~18s to ~5s.
- `add_match_players` helper centralises the four `add_player` calls used by `setup_game` and both branches of `restart_game_system`.

---

## Phase 4 — Per-Round Buff Draft

**Problem.** Rounds were mechanically identical; no Balatro-style "run" feel; no rubber-band for the Shed.

**Decisions.**
- New `GamePhase::Drafting` between `Dealing` and `Playing`. `deal_next_card` transitions to `Drafting` (not `Playing`); `apply_picks_system` flips to `Playing` once every seat has chosen.
- **Cumulative buffs across the match.** `Player.modifiers: Vec<ActiveBuff>` grows each round. Survives round teardown via `MatchState.persistent_modifiers` (per-seat snapshot). `add_match_players` restores them on every round restart and resets each consumable's `used_this_round` to false. New match → fresh `MatchState` → empty persistent modifiers.
- **Shed rubber-band: bigger draft pool.** Pool size 5 for the previous Shed, 3 for everyone else. Round 1 (no previous Shed) → everyone gets 3. Pool generation excludes buffs the player already has, so duplicates are avoided.
- **Buff catalogue (v1).** 6 passives + 2 consumables — see the table in CLAUDE.md. We swapped the originally planned "Steady Hand" and "Sticky Fingers" for "Wild Kings" and "Half Pickup" — the originals required new UI for partial-pile choices, the replacements drop in cleanly with existing rule paths.
- **Consumables refresh, don't deplete.** A consumable buff stays in `modifiers` forever; only `used_this_round` resets per round. So drafting Mulligan in round 1 means a Mulligan available every round of the match.
- **Rule-path hooks.** `can_play_card` gained a `has_counter7: bool` parameter (4 call sites updated). `play_selection`'s burn check now reads `HotHand` (lowers threshold), `WildTwos`, `WildKings` per-player. `target_hand_size(player)` drives both refill loops (Big Hand = 4). `pickup_cards_in_play` splits the pile when Half Pickup is active.
- **Peek visual** is a `PeekRevealTimer` resource (`f32`). `update_card_face_up_state` reads it and reveals the human's face-down cards while it's positive. `tick_peek_timer` decrements each frame.
- **AI draft picks are random** — personality-aware preferences are deliberately out of scope for v1.

### Trade-offs accepted
- **Single overlay, no AI pick reveal.** The draft UI shows the human's options only; AI picks happen invisibly. Cleaner UX, less coupling, easier to extend later.
- **No pool exhaustion handling beyond an `Mulligan` fallback** for AI seats. With 8 buffs and 5-round matches, hitting empty is rare.
- **No duplicate suppression in `apply_picks_system` for the *human*.** The pool already excludes owned kinds, so a duplicate can only happen via the AI fallback path; `apply_picks_system` guards anyway.

---

## Swap Phase

**Problem.** Standard Shed lets you reshuffle your dealt hand into your face-up row before play begins. The game jumped straight from dealing to play, so a bad face-up roll was unrecoverable.

**Decisions.**
- New `GamePhase::Swap` between `Dealing` and `Drafting`. `SwapState` (transient resource) tracks the human's currently-selected hand card.
- Human: click a hand card (highlights green), then click a face-up card to swap the two; repeat freely. The "Done Swapping" button (`DoneSwapButton`, shares the play button's slot, toggled by `update_swap_button_visibility`) confirms and advances.
- AI: `ai_swap_system` greedily promotes any hand card whose rank beats a face-up card, picking the biggest gain each iteration until no improvement remains. Idempotent once optimal. Personality-aware swap preferences are deferred.
- `advance_swap_phase` flips to `Drafting` once the human presses Done and the AIs have settled.

---

## Background Music

- `audio.rs` owns `BackgroundMusic` + `MusicMuted` and the `setup_music` startup system, plus `toggle_music_mute` (Ctrl+M).
- Music is **not tracked in git**; `scripts/download-music.sh` only creates `assets/music/` and prints CC0 sources. Drop a loop at `assets/music/lofi_loop.ogg` to enable it — the game runs silently (one-line Bevy warning) if the file is missing.

---

## ECS Architecture

### Current Shape
- `GameState` holds both round-scoped rule state and entity references (player card vecs, pile vecs). `MatchState` is the match-scoped sibling; `DraftState` is the draft-scoped sibling. Resource boundaries roughly match lifecycle.
- Systems mutate state directly rather than dispatching events. The one event currently in use is `InvalidCardClicked`, which carries visual feedback only.
- `update_card_face_up_state` reads game state and writes component flags every frame — works but is essentially a reconciler.

### Plugin Layout
- `GamePlugin` registers everything: resources, the `InvalidCardClicked` event, `setup_game` + `setup_music`, and three Update tuples (split because the system list crossed Bevy 0.14's 20-system tuple ceiling as draft, swap, and audio systems landed). System bodies live in `systems::*` / `ui::*` / `audio` — the plugin is wiring only.
- `CardRendererPlugin` owns camera setup, the pickup highlight sprite, animation, layout, and visuals.
- Plugins are not currently ordered with `.before()` / `.after()`; the system tuple order is the only sequencing.

### Known Architectural Issues
- `GameState` should arguably split into `GameRules` (transient flags, turn, phase) and `GameBoard` (entity collections).
- Most state transitions happen via direct resource mutation — would benefit from an event layer (`CardPlayedEvent`, `PileBurnedEvent`, `TurnEndedEvent`, `BuffPickedEvent`).

---

## Layout

### Four-Seat Table
- Human at index 0: bottom centre (`table_x = 0`, `face_y = -200`, `is_bottom = true`).
- AI 1: top left (`-440`).
- AI 2: top centre (`0`).
- AI 3: top right (`+440`).
- For top players the face-up row is rendered *below* the face-down row so the cards closer to the centre of the table are the face-up ones from each player's perspective.

### Hand Fan
- Constants in `card_constants.rs`: `HAND_FAN_STEP = 36px`, `HAND_FAN_ANGLE = 5°`, `HAND_FAN_ARC = 4px`.
- Each card in hand is offset horizontally from centre, rotated proportionally, and dropped vertically by `offset.abs() * arc`. Bottom and top players use opposite rotation signs so both fans curve "outward" from the table.
- **Big Hand caveat:** the fan layout assumes 3 hand slots. With Big Hand active the 4th card overlaps the 3rd visually (data is correct, layout isn't aware). Easy fix in a follow-up.

### Hover / Selection Raise
- Hovered or selected hand cards are pushed 20px toward the play pile (positive for bottom, negative for top). Done inside `layout_cards` so it stays in lockstep with the rest of the layout each frame.

---

## Outstanding Work

### Animation polish (next iteration focus)
- **Pile pickup**: cards teleport into the player's hand. Animate them flying back, especially for Half Pickup which splits to discard.
- **Burn**: 10 / 4-of-a-kind / Wild Twos / Wild Kings cards teleport to discard. Animate the burn off the pile (could be a satisfying flash + sweep).
- **Draft pick**: clicking a buff dismisses the overlay instantly. A brief confirm flourish would sell the choice.
- **Active-seat indicator**: nothing visually signals whose turn it is — pulsing border on the active seat would help.
- **Pile size**: no count badge or ghost stack — the pile size is invisible until you read the rule text.
- **AI turn telegraph**: AIs play with no warning. A small "thinking" indicator before the play would smooth the rhythm.

### Core gameplay loop (next iteration focus)
- **J, Q, A** still have no special behaviour. Pick a rule variant per rank (skip turn, reverse direction, force lowest, mirror last, etc.).
- **AI persona-aware draft picks**: Mike hoards consumables, Dave picks chaotically, Rob picks aggressive burn buffs.
- **More buffs**: cards with rare-tier rolls, anti-buffs (drawbacks for big rewards), buffs that target opponents.
- **Round-result peek**: show what each AI drafted in a panel after the round ends.
- **Per-round stat summary**: turns taken, biggest burn, etc. Adds texture to the score screen.

### UX / Polish
- Settings: AI count, AI speed, match target, optional rule variants.
- Pause / restart hotkey.
- "Any key on Game Over" should ignore `M` and `P` (those activate consumables and shouldn't double as advance-round).
- HUD font scaling / placement tuning.

### Architecture
- Split `GameState` into rules + entity references.
- Introduce events for card-played / pile-burned / turn-changed / buff-picked; let visual and audio systems subscribe instead of polling resource state.
- Add system ordering for the systems whose order currently matters implicitly.

### Quality
- **61 tests** today: 28 inline unit tests (`src/rules.rs`, `src/components/game.rs`) over pure logic, plus 33 integration tests in `tests/` that build real Bevy `App`s and exercise `play_selection`, `pickup_cards_in_play`, `ai_swap_system`, `advance_swap_phase`, `check_valid_plays_system`, and `has_valid_play`. Shared fixtures in `tests/common/mod.rs`. Run with `cargo test`.
- Still uncovered: `ai_player_system` / `ai_draft_system` (need a seedable RNG resource to be deterministic) and a full-round end-to-end test (deal → Shed across every phase).
- Manual test loop: `cargo run`, deal, swap a card or two, draft a buff, play a few hands per AI, trigger a 10-burn, trigger a 4-of-a-kind burn, force a pickup, play a consumable, watch finish-order + score across rounds, win a match.

---

## Reference: System Wiring

`GamePlugin` ([game_plugin.rs](src/game_plugin.rs)) only does wiring now — the system bodies live in `systems::*` and `ui::*`, with audio in `audio`. The Update schedule is split across **three** `add_systems(Update, ...)` calls because each block hits Bevy 0.14's 20-tuple ceiling. Startup runs `setup_game` + `setup_music`.

**Block 1 — core loop** (`systems::*`, `ui::*`):

| System | Purpose |
|---|---|
| `update_hovered_card` | raycast cursor → entity, write `HoveredCard` |
| `tick_last_click` | count down the double-click window (`LastClick`) |
| `handle_mouse_input` | stage cards / double-click play / pile pickup click |
| `handle_invalid_card_event` | drive red flash + orange text on invalid click |
| `confirm_play_system` | Enter / Escape for staged plays |
| `handle_play_button` | bottom-centre "Play Cards" button |
| `update_play_button_style` | green when armed, grey otherwise |
| `deal_cards_system` | tick `DealTimer`, deal next card |
| `update_card_face_up_state` | reconcile `is_face_up` / `show_text` / hover / selection / peek-reveal flags |
| `draw_first_card_system` | flip the starter card after dealing finishes |
| `draw_refill_system` | deferred refill for the human's hand |
| `check_valid_plays_system` | set `needs_to_pickup` when active player has no legal move |
| `handle_card_pickup_system` | Space picks up for the human |
| `ai_player_system` | tick `AITimer`, run `ai::choose_play`, dispatch via `play_selection` |
| `update_pile_status_text` | rule-text label above the pile |
| `update_score_hud` | round, target, scores, per-player buffs |
| `game_over_screen_system` | spawn overlay on `GameOver`, award round points |
| `restart_game_system` | any keypress on game over → next round or new match |

**Block 2 — draft + consumables** (`systems::draft`, `systems::consumables`):

| System | Purpose |
|---|---|
| `setup_draft_system` | populate `DraftState.pools` on entry to `Drafting` |
| `draft_screen_system` | spawn the human's clickable buff overlay |
| `handle_draft_click` | record human's pick on click |
| `ai_draft_system` | AI seats auto-pick instantly |
| `apply_picks_system` | push picks → `modifiers`, snapshot to `persistent_modifiers`, advance to `Playing` |
| `handle_mulligan_key` | M: swap hand ↔ face-up (consumes Mulligan) |
| `handle_peek_key` | P: arm `PeekRevealTimer` (consumes Peek) |
| `tick_peek_timer` | count down the peek reveal |

**Block 3 — swap + audio** (`systems::swap`, `audio`):

| System | Purpose |
|---|---|
| `handle_swap_input` | human click hand → face-up swap |
| `handle_done_swap_button` | "Done Swapping" confirms the phase |
| `ai_swap_system` | greedy AI hand → face-up promotion |
| `advance_swap_phase` | flip `Swap` → `Drafting` once settled |
| `update_swap_button_visibility` | show/hide the Done Swapping button by phase |
| `toggle_music_mute` | Ctrl+M background-music mute |
| `update_rules_info_panel` | rewrite the bottom-left rules/buffs panel text each frame |

`CardRendererPlugin` Update systems:

| System | Purpose |
|---|---|
| `update_card_animations` | lerp `Transform` toward `CardAnimation.target_position` |
| `layout_cards` | per-frame layout for hands, table cards, draw / play / discard piles |
| `update_card_visuals` | sprite color + text visibility from `Card` flags |
| `update_pickup_highlight` | pulse the yellow highlight when pickup is required |
