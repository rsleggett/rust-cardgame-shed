//! Core play loop: validating plays, sending cards to the pile, refilling
//! the hand, burning the pile, and routing invalid attempts to pickup.

use bevy::prelude::*;

use crate::components::card::{Card, Rank};
use crate::components::game::{BuffKind, GamePhase, GameState, Player};
use crate::rendering::card_constants::{CARD_HEIGHT, PLAY_PILE_X, Z_INDEX_STEP};
use crate::rendering::card_renderer::{CardAnimation, Layout};
use crate::rules::{can_play_card, is_burn};

pub fn has_valid_play(
    game_state: &GameState,
    cards: &Query<&Card>,
    player_index: usize,
) -> bool {
    let sa = game_state.seven_active;
    let acp = game_state.any_card_playable;
    let effective_rank = game_state.effective_rank;
    let draw_pile_not_empty = !game_state.draw_pile.is_empty();
    let has_counter7 = game_state
        .players
        .get(player_index)
        .map(|p| p.has_buff(BuffKind::Counter7))
        .unwrap_or(false);

    if let Some(player) = game_state.players.get(player_index) {
        let source: &[Entity] = if draw_pile_not_empty || !player.hand.is_empty() {
            &player.hand
        } else if !player.face_up_cards.is_empty() {
            &player.face_up_cards
        } else {
            // Face-down phase: player must blind-flip; can't preempt with pickup.
            return !player.face_down_cards.is_empty();
        };
        for &card_entity in source {
            if let Ok(card) = cards.get(card_entity) {
                if can_play_card(card, effective_rank, sa, acp, has_counter7) {
                    return true;
                }
            }
        }
    }
    false
}

/// Per-player hand size. Big Hand drafted? You refill to 4.
pub(crate) fn target_hand_size(player: &Player) -> usize {
    if player.has_buff(BuffKind::BigHand) { 4 } else { 3 }
}

pub fn pickup_cards_in_play(game_state: &mut GameState, player_index: usize) {
    let half_pickup = game_state
        .players
        .get(player_index)
        .map(|p| p.has_buff(BuffKind::HalfPickup))
        .unwrap_or(false);
    let pile_len = game_state.cards_in_play.len();
    let to_hand = if half_pickup {
        // Keep the most recent half (rounded up). Oldest cards are discarded.
        pile_len.div_ceil(2)
    } else {
        pile_len
    };
    let to_discard = pile_len - to_hand;

    let mut drained = std::mem::take(&mut game_state.cards_in_play).into_iter();
    for _ in 0..to_discard {
        if let Some(e) = drained.next() {
            game_state.discard_pile.push(e);
        }
    }
    if let Some(player) = game_state.players.get_mut(player_index) {
        for e in drained {
            player.hand.push(e);
        }
        if half_pickup {
            info!(
                "Player {} picked up {} (Half Pickup: {} discarded)",
                player_index, to_hand, to_discard
            );
        } else {
            info!("Player {} picked up {} cards", player_index, to_hand);
        }
    }
    game_state.current_card = None;
    game_state.effective_rank = None;
    game_state.seven_active = false;
    game_state.any_card_playable = false;
    game_state.selected_cards.clear();
}

/// Plays all selected cards at once. Handles 4-of-a-kind burn and all rank effects.
///
/// May be invoked with a card that turns out to be illegal in two cases:
/// blind face-down flips, and face-up endgame plays where the click handler
/// relaxes validation. Both route an invalid attempt to pickup rather than
/// flashing red — the cards still ride the animation onto the pile so they
/// travel back to the player's hand with the rest of the stack.
pub fn play_selection(
    commands: &mut Commands,
    game_state: &mut GameState,
    cards: &Query<&Card>,
    transforms: &Query<&GlobalTransform>,
    selection: &[Entity],
) {
    if selection.is_empty() { return; }

    let rank = match cards.get(selection[0]) {
        Ok(c) => c.rank,
        Err(_) => return,
    };
    let playing_player = game_state.current_player;

    // Push all to play pile with animations
    for &entity in selection {
        game_state.cards_in_play.push(entity);
        game_state.current_card = Some(entity);
        let target_z = 500.0 + game_state.cards_in_play.len() as f32 * Z_INDEX_STEP;
        let start_pos = transforms.get(entity).map(|t| t.translation()).unwrap_or(Vec3::ZERO);
        commands.entity(entity).insert(CardAnimation {
            target_position: Vec3::new(PLAY_PILE_X, 0.0, target_z),
            start_position: start_pos,
            progress: 0.0,
            speed: 3.0,
        });
        // Remove from player's collection
        if let Some(player) = game_state.players.get_mut(playing_player) {
            if let Some(pos) = player.hand.iter().position(|&e| e == entity) {
                player.hand.remove(pos);
            } else if let Some(pos) = player.face_up_cards.iter().position(|&e| e == entity) {
                player.face_up_cards.remove(pos);
            } else if let Some(pos) = player.face_down_cards.iter().position(|&e| e == entity) {
                player.face_down_cards.remove(pos);
            }
        }
    }

    // Validate against the pile state captured before this play. If the play
    // is illegal (blind face-down brick, or staged face-up that bricks), the
    // cards remain on the pile and the player picks the stack up. Skip refill,
    // burn, rank effects, and turn advance — the pickup flow takes over.
    let has_counter7 = game_state.players[playing_player].has_buff(BuffKind::Counter7);
    let first_card_valid = cards.get(selection[0]).map(|c| {
        can_play_card(
            c,
            game_state.effective_rank,
            game_state.seven_active,
            game_state.any_card_playable,
            has_counter7,
        )
    }).unwrap_or(false);

    if !first_card_valid {
        game_state.needs_to_pickup = true;
        info!("Player {} bricked the play — pickup pending", playing_player);
        return;
    }

    // Human refill is deferred so the new card animates in after the played
    // card lands; AI refills inline (no animation needed for an unseen hand).
    let refill_target = target_hand_size(&game_state.players[playing_player]);
    if playing_player == 0 {
        game_state.pending_refill = true;
        game_state.refill_timer = 0.45;
    } else {
        while game_state.players[playing_player].hand.len() < refill_target
            && !game_state.draw_pile.is_empty()
        {
            if let Some(new_card) = game_state.draw_pile.pop() {
                game_state.players[playing_player].hand.push(new_card);
            }
        }
    }

    // Burn check delegates to rules::is_burn for the predicate; we just need
    // to resolve the per-seat buff flags and the top ≤4 pile ranks here.
    // Same-rank streak burns cap at 4 (or 3 with Hot Hand), so we never need
    // to look further down. If any top entity fails to resolve as a Card the
    // burn is suppressed (defensive — should never happen in normal play).
    let hot_hand = game_state.players[playing_player].has_buff(BuffKind::HotHand);
    let wild_twos = game_state.players[playing_player].has_buff(BuffKind::WildTwos);
    let wild_kings = game_state.players[playing_player].has_buff(BuffKind::WildKings);
    let pile_len = game_state.cards_in_play.len();
    let take = pile_len.min(4);
    let top_ranks: Option<Vec<Rank>> = game_state.cards_in_play[pile_len - take..]
        .iter()
        .map(|&e| cards.get(e).ok().map(|c| c.rank))
        .collect();
    let burn = top_ranks
        .map(|ranks| is_burn(rank, &ranks, hot_hand, wild_twos, wild_kings))
        .unwrap_or(false);

    if burn {
        game_state.seven_active = false;
        game_state.any_card_playable = false;
        game_state.effective_rank = None;
        let cards = std::mem::take(&mut game_state.cards_in_play);
        game_state.discard_pile.extend(cards);
        game_state.current_card = None;
        info!("{:?} burned the pile (4-of-a-kind or 10), player {} goes again", rank, playing_player);
        if game_state.check_and_eliminate(playing_player) {
            info!("Player {} finished {}", playing_player, game_state.finish_order.len());
            game_state.advance_to_next_active();
        }
        return;
    }

    match rank {
        Rank::Three => { /* transparent — effective_rank and flags unchanged */ }
        Rank::Two => {
            game_state.seven_active = false;
            game_state.any_card_playable = true;
            game_state.effective_rank = None;
        }
        Rank::Seven => {
            game_state.seven_active = true;
            game_state.any_card_playable = false;
            game_state.effective_rank = Some(Rank::Seven);
        }
        _ => {
            game_state.seven_active = false;
            game_state.any_card_playable = false;
            game_state.effective_rank = Some(rank);
        }
    }

    if game_state.check_and_eliminate(playing_player) {
        info!("Player {} finished {}", playing_player, game_state.finish_order.len());
    }
    game_state.advance_to_next_active();
}

pub fn check_valid_plays_system(
    mut game_state: ResMut<GameState>,
    cards: Query<&Card>,
) {
    if game_state.phase != GamePhase::Playing || game_state.current_card.is_none() {
        return;
    }
    let current_player_index = game_state.current_player;
    if !game_state.needs_to_pickup && !has_valid_play(&game_state, &cards, current_player_index) {
        game_state.needs_to_pickup = true;
        info!("Player {} needs to pick up cards", current_player_index);
    }
}

pub(crate) fn handle_card_pickup_system(
    mut game_state: ResMut<GameState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if !game_state.needs_to_pickup || game_state.current_player != 0 {
        return;
    }
    if keyboard.just_pressed(KeyCode::Space) {
        let current_player_index = game_state.current_player;
        pickup_cards_in_play(&mut game_state, current_player_index); // also clears selected_cards
        game_state.needs_to_pickup = false;
        game_state.advance_to_next_active();
        info!("Player picked up cards");
    }
}

/// Draws replacement cards into the human's hand with animation after play completes.
pub(crate) fn draw_refill_system(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
    layout: Res<Layout>,
) {
    if !game_state.pending_refill || game_state.phase != GamePhase::Playing { return; }

    // Count down the delay
    if game_state.refill_timer > 0.0 {
        game_state.refill_timer = (game_state.refill_timer - time.delta_seconds()).max(0.0);
        return;
    }

    // Approximate target at the human hand's bottom-edge anchor in design space
    // (layout_cards snaps to the exact fan position once the animation finishes).
    let hand_base_y = -layout.design_height / 2.0 + CARD_HEIGHT / 2.0;
    let refill_target = target_hand_size(&game_state.players[0]);

    while game_state.players[0].hand.len() < refill_target && !game_state.draw_pile.is_empty() {
        let new_card = game_state.draw_pile.pop().unwrap();
        let hand_idx = game_state.players[0].hand.len();
        game_state.players[0].hand.push(new_card);

        // Approximate target at the hand fan centre — layout_cards snaps to exact pos
        // once the animation finishes (1-frame, imperceptible).
        let target_z = 200.0 + hand_idx as f32 * Z_INDEX_STEP;
        commands.entity(new_card).insert(CardAnimation {
            target_position: Vec3::new(0.0, hand_base_y, target_z),
            start_position: Vec3::new(0.0, 0.0, 390.0),
            progress: 0.0,
            speed: 2.5,
        });
    }

    game_state.pending_refill = false;
}
