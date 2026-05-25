//! Integration tests for phase-transition systems: `advance_swap_phase`
//! (Swap → Drafting) and `check_valid_plays_system` (sets `needs_to_pickup`
//! when the current player has no legal play).

mod common;

use bevy::ecs::system::RunSystemOnce;

use shed::components::card::{Rank, Suit};
use shed::components::game::{GamePhase, GameState};
use shed::systems::play::check_valid_plays_system;
use shed::systems::swap::{advance_swap_phase, SwapState};

use common::*;

// ── advance_swap_phase ───────────────────────────────────────────────────────

#[test]
fn swap_phase_advances_when_human_and_ai_both_done() {
    let mut app = test_app();
    let staged = spawn_card(&mut app, Suit::Hearts, Rank::Four);
    app.world_mut().resource_mut::<GameState>().phase = GamePhase::Swap;
    app.insert_resource(SwapState {
        human_done: true,
        ai_done: [true; 3],
        human_selected_hand: Some(staged),
    });

    app.world_mut().run_system_once(advance_swap_phase);

    let gs = app.world().resource::<GameState>();
    assert_eq!(gs.phase, GamePhase::Drafting);
    // SwapState is fully reset on exit — every field should be back at default.
    let swap = app.world().resource::<SwapState>();
    assert!(!swap.human_done);
    assert!(swap.ai_done.iter().all(|&d| !d));
    assert!(swap.human_selected_hand.is_none(), "Staged hand card cleared on exit");
}

#[test]
fn swap_phase_waits_for_human() {
    let mut app = test_app();
    app.world_mut().resource_mut::<GameState>().phase = GamePhase::Swap;
    app.insert_resource(SwapState {
        human_done: false, // not yet
        ai_done: [true; 3],
        ..Default::default()
    });

    app.world_mut().run_system_once(advance_swap_phase);

    let gs = app.world().resource::<GameState>();
    assert_eq!(gs.phase, GamePhase::Swap, "Should still be in Swap");
}

#[test]
fn swap_phase_waits_for_ai() {
    let mut app = test_app();
    app.world_mut().resource_mut::<GameState>().phase = GamePhase::Swap;
    app.insert_resource(SwapState {
        human_done: true,
        ai_done: [true, false, true], // seat 2 not done
        ..Default::default()
    });

    app.world_mut().run_system_once(advance_swap_phase);

    let gs = app.world().resource::<GameState>();
    assert_eq!(gs.phase, GamePhase::Swap, "Should still be in Swap");
}

// ── check_valid_plays_system ─────────────────────────────────────────────────

#[test]
fn flags_pickup_when_hand_has_no_valid_play() {
    // Pile is a King, hand is just a Four (with draw pile not empty so we're
    // in hand phase, but no card in hand can beat King). check_valid_plays
    // should set needs_to_pickup.
    let mut app = test_app();
    enter_playing(&mut app);
    let king = spawn_card(&mut app, Suit::Hearts, Rank::King);
    let four = spawn_card(&mut app, Suit::Clubs, Rank::Four);
    let draw_filler = spawn_card(&mut app, Suit::Diamonds, Rank::Six);
    set_pile(&mut app, vec![king], Some(Rank::King));
    set_hand(&mut app, 0, vec![four]);
    app.world_mut().resource_mut::<GameState>().draw_pile = vec![draw_filler];

    app.world_mut().run_system_once(check_valid_plays_system);

    let gs = app.world().resource::<GameState>();
    assert!(gs.needs_to_pickup, "Should flag pickup when no valid play");
}

#[test]
fn does_not_flag_pickup_when_hand_has_valid_play() {
    let mut app = test_app();
    enter_playing(&mut app);
    let five = spawn_card(&mut app, Suit::Hearts, Rank::Five);
    let king = spawn_card(&mut app, Suit::Clubs, Rank::King); // King beats Five
    let draw_filler = spawn_card(&mut app, Suit::Diamonds, Rank::Six);
    set_pile(&mut app, vec![five], Some(Rank::Five));
    set_hand(&mut app, 0, vec![king]);
    app.world_mut().resource_mut::<GameState>().draw_pile = vec![draw_filler];

    app.world_mut().run_system_once(check_valid_plays_system);

    let gs = app.world().resource::<GameState>();
    assert!(!gs.needs_to_pickup);
}

#[test]
fn special_rank_two_satisfies_validity_check() {
    // Even with the pile demanding a King, a Two in hand is always playable.
    let mut app = test_app();
    enter_playing(&mut app);
    let king = spawn_card(&mut app, Suit::Hearts, Rank::King);
    let two = spawn_card(&mut app, Suit::Clubs, Rank::Two);
    let draw_filler = spawn_card(&mut app, Suit::Diamonds, Rank::Six);
    set_pile(&mut app, vec![king], Some(Rank::King));
    set_hand(&mut app, 0, vec![two]);
    app.world_mut().resource_mut::<GameState>().draw_pile = vec![draw_filler];

    app.world_mut().run_system_once(check_valid_plays_system);

    let gs = app.world().resource::<GameState>();
    assert!(!gs.needs_to_pickup, "Two is always playable");
}

#[test]
fn no_op_outside_playing_phase() {
    let mut app = test_app();
    // Stay in Dealing.
    let king = spawn_card(&mut app, Suit::Hearts, Rank::King);
    set_pile(&mut app, vec![king], Some(Rank::King));

    app.world_mut().run_system_once(check_valid_plays_system);

    let gs = app.world().resource::<GameState>();
    assert!(!gs.needs_to_pickup, "Outside Playing the system shouldn't fire");
}
