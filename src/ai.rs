//! AI play strategies. One function per personality, dispatched by `choose_play`.
//!
//! The caller (game_plugin::ai_player_system) is responsible for picking the
//! active source pile and filtering to legal plays via `can_play_card`. This
//! module just decides *which* of those legal cards to play, and whether to
//! bundle multiple same-rank cards.

use bevy::prelude::*;

use crate::components::card::{Card, Rank};
use crate::components::game::Personality;

/// Returns 1+ same-rank entities to play. Empty Vec means "no play — pick up".
///
/// `candidates` must already be filtered through `can_play_card` and must all
/// come from the same source pile. `from_face_down` short-circuits strategy
/// because cards in the face-down pile are nominally unknown.
pub fn choose_play(
    personality: Personality,
    candidates: &[Entity],
    cards: &Query<&Card>,
    from_face_down: bool,
) -> Vec<Entity> {
    if candidates.is_empty() {
        return Vec::new();
    }
    if from_face_down {
        // Blind flip — first card in the pile is fine; rank is irrelevant.
        return vec![candidates[0]];
    }
    match personality {
        Personality::Rob => choose_rob(candidates, cards),
        Personality::Mike => choose_mike(candidates, cards),
        Personality::Dave => choose_dave(candidates, cards),
    }
}

/// 2, 3, 10 are "specials" with non-rank effects. Most strategies treat them
/// separately from normal high/low play. (7 is *also* special-effect for
/// gameplay, but Rob is happy to throw 7s with the normals; Mike is not — see
/// `choose_mike`.)
fn is_basic_special(r: Rank) -> bool {
    matches!(r, Rank::Two | Rank::Three | Rank::Ten)
}

/// Buckets candidates by rank, ascending. Each entry: (rank, all cards of that rank).
fn group_by_rank(candidates: &[Entity], cards: &Query<&Card>) -> Vec<(Rank, Vec<Entity>)> {
    let mut buckets: Vec<(Rank, Vec<Entity>)> = Vec::new();
    for &e in candidates {
        if let Ok(card) = cards.get(e) {
            if let Some(b) = buckets.iter_mut().find(|(r, _)| *r == card.rank) {
                b.1.push(e);
            } else {
                buckets.push((card.rank, vec![e]));
            }
        }
    }
    buckets.sort_by_key(|(r, _)| *r as u8);
    buckets
}

/// **Rob** — gung-ho + smart. Bundles same-rank, proactive with burns, but
/// otherwise plays the lowest normal to keep options open.
fn choose_rob(candidates: &[Entity], cards: &Query<&Card>) -> Vec<Entity> {
    let groups = group_by_rank(candidates, cards);

    // 1. 4-of-a-kind → burn the pile.
    if let Some((_, ents)) = groups.iter().find(|(_, e)| e.len() >= 4) {
        return ents.iter().take(4).copied().collect();
    }
    // 2. Bundle any 2+ same-rank normals to dump faster.
    if let Some((_, ents)) = groups.iter().find(|(r, e)| !is_basic_special(*r) && e.len() >= 2) {
        return ents.clone();
    }
    // 3. Lowest normal single (7 included — Rob is fine throwing 7s).
    if let Some((_, ents)) = groups.iter().find(|(r, _)| !is_basic_special(*r)) {
        return vec![ents[0]];
    }
    // 4. Specials in order: 3 (free) → 10 (proactive burn) → 2 (last-ditch reset).
    for special in [Rank::Three, Rank::Ten, Rank::Two] {
        if let Some((_, ents)) = groups.iter().find(|(r, _)| *r == special) {
            return vec![ents[0]];
        }
    }
    Vec::new()
}

/// **Mike** — cunning, hoards. Always plays exactly one card, treats 7 as a
/// reserved weapon, blows specials only when there's nothing else.
fn choose_mike(candidates: &[Entity], cards: &Query<&Card>) -> Vec<Entity> {
    let is_mike_special =
        |r: Rank| matches!(r, Rank::Two | Rank::Three | Rank::Seven | Rank::Ten);
    let groups = group_by_rank(candidates, cards);

    // 1. Lowest non-special normal.
    if let Some((_, ents)) = groups.iter().find(|(r, _)| !is_mike_special(*r)) {
        return vec![ents[0]];
    }
    // 2. Specials in hoarder order: 3 (free) → 7 (forces opponent low) → 10 → 2.
    for special in [Rank::Three, Rank::Seven, Rank::Ten, Rank::Two] {
        if let Some((_, ents)) = groups.iter().find(|(r, _)| *r == special) {
            return vec![ents[0]];
        }
    }
    Vec::new()
}

/// **Dave** — chaotic. Random rank, 50/50 bundle, will happily waste specials
/// by playing multiples of them.
fn choose_dave(candidates: &[Entity], cards: &Query<&Card>) -> Vec<Entity> {
    let groups = group_by_rank(candidates, cards);
    if groups.is_empty() {
        return Vec::new();
    }
    let pick = (rand::random::<f32>() * groups.len() as f32) as usize % groups.len();
    let (_, ents) = &groups[pick];
    if ents.len() >= 2 && rand::random::<f32>() < 0.5 {
        ents.clone()
    } else {
        vec![ents[0]]
    }
}
