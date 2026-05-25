//! Consumable buff triggers: Mulligan (M) swaps hand ↔ face-ups,
//! Peek (P) reveals the human's face-downs for a few seconds.

use bevy::prelude::*;

use crate::components::game::{BuffKind, GamePhase, GameState};

/// Counts down while the human's face-down cards (and the top draw card) are
/// shown face-up because the human triggered Peek.
#[derive(Resource, Default)]
pub(crate) struct PeekRevealTimer(pub(crate) f32);

pub(crate) fn handle_mulligan_key(
    mut game_state: ResMut<GameState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if game_state.phase != GamePhase::Playing {
        return;
    }
    if game_state.current_player != 0 {
        return;
    }
    if !keyboard.just_pressed(KeyCode::KeyM) {
        return;
    }
    let player = &mut game_state.players[0];
    if !player.try_consume(BuffKind::Mulligan) {
        return;
    }
    std::mem::swap(&mut player.hand, &mut player.face_up_cards);
    info!("Mulligan used: hand <-> face-up swapped");
}

pub(crate) fn handle_peek_key(
    mut peek_timer: ResMut<PeekRevealTimer>,
    mut game_state: ResMut<GameState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if game_state.phase != GamePhase::Playing {
        return;
    }
    if !keyboard.just_pressed(KeyCode::KeyP) {
        return;
    }
    let player = &mut game_state.players[0];
    if !player.try_consume(BuffKind::Peek) {
        return;
    }
    peek_timer.0 = 3.0;
    info!("Peek used: revealing face-down cards for 3s");
}

pub(crate) fn tick_peek_timer(mut peek_timer: ResMut<PeekRevealTimer>, time: Res<Time>) {
    if peek_timer.0 > 0.0 {
        peek_timer.0 = (peek_timer.0 - time.delta_seconds()).max(0.0);
    }
}
