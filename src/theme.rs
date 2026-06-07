//! Arcade Felt palette + small helpers — the single source of truth for the
//! game's colours, special-card metadata, seat colours, and draft rarities.
//! Every UI/render module pulls from here so literals don't drift across files.
//! Translated from the design handoff's token table.

use bevy::prelude::*;

use crate::components::card::Rank;
use crate::components::game::{BuffKind, Personality};

// ── Felt / surfaces ────────────────────────────────────────────────────────
pub const FELT_BASE: Color = Color::srgb(0.071, 0.322, 0.224); // #125239
pub const FELT_DARK: Color = Color::srgb(0.039, 0.192, 0.133); // #0a3122
pub const FELT_INK: Color = Color::srgb(0.039, 0.165, 0.114); // #0a2a1d

/// Standard translucent felt-panel fill for HUD chips / list rows.
pub const PANEL: Color = Color::srgba(0.031, 0.118, 0.078, 0.62);
/// Heavier veil drawn behind modal overlays (draft / game over).
pub const VEIL: Color = Color::srgba(0.02, 0.071, 0.047, 0.62);

// ── Accents ────────────────────────────────────────────────────────────────
pub const MAGENTA: Color = Color::srgb(1.0, 0.243, 0.604); // #ff3e9a
pub const CYAN: Color = Color::srgb(0.153, 0.769, 0.847); // #27c4d8
pub const GOLD: Color = Color::srgb(1.0, 0.824, 0.243); // #ffd23e
pub const LIME: Color = Color::srgb(0.486, 1.0, 0.420); // #7cff6b
pub const PURPLE: Color = Color::srgb(0.608, 0.420, 1.0); // #9b6bff
pub const LOCKED_GREY: Color = Color::srgb(0.227, 0.317, 0.278); // #3a5147
pub const MUTED_TEXT: Color = Color::srgb(0.416, 0.514, 0.471); // #6a8378
pub const WHITE: Color = Color::WHITE;

// ── Paper cards ────────────────────────────────────────────────────────────
pub const CARD_PAPER: Color = Color::srgb(0.984, 0.973, 0.937); // #fbf8ef
pub const CARD_INK: Color = Color::srgb(0.090, 0.090, 0.125); // #171720
pub const CARD_RED: Color = Color::srgb(0.886, 0.231, 0.337); // #e23b56

// ── Special-card colours ────────────────────────────────────────────────────
const SPECIAL_2: Color = Color::srgb(0.153, 0.769, 0.847); // #27c4d8 reset
const SPECIAL_3: Color = Color::srgb(0.545, 0.576, 0.655); // #8b93a7 ghost
const SPECIAL_7: Color = Color::srgb(0.957, 0.718, 0.251); // #f4b740 under
const SPECIAL_10: Color = Color::srgb(1.0, 0.353, 0.235); // #ff5a3c burn

/// The neon ring/badge colour for a special card, or `None` for normal ranks.
pub fn special_color(rank: Rank) -> Option<Color> {
    match rank {
        Rank::Two => Some(SPECIAL_2),
        Rank::Three => Some(SPECIAL_3),
        Rank::Seven => Some(SPECIAL_7),
        Rank::Ten => Some(SPECIAL_10),
        _ => None,
    }
}

/// The bottom-centre word badge shown on a special card ("RESET" etc.).
pub fn special_badge(rank: Rank) -> Option<&'static str> {
    match rank {
        Rank::Two => Some("RESET"),
        Rank::Three => Some("GHOST"),
        Rank::Seven => Some("UNDER"),
        Rank::Ten => Some("BURN"),
        _ => None,
    }
}

/// The ink colour for a card's rank/suit, by suit.
pub fn card_text_color(is_red: bool) -> Color {
    if is_red { CARD_RED } else { CARD_INK }
}

// ── Seats ──────────────────────────────────────────────────────────────────
/// Seat accent colour. The human (seat 0) is gold; AIs take their personality
/// colour (Rob magenta, Mike purple, Dave lime).
pub fn seat_color(player_index: usize, personality: Personality) -> Color {
    if player_index == 0 {
        return GOLD;
    }
    match personality {
        Personality::Rob => MAGENTA,
        Personality::Mike => PURPLE,
        Personality::Dave => LIME,
    }
}

/// A short flavour "mood" tag shown under an AI seat's name.
pub fn seat_mood(personality: Personality) -> &'static str {
    match personality {
        Personality::Rob => "on a tear",
        Personality::Mike => "hoarding",
        Personality::Dave => "chaos!",
    }
}

// ── Chunky buttons ──────────────────────────────────────────────────────────
/// The solid drop-shadow colour under a chunky button — a darker shade of the
/// fill. Generic darken (~0.55) so any accent works without a lookup table.
pub fn chunky_shadow(fill: Color) -> Color {
    let c = fill.to_srgba();
    Color::srgb(c.red * 0.55, c.green * 0.55, c.blue * 0.55)
}

/// Lightens a colour toward white by `t` (0.0 = unchanged, 1.0 = white). Used
/// for hover states on solid-fill buttons where a darker shadow would read wrong.
pub fn lighten(color: Color, t: f32) -> Color {
    let c = color.to_srgba();
    Color::srgb(
        c.red + (1.0 - c.red) * t,
        c.green + (1.0 - c.green) * t,
        c.blue + (1.0 - c.blue) * t,
    )
}

// ── Draft rarities ──────────────────────────────────────────────────────────
#[derive(Clone, Copy)]
pub enum Rarity {
    Common,
    Rare,
    Epic,
    Gold,
}

impl Rarity {
    pub fn color(self) -> Color {
        match self {
            Rarity::Common => Color::srgb(0.604, 0.655, 0.706), // #9aa7b4
            Rarity::Rare => CYAN,
            Rarity::Epic => Color::srgb(0.690, 0.420, 1.0), // #b06bff
            Rarity::Gold => GOLD,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Rarity::Common => "COMMON",
            Rarity::Rare => "RARE",
            Rarity::Epic => "EPIC",
            Rarity::Gold => "GOLD",
        }
    }
}

/// Rarity tier + glyph for a draftable perk, used to style the draft overlay.
pub fn buff_rarity(kind: BuffKind) -> Rarity {
    match kind {
        BuffKind::WildTwos | BuffKind::Counter7 => Rarity::Rare,
        BuffKind::HotHand | BuffKind::WildKings => Rarity::Epic,
        BuffKind::BigHand | BuffKind::HalfPickup => Rarity::Gold,
        BuffKind::Mulligan | BuffKind::Peek => Rarity::Common,
    }
}

/// A single-character placeholder icon for a perk (replace with real art later).
pub fn buff_icon(kind: BuffKind) -> &'static str {
    match kind {
        BuffKind::WildTwos => "2",
        BuffKind::HotHand => "^",
        BuffKind::Counter7 => "7",
        BuffKind::BigHand => "+",
        BuffKind::WildKings => "K",
        BuffKind::HalfPickup => "%",
        BuffKind::Mulligan => "~",
        BuffKind::Peek => "o",
    }
}
