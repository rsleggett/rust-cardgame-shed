//! World-space text above the play pile: "Play X or higher" / "Play 7 or lower"
//! / "Play anything". Turns orange briefly when an invalid play is attempted.

use bevy::prelude::*;

use crate::components::game::{GamePhase, GameState};
use crate::systems::input::InvalidFeedbackTimer;

#[derive(Component)]
pub(crate) struct PileStatusText;

pub(crate) fn update_pile_status_text(
    game_state: Res<GameState>,
    time: Res<Time>,
    mut feedback: ResMut<InvalidFeedbackTimer>,
    mut text_q: Query<&mut Text, With<PileStatusText>>,
) {
    let Ok(mut text) = text_q.get_single_mut() else { return; };

    if feedback.0 > 0.0 {
        feedback.0 = (feedback.0 - time.delta_seconds()).max(0.0);
    }

    let msg = if game_state.phase != GamePhase::Playing {
        String::new()
    } else if game_state.cards_in_play.is_empty() {
        "Play anything".to_string()
    } else if game_state.any_card_playable {
        "Play anything".to_string()
    } else if game_state.seven_active {
        "Play 7 or lower".to_string()
    } else if let Some(rank) = game_state.effective_rank {
        format!("Play {} or higher", rank)
    } else {
        "Play anything".to_string()
    };

    text.sections[0].value = msg;
    text.sections[0].style.color = if feedback.0 > 0.0 {
        Color::srgb(1.0, 0.5, 0.0) // orange highlight on invalid play
    } else {
        Color::WHITE
    };
}
