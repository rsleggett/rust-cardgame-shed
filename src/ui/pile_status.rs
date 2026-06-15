//! World-space text above the play pile: "Play X or higher" / "Play 7 or lower"
//! / "Play anything". Turns orange briefly when an invalid play is attempted.

use bevy::prelude::*;

use crate::components::game::{GamePhase, GameState};
use crate::rendering::card_constants::CARD_HEIGHT;
use crate::rendering::card_renderer::Layout;
use crate::systems::input::InvalidFeedbackTimer;
use crate::theme;

#[derive(Component)]
pub(crate) struct PileStatusText;

pub(crate) fn update_pile_status_text(
    game_state: Res<GameState>,
    time: Res<Time>,
    layout: Res<Layout>,
    mut feedback: ResMut<InvalidFeedbackTimer>,
    mut text_q: Query<(&mut Text, &mut Transform), With<PileStatusText>>,
) {
    let Ok((mut text, mut transform)) = text_q.get_single_mut() else { return; };

    // Position/scale only change on an orientation flip — write them only then,
    // so the text entity isn't dirtied (and re-laid-out) every frame.
    if layout.is_changed() {
        let pile_scale = layout.pile_scale();
        transform.translation.x = layout.play_pile_x();
        transform.translation.y = (CARD_HEIGHT / 2.0 + 24.0) * pile_scale;
        // Grow the prompt with the (larger) portrait pile so it reads on a phone.
        transform.scale = Vec3::splat(pile_scale);
    }

    if feedback.0 > 0.0 {
        feedback.0 = (feedback.0 - time.delta_seconds()).max(0.0);
    }

    // Only show the prompt on the human's own turn — during AI turns it's just
    // noise (and the AI doesn't need telling what to play).
    let msg = if game_state.phase != GamePhase::Playing || game_state.current_player != 0 {
        String::new()
    } else {
        let prompt = if game_state.cards_in_play.is_empty() || game_state.any_card_playable {
            "PLAY ANYTHING".to_string()
        } else if game_state.seven_active {
            "PLAY 7 OR LOWER".to_string()
        } else if let Some(rank) = game_state.effective_rank {
            format!("PLAY {} OR HIGHER", rank)
        } else {
            "PLAY ANYTHING".to_string()
        };
        format!(">> YOUR TURN - {}", prompt)
    };

    // Only write when changed — a Text mutation forces a glyph re-layout, so
    // skipping unchanged frames is the win on mobile.
    if text.sections[0].value != msg {
        text.sections[0].value = msg;
    }
    let new_color = if feedback.0 > 0.0 {
        Color::srgb(1.0, 0.5, 0.0) // orange highlight on invalid play
    } else {
        theme::GOLD
    };
    if text.sections[0].style.color != new_color {
        text.sections[0].style.color = new_color;
    }
}
