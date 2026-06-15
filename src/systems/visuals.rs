//! Per-frame card visual state — face-up/down, text visibility, hover/select
//! flags, and the invalid-flash countdown.

use bevy::prelude::*;

use crate::components::card::Card;
use crate::components::game::GameState;
use crate::rendering::card_renderer::CardAnimation;
use crate::systems::consumables::PeekRevealTimer;
use crate::systems::input::HoveredCard;

pub(crate) fn update_card_face_up_state(
    game_state: Res<GameState>,
    hovered: Res<HoveredCard>,
    peek_timer: Res<PeekRevealTimer>,
    animating: Query<Entity, With<CardAnimation>>,
    time: Res<Time>,
    mut cards: Query<(Entity, &mut Card)>,
) {
    // The topmost pile card that has FINISHED animating — this one shows its text.
    // Using last() would switch text to the incoming card before it arrives visually.
    let top_visible = game_state.cards_in_play.iter().rev()
        .find(|&&e| !animating.contains(e))
        .copied();

    let peek_active = peek_timer.0 > 0.0;
    let dt = time.delta_seconds();

    for (card_entity, mut card) in cards.iter_mut() {
        // Compute the target flags up front, then write each only if it actually
        // changed. `Mut<Card>` only marks the component dirty on assignment, so
        // guarding the writes keeps un-changed cards out of change detection —
        // which is what lets update_card_visuals' work scale with activity
        // rather than running for all 52 cards every frame (mobile-WebGL win).
        let new_hovered = hovered.0 == Some(card_entity);
        let new_selected = game_state.selected_cards.contains(&card_entity);

        let mut found = false;
        let mut new_face_up = false;
        let mut new_show_text = false;
        for (player_index, player) in game_state.players.iter().enumerate() {
            if player.face_up_cards.contains(&card_entity) {
                found = true;
                new_face_up = true;
                new_show_text = true;
                break;
            }
            if player.face_down_cards.contains(&card_entity) {
                found = true;
                // Peek reveals the human's face-down cards for a few seconds.
                let reveal = peek_active && player_index == 0;
                new_face_up = reveal;
                new_show_text = reveal;
                break;
            }
            if player.hand.contains(&card_entity) {
                found = true;
                let face_up = player_index == 0; // only human sees their hand
                new_face_up = face_up;
                new_show_text = face_up;
                break;
            }
        }
        if !found {
            let in_play = game_state.cards_in_play.contains(&card_entity);
            new_face_up = in_play;
            // Show text only on the topmost card that has finished its animation
            new_show_text = in_play && Some(card_entity) == top_visible;
        }

        if card.is_hovered != new_hovered { card.is_hovered = new_hovered; }
        if card.is_selected != new_selected { card.is_selected = new_selected; }
        if card.is_face_up != new_face_up { card.is_face_up = new_face_up; }
        if card.show_text != new_show_text { card.show_text = new_show_text; }
        // Only a live flash actually changes the timer; once it hits 0 the guard
        // stops further writes (and thus stops dirtying the card).
        if card.invalid_timer > 0.0 {
            card.invalid_timer = (card.invalid_timer - dt).max(0.0);
        }
    }
}
