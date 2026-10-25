# Handoff: Shed — "Arcade Felt" Visual Direction + Ante Ladder Hook

## Overview
This package is the polished visual direction for **Shed** (the Rust + Bevy card game — one human vs three AI, matches of rounds with a per-round buff draft). It covers a complete reskin in a Balatro-flavoured **"Arcade Felt"** style, plus a new **meta-hook**: the **Ante Ladder** (escalating difficulty across antes, with a stake ladder for long-tail replay).

It documents every screen, the card system, all design tokens, the interaction/juice spec, and the difficulty-scaling design so it can be built into the existing Bevy app.

## About the Design Files
The files in this bundle are **design references created in HTML/React (JSX)** — prototypes showing the intended look, layout, and behaviour. **They are not production code to copy.** The task is to **recreate these designs inside the existing Rust + Bevy codebase**, using its established patterns:

- Rendering / layout → `src/rendering/card_renderer.rs`, `src/rendering/card_constants.rs`
- Card visuals → `src/components/card_visual.rs`
- HUD / overlays / widgets → `src/ui/` (play button, score HUD, rules panel, pile status, game over)
- Game/match state, buffs, personalities → `src/components/game.rs`
- New systems (ante progression, hazards, stakes) → `src/systems/` + a new `src/ante.rs`

Colours, fonts, sizes, and shadows below are the spec. Bevy has no CSS, so translate them to `Color`, `bevy_ui` nodes, `Text`/`TextStyle`, sprite layers, and (where glows/gradients are needed) layered sprites or a small shader/9-slice. Treat the HTML as the source of truth for *appearance and feel*; reuse the existing ECS architecture for *implementation*.

## Fidelity
**High-fidelity.** Final colours, typography, spacing, card treatments, and HUD layout. Recreate pixel-faithfully. The one exception: small placeholder glyph icons (perk/dial icons like `↑ ◉ ☻ ♣`) are stand-ins — replace with real icon art when available.

---

## Design Tokens

### Palette
| Token | Hex | Use |
|---|---|---|
| Felt (bg) | radial `#1c6b4f → #125239 (45%) → #0a3122 (100%)` from 50% top | Table / all screen backgrounds |
| Felt ink | `#0a2a1d` | Deepest felt |
| Panel | `rgba(8,30,20,0.50–0.70)` | HUD chips, list rows, cards-on-felt |
| Magenta | `#ff3e9a` (dark `#b51c64`) | Primary accent, Rob, hazards, "NEW RUN" |
| Cyan | `#27c4d8` (dark `#127e8c`) | Secondary accent, kickers, info |
| Gold | `#ffd23e` (dark `#c39400`) | Highlight, active seat, titles, primary CTA |
| Lime | `#7CFF6B` (dark `#3da033`) | Success, points gained, "PLAY", Dave |
| Purple | `#9b6bff` | Mike |
| Locked grey | `#3a5147` | Locked ladder rungs |
| Muted text | `#6a8378` | Disabled / zero points |
| White | `#ffffff` | Primary text on felt |

The "dark" column is the solid drop-shadow colour for chunky buttons (see Buttons).

### Card palette (paper cards)
| Token | Hex |
|---|---|
| Card paper | `#fbf8ef` |
| Card ink (black suits/rank) | `#171720` |
| Card red (♥ ♦) | `#e23b56` |
| Card corner radius | `9px` (scale with card size) |
| Special: **2** RESET | `#27c4d8` |
| Special: **3** GHOST | `#8b93a7` |
| Special: **7** UNDER | `#f4b740` |
| Special: **10** BURN | `#ff5a3c` |
| Card back | linear `150deg, #241d4a → #15123a (60%) → #0e0b27`; neon lattice `#7c6cf7` / `#26d2d9`; centre disc emblem text `#cfc6ff` |

### Typography
- **Rubik** — UI font. Weights: titles **900**, buttons/labels **800**, body **400–700**. Card ranks **800**.
- **Silkscreen** — pixel display font for HUD numerals, kickers, status tags, the SHED logo, badges.
- Letter-spacing: logo `+2`, kickers `+2 to +6`, body default.
- Neon text-shadow recipe: `0 0 1px C, 0 0 6px C, 0 0 16px C`.

### Radii & shadows
- Buttons `radius 12`; cards `radius 9`; panels/rows `radius 11–14`; pills `radius 20`; small tags `radius 4–7`.
- Card shadow (rest): `0 3px 7px rgba(0,0,0,.28)`; (selected/raised): `0 14px 26px rgba(0,0,0,.45)`.
- Special-card glow ring: `0 0 0 2px <specialColor>, 0 0 16px <specialColor>88`.
- Chunky button: `0 5px 0 <darkColor>, 0 9px 16px rgba(0,0,0,.4)` + `2px solid rgba(255,255,255,.35)` border.
- Frame rim (every screen): `inset 0 0 0 3px rgba(255,210,62,.18), inset 0 0 90px rgba(0,0,0,.55)`.

### Design canvas
Mobile-portrait reference frame is **390 × 844**. Design mobile-first; landscape keeps the existing desktop table layout (see original README's orientation notes).

---

## Shared Component: the Card

One reusable card primitive drives every card on screen (see `shared.jsx` → `Card`). Props: `rank, suit, faceDown, w, h, sel, dim, glow, flat`.

- **Face:** paper background, rank+suit in top-left corner and rotated 180° in bottom-right, large centre suit glyph. Black suits use card-ink, red suits use card-red.
- **Special cards (2/3/7/10):** when `glow` is on, add the coloured glow ring AND a **word badge** centred at the bottom (`RESET / GHOST / UNDER / BURN`) — a solid pill in the special colour, white 800 8px uppercase text. This replaces the bottom-right corner index on specials. *This is the key readability feature: a player instantly sees which cards are special and what they do.*
- **Selected (`sel`):** translate up 12px + deeper shadow.
- **Invalid/spent (`dim`):** opacity 0.5.
- **Face-down (`faceDown`):** the neon card back.
- **Flat (`flat`):** no shadow — used for stacked/underlay cards.

Map to `card_visual.rs` / `card_renderer.rs`: the special badge + glow should be driven by `Rank` (and by `can_play_card`/`is_burn` predicates from `src/rules.rs` for the live "playable" highlight).

---

## Screens / Views

> Files: all screens are in `screens-a.jsx`. The canvas wrapper is `app.jsx` + `design-canvas.jsx` (the canvas chrome is presentation-only; ignore it for implementation).

### 1. Title / Main Menu  (`TitleA`)
- **Purpose:** Entry point; resume or start a run.
- **Layout:** centred column, padding `88px 30px 30px`.
  - Logo **"SHED"** — Silkscreen 700, 64px, white, magenta neon shadow, rotated `-2deg`.
  - Tagline "ROGUELIKE CARD GAME" — Rubik 700, 13px, cyan neon, letter-spacing 6.
  - Hero card fan: 4 cards (A♠, 2♥, 10♣, 7♦) at 64×90, fanned with rotation, `glow` on (specials show badges).
  - Button stack (bottom): `CONTINUE RUN` (gold, big, sub "ante 1 · stake white"), `NEW RUN` (magenta), then a row of `DAILY` + `COLLECTION` (cyan).
  - Footer "v0.4 · ♪ on" — Silkscreen 10, 50% opacity.

### 2. Play Table  (`TableA`) — **the core screen**
- **Purpose:** Play a hand of Shed. Portrait layout (the desktop table is preserved in landscape).
- **Layout (top → bottom):**
  - **Opponent strip** (`top:16`, space-around): three Seats — Rob, Mike, Dave.
  - **Scoreboard chip** (`top:14 right:12`): panel, Silkscreen, "ANTE n" (gold) + "YOU 0 · 1ST→10".
  - **Centre pile** (`top:322`, centred): deck (face-down 56×80 + "DECK n" caption) next to the play pile — current top card 64×90 with `glow`, two fanned underlay cards behind it (rotations ±), and a magenta **×N count badge** top-right of the pile.
  - **Turn prompt:** Silkscreen 14, gold neon — "▲ YOUR TURN · PLAY ≥ 9".
  - **Score pop (juice):** "+3!" in lime neon, rotated `-8deg`, appears on point gain.
  - **Active buffs HUD** (`bottom:232 left:10`): pill chips, outline in each buff's accent colour (e.g. Wild 2s = cyan, Hot Hand = magenta, Counter-7 = gold).
  - **Your face-up/face-down stacks** (`bottom:120`, centred): three pairs, a face-down card peeking under each face-up card (50×70).
  - **Your hand** (`bottom:26`, centred, reserves right gutter): 4 cards at 62×88; staged card uses `sel`, a playable special uses `glow`.
  - **PLAY button** (`bottom:30 right:12`): lime chunky button "PLAY ▸".
- **Seat component** (`SeatA`): 30px circle avatar (radial gradient → dark, 2px ring in the seat's colour, monogram), name (11px 700) + **mood tag** (Silkscreen 9, gold) e.g. Rob "‹ on a tear", Mike "…hoarding", Dave "chaos!". Mini face-down fan + a 3-card face-up/down row. **Active seat** gets a gold glow ring (`0 0 0 2px gold, 0 0 18px gold`). Seat colours: Rob magenta, Mike `#9b6bff`, Dave lime.

### 3. Buff Draft  (`DraftA`)
- **Purpose:** Pick one of three perks each round (the roguelike layer). Buffs stack for the whole run.
- **Layout:** dimmed table behind (`rgba(5,18,12,.62)` veil). Padding `70px 18px 26px`.
  - Kicker "ROUND 1 · ANTE 1" (cyan) → title "PICK A PERK" (Rubik 900, 30px, magenta neon) → subtitle "Buffs stack for the whole run. Choose 1 of 3."
  - Three **perk cards** (`PerkA`): rarity-bordered panel with glow, a 50px icon chip, name + rarity tag, blurb, optional gold synergy sub-line.
    - Rarity colours: common `#9aa7b4`, rare cyan, epic `#b06bff`, gold `#ffd23e`.
    - Example content: *Wild Twos* (rare), *Hot Hand* (epic, "synergises with Counter-7"), *Peek* (common, "consumable · refreshes each round").
  - Page dots + "REROLL POOL · $3" footer (Silkscreen).
- Note: the previous round's **Shed** gets a bigger pool (5 vs 3) — rubber-band, as in current game.

### 4. Round-End / Scoring  (`RoundEndA`)
- **Purpose:** Show finish order, points this round, cumulative run totals.
- **Layout:** veiled bg. Kicker "ROUND 2 COMPLETE" → title "FINISH ORDER" (gold neon).
  - Four **finish rows**: position (1st–4th, Silkscreen, in seat colour) + avatar + name + points (`+3/+2/+1/+0`, lime neon; +0 in muted grey). 1st place row tinted gold. The last-place player gets a small magenta **"THE SHED"** tag.
  - **Run total panel:** "RUN TOTAL · FIRST TO 10" + 4 score bars (track `rgba(255,255,255,.08)`, fill gradient `cyan→gold`, with numeric value).
  - Note line: "Dave was the Shed — they play first next round, with a bigger draft pool."
  - `CONTINUE ▸` lime chunky button.
- Scoring rule (from game): 3/2/1/0 points by finish position; previous Shed plays first next round.

### 5. Card Kit  (`CardsA`)
- **Purpose:** Reference sheet for the card system. Rows: **FACES** (A♠, 9♥, K♦), **SPECIALS · glow + badge** (2/3/7/10 with rings + badges), **STATES** (staged `sel`, invalid `dim`, back). Use this as the authoritative card spec.

### 6. Ante Ladder  (`HookA`) — **the hook**
- **Purpose:** The run map. Climb antes; each tightens difficulty. Beat the top to unlock the next stake.
- **Layout:** Kicker "THE RUN" → "ANTE LADDER" → subtitle "Win the match to climb. Each ante tightens the screws."
  - **Ante rungs** (vertical list): numbered node (44px, gold-glowing on the current "NOW" ante, locked rungs greyed) + a row showing `Ante n · <Stake colour>`, status (cleared / NOW / locked), and a detail line: **target** (`1ST→10/12/14`), **AI skill** as filled/empty stars `★★☆☆`, and a magenta **hazard tag** (`▲ Glass Sevens`) on boss antes.
  - **UNLOCKS** strip: row of buff icon chips collected this run + a `+N` more.
  - `ENTER ANTE 3 ▸` gold chunky CTA with sub "red stake · Glass Sevens · 1st→12".

### 7. Boss Ante (in play)  (`BossTableA` = `TableA` with `hazard`)
- **Purpose:** Show how a boss-ante hazard reads during play.
- Same play table, plus: a reddish radial tint over the felt, scoreboard shows the higher ante/target, and a **HAZARD banner** (`top:150`): hazard-stripe background, magenta border + glow, "▲ HAZARD · GLASS SEVENS" (Silkscreen) and a one-line rule explanation. Implement as a dismissible-on-first-turn banner that then collapses to the scoreboard.

### 8. Ante Difficulty (design doc)  (`AnteSpecA`)
Not a game screen — the **balance design** for the hook. Build the systems it describes:

**An ante = one full match** (first to its point target). Win → climb (fresh draft + a difficulty bump). Lose → run ends, meta unlocks kept. Power and pressure rise together.

**Dials that tighten (per ante):**
| Dial | What scales |
|---|---|
| Point target | first-to **10 → 12 → 14 → 16** |
| AI skill | rivals bundle / hoard / burn smarter (tie to personality logic in `src/ai.rs`) |
| Rivals draft too | from Ante 2, opponents also gain buffs (**+1 → +3** each) |
| Runway | high antes shrink your hand refill and cap pile pickups |
| Boss hazard | every **2nd** ante twists one rule for that match only |

**Boss hazards (one per boss ante):**
| Hazard | Effect |
|---|---|
| Glass Sevens | a 7 forces the next **TWO** plays ≤ 7 |
| Stacked Deck | each round the pile starts on a high card |
| No Mercy | pickup-softening buffs (e.g. Half Pickup) are disabled |
| Cold Tens | your 10s no longer burn — only opponents' do |
| Cull | last place each round loses a random buff |
| Twin Shed | finish bottom-two and the run ends |

**Stakes (meta ladder across runs — unlock by beating the top ante):**
| Stake | Permanent rule | Default |
|---|---|---|
| White | Base rules | unlocked |
| Red | Rivals open each round having ditched a low card | unlocked |
| Green | Half-Pickup & similar mitigations weakened | locked |
| Black | A random boss hazard runs on **every** ante | locked |
| Gold | All hazards harder, AI maxed | locked |

**Suggested reference table (Ante 1–8):**
| Ante | Target | AI | Rival buffs | Hazard |
|---|---|---|---|---|
| 1 | 10 | ★☆☆☆ | — | — |
| 2 | 10 | ★★☆☆ | +1 | — |
| 3 | 12 | ★★☆☆ | +1 | Glass Sevens |
| 4 | 12 | ★★★☆ | +2 | — |
| 5 | 14 | ★★★☆ | +2 | Stacked Deck |
| 6 | 14 | ★★★★ | +3 | — |
| 7 | 16 | ★★★★ | +3 | No Mercy |
| 8 | 16 | ★★★★★ | +3 (epic) | Twin Shed (final) |

---

## Interactions & Behavior
(Existing controls from the project README still apply: click/tap to stage, double-click to play immediately, Enter/Play to confirm, click pile/Space to pick up, etc.)

- **Card hover/stage:** staged card raises 12px with a deeper shadow; deselect lowers it.
- **Playable highlight:** legal plays (per `rules.rs::can_play_card`) get the glow ring; illegal cards dim to 0.5.
- **Special play feedback:** when a special resolves, pulse its glow; on **burn** (10 or 4-of-a-kind) play a brief flash + clear the pile with a sweep.
- **Score pop:** on points awarded, spawn a lime neon "+N!" near the pile, drift up + fade (~700ms, ease-out, rotate ~-8deg).
- **Active seat:** the current player's seat shows the gold glow ring; advance it on turn change.
- **Pile count badge:** magenta pill with `×N` reflecting pile size (addresses the "pile size indicator" future-improvement).
- **Boss-ante banner:** slides in at round start, collapses after the player's first turn.
- **Draft:** clicking a perk adds it (animate the chosen card to the buffs HUD). Reroll consumes currency.
- **Round-end:** one-key/tap continue. Ignore M/P here so consumable hotkeys don't double as advance (a noted bug to fix).
- **Reduced motion:** all juice (pops, glows, shakes) should respect a reduced-motion setting and degrade to instant state changes.

Motion language: snappy, springy, "arcade" — short durations (120–250ms), ease-out, slight overshoot on card raises and button presses. Buttons depress (translate down to meet their solid bottom shadow) on press.

## State Management
Extend the existing `GameState` / `MatchState` (`src/components/game.rs`). New state to add (suggested `src/ante.rs`):
- **AnteState:** `ante_index`, `point_target`, `stake: StakeColor`, `active_hazard: Option<Hazard>`, `ai_skill: u8`, `rival_buff_count: u8`, runway modifiers (hand-refill size, pickup cap).
- **RunState (meta, persisted):** highest stake unlocked, buff/persona/card-back collection, current run progress (for "CONTINUE RUN").
- **Enums:** `StakeColor { White, Red, Green, Black, Gold }`, `Hazard { GlassSevens, StackedDeck, NoMercy, ColdTens, Cull, TwinShed }`.
- Hazards should hook into the rule predicates: thread `&AnteState` into `can_play_card` / `is_burn` / pickup logic so a hazard can override behaviour for that match only.
- Transitions: win ante → increment ante, re-roll target/AI/hazard, fresh draft; lose → write meta unlocks, return to title. Reuse the existing event-based direction (`CardPlayedEvent`, `PileBurnedEvent`, etc. from the future-work list) where possible.

## Assets
- **Fonts:** Rubik + Silkscreen (Google Fonts). The project already fetches fonts via `scripts/download-fonts.sh` — add these two there. Silkscreen is a pixel display face; Rubik is the UI face.
- **No bitmap art** is used — everything is procedural (gradients, shadows, text). Card backs and the felt are pure gradient/box-shadow; recreate with layered sprites/quads or a small shader in Bevy.
- **Placeholder icons:** perk/dial glyphs (`↑ ◉ ☻ ♣ ◎ ▾ ▲ ½`) are stand-ins — commission/replace with real perk icons.
- **Audio:** unchanged from current build (background music + Ctrl+M mute).

## Files (in this bundle)
- `index.html` — entry; loads React + Babel + the scripts below.
- `shared.jsx` — the **Card** primitive, card backs, `MiniFan`, suit/special tables, the `SKINS` token map (Arcade Felt = skin `A`).
- `screens-a.jsx` — all eight screens/components (`TitleA, TableA, DraftA, RoundEndA, CardsA, HookA, BossTableA, AnteSpecA`) + theme tokens (`themeA`), buttons (`ChunkyA`), seats (`SeatA`).
- `app.jsx` — assembles the screens onto the design canvas (presentation only).
- `design-canvas.jsx` — the pan/zoom canvas chrome (presentation only; **not** part of the game UI).

To view: open `index.html`. Each frame is a 390×844 mobile screen; click a frame on the canvas to focus it.
