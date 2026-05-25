//! Integration tests for `systems::play::pickup_cards_in_play`.

mod common;

use shed::components::card::{Rank, Suit};
use shed::components::game::{BuffKind, GameState};
use shed::systems::play::pickup_cards_in_play;

use common::*;

#[test]
fn full_pickup_moves_pile_to_hand_and_clears_pile_flags() {
    let mut app = test_app();
    enter_playing(&mut app);
    let pile = spawn_ranks(&mut app, &[Rank::Five, Rank::Six, Rank::Seven]);
    set_pile(&mut app, pile);
    {
        let mut gs = app.world_mut().resource_mut::<GameState>();
        gs.effective_rank = Some(Rank::Seven);
        gs.seven_active = true;
        gs.selected_cards = vec![]; // start clean
    }

    pickup_cards_in_play(&mut app.world_mut().resource_mut::<GameState>(), 0);

    let gs = app.world().resource::<GameState>();
    assert!(gs.cards_in_play.is_empty());
    assert_eq!(gs.players[0].hand.len(), 3, "Whole pile goes to hand");
    assert!(gs.discard_pile.is_empty(), "Without Half Pickup, nothing discards");
    assert_eq!(gs.effective_rank, None, "Pickup resets pile state");
    assert!(!gs.seven_active);
    assert!(!gs.any_card_playable);
    assert!(gs.current_card.is_none());
}

#[test]
fn half_pickup_buff_discards_oldest_half() {
    // Half Pickup keeps the most-recent ceil(N/2). With 5 cards on the pile,
    // 3 go to hand (the newest), 2 to discard (the oldest).
    let mut app = test_app();
    enter_playing(&mut app);
    give_buff(&mut app, 0, BuffKind::HalfPickup);
    let pile = spawn_ranks(&mut app, &[
        Rank::Two,   // oldest
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,   // newest
    ]);
    set_pile(&mut app, pile);

    pickup_cards_in_play(&mut app.world_mut().resource_mut::<GameState>(), 0);

    let gs = app.world().resource::<GameState>();
    assert_eq!(gs.players[0].hand.len(), 3, "Half Pickup: ceil(5/2) = 3 to hand");
    assert_eq!(gs.discard_pile.len(), 2, "Other 2 discarded");
}

#[test]
fn half_pickup_with_even_count_splits_evenly() {
    let mut app = test_app();
    enter_playing(&mut app);
    give_buff(&mut app, 0, BuffKind::HalfPickup);
    let pile = spawn_ranks(&mut app, &[Rank::Two, Rank::Three, Rank::Four, Rank::Five]);
    set_pile(&mut app, pile);

    pickup_cards_in_play(&mut app.world_mut().resource_mut::<GameState>(), 0);

    let gs = app.world().resource::<GameState>();
    assert_eq!(gs.players[0].hand.len(), 2);
    assert_eq!(gs.discard_pile.len(), 2);
}

#[test]
fn pickup_on_empty_pile_is_noop() {
    let mut app = test_app();
    enter_playing(&mut app);

    pickup_cards_in_play(&mut app.world_mut().resource_mut::<GameState>(), 0);

    let gs = app.world().resource::<GameState>();
    assert!(gs.cards_in_play.is_empty());
    assert!(gs.players[0].hand.is_empty());
    assert!(gs.discard_pile.is_empty());
}

#[test]
fn pickup_clears_staged_selection() {
    let mut app = test_app();
    enter_playing(&mut app);
    let stage = spawn_card(&mut app, Suit::Hearts, Rank::Four);
    {
        let mut gs = app.world_mut().resource_mut::<GameState>();
        gs.selected_cards = vec![stage];
    }

    pickup_cards_in_play(&mut app.world_mut().resource_mut::<GameState>(), 0);

    let gs = app.world().resource::<GameState>();
    assert!(gs.selected_cards.is_empty(), "Pickup clears any staged play");
}
