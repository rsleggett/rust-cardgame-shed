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

    for (card_entity, mut card) in cards.iter_mut() {
        card.is_hovered = hovered.0 == Some(card_entity);
        card.is_selected = game_state.selected_cards.contains(&card_entity);
        if card.invalid_timer > 0.0 {
            card.invalid_timer = (card.invalid_timer - time.delta_seconds()).max(0.0);
        }

        let mut is_in_player_hand = false;

        for (player_index, player) in game_state.players.iter().enumerate() {
            if player.face_up_cards.contains(&card_entity) {
                is_in_player_hand = true;
                card.is_face_up = true;
                card.show_text = true;
                break;
            }
            if player.face_down_cards.contains(&card_entity) {
                is_in_player_hand = true;
                // Peek reveals the human's face-down cards for a few seconds.
                let reveal = peek_active && player_index == 0;
                card.is_face_up = reveal;
                card.show_text = reveal;
                break;
            }
            if player.hand.contains(&card_entity) {
                is_in_player_hand = true;
                let face_up = player_index == 0; // only human sees their hand
                card.is_face_up = face_up;
                card.show_text = face_up;
                break;
            }
        }

        if !is_in_player_hand {
            let in_play = game_state.cards_in_play.contains(&card_entity);
            card.is_face_up = in_play;
            // Show text only on the topmost card that has finished its animation
            card.show_text = in_play && Some(card_entity) == top_visible;
        }
    }
}
