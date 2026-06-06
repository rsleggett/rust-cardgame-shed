//! Card dealing tick + initial first-card draw onto the play pile.

use bevy::prelude::*;

use crate::components::card::Card;
use crate::components::game::{GamePhase, GameState};
use crate::rendering::card_constants::PLAY_PILE_X;
use crate::rendering::card_renderer::{CardAnimation, Layout};

/// Seconds between dealing each card during round setup. 36 cards × this value
/// is the total deal time.
pub(crate) const DEAL_INTERVAL: f32 = 0.15;

#[derive(Resource)]
pub(crate) struct DealTimer(pub(crate) Timer);

impl DealTimer {
    pub(crate) fn new() -> Self {
        Self(Timer::from_seconds(DEAL_INTERVAL, TimerMode::Repeating))
    }
}

pub(crate) fn deal_cards_system(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
    mut deal_timer: ResMut<DealTimer>,
    cards: Query<&Card>,
    layout: Res<Layout>,
) {
    if game_state.dealing_in_progress && deal_timer.0.tick(time.delta()).just_finished() {
        game_state.deal_next_card(&mut commands, &cards, &layout);
    }
}

pub(crate) fn draw_first_card_system(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    cards: Query<&Card>,
) {
    if game_state.phase == GamePhase::Playing
        && game_state.current_card.is_none()
        && !game_state.draw_pile.is_empty()
    {
        if let Some(card_entity) = game_state.draw_pile.pop() {
            // Record the effective rank so the first player knows what to beat
            if let Ok(card) = cards.get(card_entity) {
                game_state.effective_rank = Some(card.rank);
            }
            game_state.current_card = Some(card_entity);
            game_state.cards_in_play.push(card_entity);

            commands.entity(card_entity).insert(CardAnimation {
                target_position: Vec3::new(PLAY_PILE_X, 0.0, 500.0),
                start_position: Vec3::new(0.0, 0.0, 400.0),
                progress: 0.0,
                speed: 2.0,
            });

            info!("First card drawn and placed on table");
        }
    }
}
