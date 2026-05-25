//! Integration tests for `systems::swap::ai_swap_system` — the greedy AI swap
//! that promotes hand cards into face-up slots whenever the hand has the higher
//! rank.

mod common;

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use shed::components::card::{Card, Rank};
use shed::components::game::{GamePhase, GameState};
use shed::systems::swap::{ai_swap_system, SwapState};

use common::*;

fn enter_swap(app: &mut App) {
    app.world_mut().resource_mut::<GameState>().phase = GamePhase::Swap;
    app.insert_resource(SwapState::default());
}

fn ranks_of(app: &App, entities: &[Entity]) -> Vec<Rank> {
    entities
        .iter()
        .map(|&e| app.world().get::<Card>(e).unwrap().rank)
        .collect()
}

fn run_ai_swap(app: &mut App) {
    app.world_mut().run_system_once(ai_swap_system);
}

#[test]
fn promotes_higher_hand_card_to_face_up() {
    let mut app = test_app();
    enter_swap(&mut app);

    // AI seat 1: hand has King (high), face-up has Four (low). Should swap.
    let king = spawn_card(&mut app, shed::components::card::Suit::Hearts, Rank::King);
    let four = spawn_card(&mut app, shed::components::card::Suit::Clubs, Rank::Four);
    set_hand(&mut app, 1, vec![king]);
    set_face_up(&mut app, 1, vec![four]);

    run_ai_swap(&mut app);

    let gs = app.world().resource::<GameState>();
    assert_eq!(ranks_of(&app, &gs.players[1].hand), vec![Rank::Four]);
    assert_eq!(ranks_of(&app, &gs.players[1].face_up_cards), vec![Rank::King]);
}

#[test]
fn no_swap_when_face_up_already_higher() {
    let mut app = test_app();
    enter_swap(&mut app);

    // Hand 5, face-up King. Already optimal — no swap.
    let five = spawn_card(&mut app, shed::components::card::Suit::Hearts, Rank::Five);
    let king = spawn_card(&mut app, shed::components::card::Suit::Clubs, Rank::King);
    set_hand(&mut app, 1, vec![five]);
    set_face_up(&mut app, 1, vec![king]);

    run_ai_swap(&mut app);

    let gs = app.world().resource::<GameState>();
    assert_eq!(ranks_of(&app, &gs.players[1].hand), vec![Rank::Five]);
    assert_eq!(ranks_of(&app, &gs.players[1].face_up_cards), vec![Rank::King]);
}

#[test]
fn picks_biggest_gain_first() {
    let mut app = test_app();
    enter_swap(&mut app);

    // Hand: [Ace, Six]. Face-up: [Three, Five].
    // Best swap is Ace ↔ Three (gain 11). Then Six ↔ Five (gain 1).
    // After: hand = [Three, Five], face-up = [Ace, Six].
    let ace = spawn_card(&mut app, shed::components::card::Suit::Hearts, Rank::Ace);
    let six = spawn_card(&mut app, shed::components::card::Suit::Hearts, Rank::Six);
    let three = spawn_card(&mut app, shed::components::card::Suit::Clubs, Rank::Three);
    let five = spawn_card(&mut app, shed::components::card::Suit::Clubs, Rank::Five);
    set_hand(&mut app, 1, vec![ace, six]);
    set_face_up(&mut app, 1, vec![three, five]);

    run_ai_swap(&mut app);

    let gs = app.world().resource::<GameState>();
    let hand_ranks = ranks_of(&app, &gs.players[1].hand);
    let face_ranks = ranks_of(&app, &gs.players[1].face_up_cards);
    assert!(hand_ranks.contains(&Rank::Three), "Hand should hold the low Three");
    assert!(hand_ranks.contains(&Rank::Five), "Hand should hold the low Five");
    assert!(face_ranks.contains(&Rank::Ace), "Face-up should have promoted Ace");
    assert!(face_ranks.contains(&Rank::Six), "Face-up should have promoted Six");
}

#[test]
fn marks_ai_done_after_running() {
    let mut app = test_app();
    enter_swap(&mut app);

    // Empty hands and face-ups — nothing to do but mark done.
    run_ai_swap(&mut app);

    let swap = app.world().resource::<SwapState>();
    assert!(swap.ai_done.iter().all(|&d| d), "All AI seats should be marked done");
}

#[test]
fn no_op_outside_swap_phase() {
    let mut app = test_app();
    // Stay in Dealing (the default); insert SwapState anyway.
    app.insert_resource(SwapState::default());

    let king = spawn_card(&mut app, shed::components::card::Suit::Hearts, Rank::King);
    let four = spawn_card(&mut app, shed::components::card::Suit::Clubs, Rank::Four);
    set_hand(&mut app, 1, vec![king]);
    set_face_up(&mut app, 1, vec![four]);

    run_ai_swap(&mut app);

    let gs = app.world().resource::<GameState>();
    // King still in hand — no swap because we're not in Swap phase.
    assert_eq!(ranks_of(&app, &gs.players[1].hand), vec![Rank::King]);
    let swap = app.world().resource::<SwapState>();
    assert!(swap.ai_done.iter().all(|&d| !d), "ai_done flags untouched outside Swap");
}
