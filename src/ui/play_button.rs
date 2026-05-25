//! "Play Cards" button — bottom-centre, active only when the human has cards
//! staged. Click confirms the staged multi-card play.

use bevy::prelude::*;

use crate::components::card::Card;
use crate::components::game::{GamePhase, GameState};
use crate::systems::play::play_selection;

#[derive(Component)]
pub(crate) struct PlayButton;

pub(crate) fn handle_play_button(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    cards: Query<&Card>,
    transforms: Query<&GlobalTransform>,
    interaction_q: Query<&Interaction, (Changed<Interaction>, With<PlayButton>)>,
) {
    for interaction in &interaction_q {
        if *interaction == Interaction::Pressed
            && game_state.phase == GamePhase::Playing
            && game_state.current_player == 0
            && !game_state.selected_cards.is_empty()
        {
            let selection = std::mem::take(&mut game_state.selected_cards);
            play_selection(&mut commands, &mut game_state, &cards, &transforms, &selection);
        }
    }
}

pub(crate) fn update_play_button_style(
    game_state: Res<GameState>,
    mut button_q: Query<&mut BackgroundColor, With<PlayButton>>,
) {
    let active = game_state.phase == GamePhase::Playing
        && game_state.current_player == 0
        && !game_state.selected_cards.is_empty();
    for mut bg in button_q.iter_mut() {
        *bg = if active {
            Color::srgb(0.15, 0.55, 0.15).into()
        } else {
            Color::srgb(0.25, 0.25, 0.25).into()
        };
    }
}
