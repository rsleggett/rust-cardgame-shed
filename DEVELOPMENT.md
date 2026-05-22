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
- A 188 MB `NotoColorEmoji.ttf` and unused `card_suits.{png,svg}` files used to be tracked; they were removed from the index when fonts moved out of git.

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
- On pick-up, the full vec is drained into the player's hand and `effective_rank` / `seven_active` / `any_card_playable` are reset.
- On burn (10 or 4-of-a-kind), the vec is drained into `discard_pile` instead.

### Special Cards (implemented)
- **2** — `any_card_playable = true`, `effective_rank = None`, `seven_active = false`. Next player can play anything.
- **3** — transparent; `effective_rank` and special flags are left unchanged. Useful as filler that doesn't reset the trick.
- **7** — `seven_active = true`, `effective_rank = Some(Seven)`. Next must play ≤ 7.
- **10** — burns the pile, same player goes again.
- **4-of-a-kind at top** — checked in `play_selection`; if the top 4 cards in the pile share a rank (within a single play or across plays), the pile burns and the player goes again.
- All special handling lives in `play_card` (single-card / AI) and `play_selection` (multi-card / human). These two paths duplicate the rule logic — a future refactor target.

### Multi-Card Selection (human only)
- Click toggles `selected_cards`. Selecting a different rank replaces the selection.
- Up to 4 cards can be staged (matches the 4-of-a-kind burn).
- `confirm_play_system` (Enter / Escape) and `handle_play_button` both feed into `play_selection`.

### Win / Game Over / Restart
- `GamePhase::GameOver` set when a player empties all three card sets (hand, face-up, face-down).
- `game_over_screen_system` spawns a fullscreen overlay with the result.
- `restart_game_system` listens for any keypress, despawns all `Card` / `GameOverScreen` / `PileStatusText` entities, replaces `GameState` with a fresh one, re-adds the four players, and re-runs `prepare_dealing`.

### Hand Refill Timing
- AI: refilled synchronously inside `play_card`.
- Human: deferred via `pending_refill` + `refill_timer` (0.45s) so the played card's animation completes before new cards animate in from the draw pile centre. Without this delay the refill animation overlaps the play animation and looks chaotic.

---

## ECS Architecture

### Current Shape
- `GameState` holds *both* game-rule state (turn, phase, flags) *and* entity references (player card vecs, pile vecs). This is convenient but mixes concerns.
- Systems mutate `GameState` directly rather than dispatching events. The one event currently in use is `InvalidCardClicked`, which carries visual feedback only.
- `update_card_face_up_state` reads game state and writes component flags every frame — works but is essentially a reconciler.

### Plugin Layout
- `GamePlugin` ([game_plugin.rs:38](src/game_plugin.rs#L38)) registers everything: resources, the `InvalidCardClicked` event, `setup_game`, and the long Update tuple.
- `CardRendererPlugin` ([card_renderer.rs:19](src/rendering/card_renderer.rs#L19)) owns camera setup, the pickup highlight sprite, animation, layout, and visuals.
- Plugins are not currently ordered with `.before()` / `.after()`; the system tuple order is the only sequencing.

### Known Architectural Issues
- Rule logic is duplicated between `play_card` (single, AI) and `play_selection` (multi, human).
- `GameState` should arguably split into `GameRules` (transient flags, turn, phase) and `GameBoard` (entity collections).
- Most state transitions happen via direct resource mutation — would benefit from an event layer (`CardPlayedEvent`, `PileBurnedEvent`, `TurnEndedEvent`).
- The `update_game_state` system is currently a no-op left as a placeholder.

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

### Hover / Selection Raise
- Hovered or selected hand cards are pushed 20px toward the play pile (positive for bottom, negative for top). Done inside `layout_cards` so it stays in lockstep with the rest of the layout each frame.

---

## Outstanding Work

### Gameplay
- Add special-card behaviour for **J, Q, K, A** (e.g. skip turn, reverse direction, force lowest, etc.) — pick a rule variant and implement.
- AI multi-card play: bundle same-rank cards when available rather than drip-feeding them.
- Score / streak tracking across rounds; track who finishes last.
- Animate pile pickups (cards fly back to hand instead of teleporting).

### UX / Polish
- Visual indicator for pile size (count badge, fanned ghost cards).
- Animate the 10/4-of-a-kind burn (cards fly off to a discard area).
- AI turn indicator (whose turn is it, highlight their seat).
- Settings: AI count, AI speed, optional rule variants.

### Architecture
- De-duplicate `play_card` and `play_selection`.
- Split `GameState` into rules + entity references.
- Introduce events for card-played / pile-burned / turn-changed; let visual and audio systems subscribe instead of polling resource state.
- Add system ordering for the systems whose order currently matters implicitly (e.g. `update_card_face_up_state` must run after `play_card` and before `update_card_visuals`).

### Quality
- No automated tests yet. Card rule logic in `can_play_card` and the burn detection in `play_selection` are pure functions that could be unit-tested without Bevy.
- Manual test loop: `cargo run`, deal, play a few hands per AI, trigger a 10-burn, trigger a 4-of-a-kind burn, force a pickup, watch the game over / restart flow.

---

## Reference: System Wiring

`GamePlugin` Update systems, in declared order ([game_plugin.rs:48](src/game_plugin.rs#L48)):

| System | Purpose |
|---|---|
| `update_game_state` | placeholder, currently no-op |
| `update_hovered_card` | raycast cursor → entity, write `HoveredCard` |
| `handle_mouse_input` | stage cards / handle pile pickup click |
| `handle_invalid_card_event` | drive red flash + orange text on invalid click |
| `confirm_play_system` | Enter / Escape for staged plays |
| `handle_play_button` | bottom-centre "Play Cards" button |
| `update_play_button_style` | green when armed, grey otherwise |
| `deal_cards_system` | tick `DealTimer`, deal next card |
| `update_card_face_up_state` | reconcile `is_face_up` / `show_text` / hover / selection flags |
| `draw_first_card_system` | flip the starter card after dealing finishes |
| `draw_refill_system` | deferred refill for the human's hand |
| `check_valid_plays_system` | set `needs_to_pickup` when active player has no legal move |
| `handle_card_pickup_system` | Space picks up for the human |
| `ai_player_system` | tick `AITimer`, pick the best legal play |
| `update_pile_status_text` | rule-text label above the pile |
| `game_over_screen_system` | spawn overlay on `GameOver` |
| `restart_game_system` | any keypress on game over → fresh deal |

`CardRendererPlugin` Update systems ([card_renderer.rs:22](src/rendering/card_renderer.rs#L22)):

| System | Purpose |
|---|---|
| `update_card_animations` | lerp `Transform` toward `CardAnimation.target_position` |
| `layout_cards` | per-frame layout for hands, table cards, draw / play / discard piles |
| `update_card_visuals` | sprite color + text visibility from `Card` flags |
| `update_pickup_highlight` | pulse the yellow highlight when pickup is required |
