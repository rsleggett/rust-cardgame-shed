//! Drives the active AI seat. Strategy (per-personality) lives in `crate::ai`;
//! this module just dispatches and routes the chosen play through
//! `play::play_selection` (same path as human plays).

use std::time::Duration;

use bevy::prelude::*;

use crate::components::card::Card;
use crate::components::game::{BuffKind, GamePhase, GameState};
use crate::rules::can_play_card;
use crate::systems::play::{pickup_cards_in_play, play_selection};

pub(crate) const AI_TICK_NORMAL: f32 = 1.5;
pub(crate) const AI_TICK_SPECTATE: f32 = 0.3;

#[derive(Resource)]
pub(crate) struct AITimer(pub(crate) Timer);

impl AITimer {
    pub(crate) fn new() -> Self {
        Self(Timer::from_seconds(AI_TICK_NORMAL, TimerMode::Repeating))
    }
}

pub(crate) fn ai_player_system(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    cards: Query<&Card>,
    transforms: Query<&GlobalTransform>,
    time: Res<Time>,
    mut ai_timer: ResMut<AITimer>,
) {
    if game_state.phase != GamePhase::Playing || game_state.current_player == 0 {
        return;
    }
    // Keep the AI tick rate aligned with spectate mode: snappy after the human
    // is out, normal otherwise. Re-checked every frame so restarts reset for free.
    let want_secs = if game_state.spectate_mode { AI_TICK_SPECTATE } else { AI_TICK_NORMAL };
    if (ai_timer.0.duration().as_secs_f32() - want_secs).abs() > 0.01 {
        ai_timer.0.set_duration(Duration::from_secs_f32(want_secs));
    }
    if !ai_timer.0.tick(time.delta()).just_finished() {
        return;
    }

    if game_state.needs_to_pickup {
        let idx = game_state.current_player;
        pickup_cards_in_play(&mut game_state, idx);
        game_state.needs_to_pickup = false;
        game_state.advance_to_next_active();
        info!("AI {} picked up cards", idx);
        return;
    }

    let effective_rank = game_state.effective_rank;
    let sa = game_state.seven_active;
    let acp = game_state.any_card_playable;
    let current_idx = game_state.current_player;
    let draw_pile_not_empty = !game_state.draw_pile.is_empty();

    // Pick the active source pile (hand → face_up → face_down) using the same
    // priority human play uses.
    let player = &game_state.players[current_idx];
    let (source, from_face_down): (Vec<Entity>, bool) =
        if draw_pile_not_empty || !player.hand.is_empty() {
            (player.hand.clone(), false)
        } else if !player.face_up_cards.is_empty() {
            (player.face_up_cards.clone(), false)
        } else {
            (player.face_down_cards.clone(), true)
        };

    // Filter to legal plays; personality logic chooses among these. Face-down
    // candidates aren't filtered — the AI flips blind too, and play_selection
    // routes a brick to pickup.
    let has_counter7 = game_state.players[current_idx].has_buff(BuffKind::Counter7);
    let candidates: Vec<Entity> = if from_face_down {
        source
    } else {
        source
            .into_iter()
            .filter(|e| {
                cards
                    .get(*e)
                    .map(|c| can_play_card(c, effective_rank, sa, acp, has_counter7))
                    .unwrap_or(false)
            })
            .collect()
    };

    let personality = game_state.players[current_idx].personality;
    let selection = crate::ai::choose_play(personality, &candidates, &cards, from_face_down);

    if selection.is_empty() {
        if !game_state.needs_to_pickup {
            game_state.needs_to_pickup = true;
            info!("AI {} ({:?}) needs to pick up cards", current_idx, personality);
        }
    } else {
        info!(
            "AI {} ({:?}) playing {} card(s)",
            current_idx,
            personality,
            selection.len()
        );
        play_selection(&mut commands, &mut game_state, &cards, &transforms, &selection);
    }
}
