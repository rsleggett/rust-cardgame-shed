//! "Play Cards" button — bottom-centre, active only when the human has cards
//! staged. Click confirms the staged multi-card play.

use bevy::prelude::*;

use crate::components::card::Card;
use crate::components::game::{GamePhase, GameState};
use crate::systems::play::play_selection;
use crate::theme;

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
    mut button_q: Query<(&mut BackgroundColor, &mut BorderColor), With<PlayButton>>,
) {
    let active = game_state.phase == GamePhase::Playing
        && game_state.current_player == 0
        && !game_state.selected_cards.is_empty();
    let fill = if active { theme::LIME } else { theme::LOCKED_GREY };
    for (mut bg, mut border) in button_q.iter_mut() {
        *bg = fill.into();
        *border = theme::chunky_shadow(fill).into();
    }
}

/// Arcade "depress" juice shared by every chunky button: while held, the button
/// slides down to meet its drop-shadow (bottom border collapses, position drops
/// by the shadow height); on release it pops back up. Marker-agnostic — runs on
/// any `Button`, since all chunky buttons share the 12px offset / 5px edge.
#[allow(clippy::type_complexity)]
pub(crate) fn depress_buttons(
    mut button_q: Query<(&Interaction, &mut Style), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, mut style) in button_q.iter_mut() {
        if *interaction == Interaction::Pressed {
            style.bottom = Val::Px(7.0);
            style.border = UiRect::bottom(Val::Px(0.0));
        } else {
            style.bottom = Val::Px(12.0);
            style.border = UiRect::bottom(Val::Px(5.0));
        }
    }
}
