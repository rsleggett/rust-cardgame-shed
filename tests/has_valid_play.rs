//! Integration tests for `systems::play::has_valid_play` — the source-priority
//! check used by `check_valid_plays_system` to decide when to flag pickup.

mod common;

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use shed::components::card::{Card, Rank, Suit};
use shed::components::game::GameState;
use shed::systems::play::has_valid_play;

use common::*;

/// Wraps `has_valid_play` in a one-shot system so the test can drive it
/// without manually constructing a Query.
fn check_via_system(app: &mut App, player_index: usize) -> bool {
    app.world_mut().run_system_once_with(
        player_index,
        |In(idx): In<usize>, gs: Res<GameState>, cards: Query<&Card>| -> bool {
            has_valid_play(&gs, &cards, idx)
        },
    )
}

#[test]
fn hand_source_with_valid_card_returns_true() {
    let mut app = test_app();
    enter_playing(&mut app);
    let five = spawn_card(&mut app, Suit::Hearts, Rank::Five);
    let queen = spawn_card(&mut app, Suit::Clubs, Rank::Queen);
    let draw_filler = spawn_card(&mut app, Suit::Diamonds, Rank::Six);
    set_pile(&mut app, vec![five], Some(Rank::Five));
    set_hand(&mut app, 0, vec![queen]);
    app.world_mut().resource_mut::<GameState>().draw_pile = vec![draw_filler];

    assert!(check_via_system(&mut app, 0));
}

#[test]
fn hand_source_with_no_valid_card_returns_false() {
    let mut app = test_app();
    enter_playing(&mut app);
    let king = spawn_card(&mut app, Suit::Hearts, Rank::King);
    let four = spawn_card(&mut app, Suit::Clubs, Rank::Four);
    let draw_filler = spawn_card(&mut app, Suit::Diamonds, Rank::Six);
    set_pile(&mut app, vec![king], Some(Rank::King));
    set_hand(&mut app, 0, vec![four]);
    app.world_mut().resource_mut::<GameState>().draw_pile = vec![draw_filler];

    assert!(!check_via_system(&mut app, 0));
}

#[test]
fn face_up_source_when_hand_and_draw_empty() {
    // Draw pile empty + hand empty → face-up phase. Face-up has a Queen,
    // pile demands a Five.
    let mut app = test_app();
    enter_playing(&mut app);
    let five = spawn_card(&mut app, Suit::Hearts, Rank::Five);
    let queen = spawn_card(&mut app, Suit::Clubs, Rank::Queen);
    set_pile(&mut app, vec![five], Some(Rank::Five));
    set_face_up(&mut app, 0, vec![queen]);

    assert!(check_via_system(&mut app, 0));
}

#[test]
fn face_down_phase_always_returns_true_if_any_remain() {
    // Critical: face-down play is BLIND. Even if every face-down card would
    // technically be illegal vs the pile, has_valid_play returns true so the
    // player gets a flip rather than an info-leaking auto-pickup.
    let mut app = test_app();
    enter_playing(&mut app);
    let king = spawn_card(&mut app, Suit::Hearts, Rank::King);
    let unplayable = spawn_card(&mut app, Suit::Clubs, Rank::Four);
    set_pile(&mut app, vec![king], Some(Rank::King));
    set_face_down(&mut app, 0, vec![unplayable]);

    assert!(check_via_system(&mut app, 0), "Face-down phase must always allow the flip");
}

#[test]
fn face_down_phase_returns_false_when_no_cards_left() {
    // Edge case: no hand, no face-up, no face-down → genuinely nothing to do.
    let mut app = test_app();
    enter_playing(&mut app);
    let king = spawn_card(&mut app, Suit::Hearts, Rank::King);
    set_pile(&mut app, vec![king], Some(Rank::King));

    assert!(!check_via_system(&mut app, 0));
}
