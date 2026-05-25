//! Standard Shed pre-play swap phase: human can swap hand ↔ face-up cards
//! before play begins; AIs greedily promote higher cards to face-up.

use bevy::prelude::*;

use crate::components::card::Card;
use crate::components::game::{GamePhase, GameState};
use crate::game_plugin::PLAYER_COUNT;
use crate::rendering::card_constants::{CARD_HEIGHT, CARD_WIDTH};

/// Bottom-centre button visible only during the Swap phase. Click → human is
/// done swapping. Shares the play button's slot via mutually exclusive
/// visibility.
#[derive(Component)]
pub(crate) struct DoneSwapButton;

/// Transient per-round state for the Swap phase. Reset on exit so the next
/// round begins with a clean slate.
#[derive(Resource, Default)]
pub(crate) struct SwapState {
    /// The human's currently-staged hand card waiting for a face-up partner.
    pub(crate) human_selected_hand: Option<Entity>,
    /// Which AIs have completed their swap heuristic this round. Indexed by
    /// seat - 1 (human is seat 0).
    pub(crate) ai_done: [bool; PLAYER_COUNT - 1],
    /// Set when the human clicks the Done Swapping button.
    pub(crate) human_done: bool,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_swap_input(
    windows: Query<&Window>,
    mut game_state: ResMut<GameState>,
    mut swap_state: ResMut<SwapState>,
    transforms: Query<&GlobalTransform>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    button_q: Query<&Interaction, With<DoneSwapButton>>,
) {
    if game_state.phase != GamePhase::Swap || swap_state.human_done {
        return;
    }
    if !mouse_button_input.just_pressed(MouseButton::Left) {
        return;
    }
    // The Done button overlaps the hand fan area; drop swap input when the
    // cursor is over the button so a Done click can't also swap a card.
    if button_q
        .iter()
        .any(|i| matches!(i, Interaction::Pressed | Interaction::Hovered))
    {
        return;
    }

    let (camera, camera_transform) = camera_q.single();
    let window = windows.single();
    let Some(cursor_pos) = window.cursor_position() else { return; };
    let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else { return; };

    let hand: Vec<Entity> = game_state.players[0].hand.clone();
    let face_up: Vec<Entity> = game_state.players[0].face_up_cards.clone();

    let hit_in = |source: &[Entity]| -> Option<Entity> {
        let mut found = None;
        for &e in source {
            if let Ok(t) = transforms.get(e) {
                let p = t.translation().truncate();
                if world_pos.x >= p.x - CARD_WIDTH / 2.0
                    && world_pos.x <= p.x + CARD_WIDTH / 2.0
                    && world_pos.y >= p.y - CARD_HEIGHT / 2.0
                    && world_pos.y <= p.y + CARD_HEIGHT / 2.0
                {
                    found = Some(e);
                }
            }
        }
        found
    };

    if let Some(hand_hit) = hit_in(&hand) {
        if swap_state.human_selected_hand == Some(hand_hit) {
            swap_state.human_selected_hand = None;
            game_state.selected_cards.retain(|&e| e != hand_hit);
        } else {
            game_state.selected_cards.clear();
            swap_state.human_selected_hand = Some(hand_hit);
            game_state.selected_cards.push(hand_hit);
        }
        return;
    }

    if let Some(fu_hit) = hit_in(&face_up) {
        let Some(hand_card) = swap_state.human_selected_hand.take() else { return; };
        if let Some(player) = game_state.players.get_mut(0) {
            if let (Some(h_pos), Some(f_pos)) = (
                player.hand.iter().position(|&e| e == hand_card),
                player.face_up_cards.iter().position(|&e| e == fu_hit),
            ) {
                player.hand[h_pos] = fu_hit;
                player.face_up_cards[f_pos] = hand_card;
            }
        }
        game_state.selected_cards.clear();
    }
}

pub(crate) fn handle_done_swap_button(
    game_state: Res<GameState>,
    mut swap_state: ResMut<SwapState>,
    interaction_q: Query<&Interaction, (Changed<Interaction>, With<DoneSwapButton>)>,
) {
    if game_state.phase != GamePhase::Swap {
        return;
    }
    for interaction in &interaction_q {
        if *interaction == Interaction::Pressed {
            swap_state.human_done = true;
        }
    }
}

/// Each AI greedily swaps any hand card whose rank exceeds a face-up card's
/// rank, picking the biggest gain each iteration until no improvement remains.
/// Runs once per round entry into Swap; the `ai_done` flags make subsequent
/// frames no-ops.
pub(crate) fn ai_swap_system(
    mut game_state: ResMut<GameState>,
    mut swap_state: ResMut<SwapState>,
    cards: Query<&Card>,
) {
    if game_state.phase != GamePhase::Swap {
        return;
    }
    let n_players = game_state.players.len();
    for ai_idx in 1..n_players {
        let slot = ai_idx - 1;
        if swap_state.ai_done.get(slot).copied().unwrap_or(true) {
            continue;
        }
        loop {
            let mut best: Option<(usize, usize, u8)> = None; // (hand_idx, fu_idx, gain)
            {
                let player = &game_state.players[ai_idx];
                for (h_idx, &h_e) in player.hand.iter().enumerate() {
                    let Ok(h_card) = cards.get(h_e) else { continue; };
                    for (f_idx, &f_e) in player.face_up_cards.iter().enumerate() {
                        let Ok(f_card) = cards.get(f_e) else { continue; };
                        let hr = h_card.rank as u8;
                        let fr = f_card.rank as u8;
                        if hr > fr {
                            let gain = hr - fr;
                            if best.map_or(true, |(_, _, g)| gain > g) {
                                best = Some((h_idx, f_idx, gain));
                            }
                        }
                    }
                }
            }
            let Some((h_idx, f_idx, _)) = best else { break; };
            let player = &mut game_state.players[ai_idx];
            let h_e = player.hand[h_idx];
            let f_e = player.face_up_cards[f_idx];
            player.hand[h_idx] = f_e;
            player.face_up_cards[f_idx] = h_e;
        }
        swap_state.ai_done[slot] = true;
    }
}

pub(crate) fn advance_swap_phase(
    mut game_state: ResMut<GameState>,
    mut swap_state: ResMut<SwapState>,
) {
    if game_state.phase != GamePhase::Swap {
        return;
    }
    let all_ai_done = swap_state.ai_done.iter().all(|&d| d);
    if swap_state.human_done && all_ai_done {
        game_state.phase = GamePhase::Drafting;
        game_state.selected_cards.clear();
        *swap_state = SwapState::default();
    }
}

pub(crate) fn update_swap_button_visibility(
    game_state: Res<GameState>,
    mut play_q: Query<&mut Style, (With<crate::ui::play_button::PlayButton>, Without<DoneSwapButton>)>,
    mut swap_q: Query<&mut Style, (With<DoneSwapButton>, Without<crate::ui::play_button::PlayButton>)>,
) {
    let in_swap = game_state.phase == GamePhase::Swap;
    for mut style in play_q.iter_mut() {
        style.display = if in_swap { Display::None } else { Display::Flex };
    }
    for mut style in swap_q.iter_mut() {
        style.display = if in_swap { Display::Flex } else { Display::None };
    }
}
