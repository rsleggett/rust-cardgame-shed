//! Integration tests for `systems::play::play_selection` — the central
//! "commit a play" function. Covers the rank-effect branches (2 resets, 7
//! caps, 3 transparent, normal sets effective_rank), the burn paths (Ten,
//! 4-of-a-kind, Hot Hand 3-of-a-kind, Wild Twos, Wild Kings), and the
//! invalid-attempt → pickup-pending branch.

mod common;

use shed::components::card::{Rank, Suit};
use shed::components::game::{BuffKind, GameState};

use common::*;

#[test]
fn ten_burns_pile_and_same_player_goes_again() {
    let mut app = test_app();
    enter_playing(&mut app);
    let five = spawn_card(&mut app, Suit::Hearts, Rank::Five);
    let ten = spawn_card(&mut app, Suit::Clubs, Rank::Ten);
    // Give the player a face-down so they're not eliminated by emptying their hand.
    let backup = spawn_card(&mut app, Suit::Diamonds, Rank::Four);
    set_pile(&mut app, vec![five], Some(Rank::Five));
    set_hand(&mut app, 0, vec![ten]);
    set_face_down(&mut app, 0, vec![backup]);

    run_play_selection(&mut app, vec![ten]);

    let gs = app.world().resource::<GameState>();
    assert!(gs.cards_in_play.is_empty(), "Ten should burn the pile");
    assert_eq!(gs.current_player, 0, "Same player goes again after burn");
    assert_eq!(gs.discard_pile.len(), 2, "Burned pile should land in discard");
    assert_eq!(gs.effective_rank, None);
}

#[test]
fn four_of_a_kind_burns_across_multi_play() {
    let mut app = test_app();
    enter_playing(&mut app);
    // Three Fives already on the pile, hand has the fourth.
    let pile_fives = spawn_ranks(&mut app, &[Rank::Two, Rank::Five, Rank::Five, Rank::Five]);
    let hand_five = spawn_card(&mut app, Suit::Spades, Rank::Five);
    let backup = spawn_card(&mut app, Suit::Diamonds, Rank::Four);
    set_pile(&mut app, pile_fives, Some(Rank::Five));
    set_hand(&mut app, 0, vec![hand_five]);
    set_face_down(&mut app, 0, vec![backup]);

    run_play_selection(&mut app, vec![hand_five]);

    let gs = app.world().resource::<GameState>();
    assert!(gs.cards_in_play.is_empty(), "Top 4 same-rank burns");
    assert_eq!(gs.current_player, 0, "Burn → same player goes again");
}

#[test]
fn hot_hand_burns_three_of_a_kind() {
    let mut app = test_app();
    enter_playing(&mut app);
    give_buff(&mut app, 0, BuffKind::HotHand);
    let pile = spawn_ranks(&mut app, &[Rank::Two, Rank::Six, Rank::Six]);
    let hand_six = spawn_card(&mut app, Suit::Spades, Rank::Six);
    let backup = spawn_card(&mut app, Suit::Diamonds, Rank::Four);
    set_pile(&mut app, pile, Some(Rank::Six));
    set_hand(&mut app, 0, vec![hand_six]);
    set_face_down(&mut app, 0, vec![backup]);

    run_play_selection(&mut app, vec![hand_six]);

    let gs = app.world().resource::<GameState>();
    assert!(gs.cards_in_play.is_empty(), "Hot Hand drops burn threshold to 3");
    assert_eq!(gs.current_player, 0, "Burn → same player goes again");
}

#[test]
fn two_resets_pile_and_advances_turn() {
    let mut app = test_app();
    enter_playing(&mut app);
    let nine = spawn_card(&mut app, Suit::Hearts, Rank::Nine);
    let two = spawn_card(&mut app, Suit::Clubs, Rank::Two);
    set_pile(&mut app, vec![nine], Some(Rank::Nine));
    set_hand(&mut app, 0, vec![two]);

    run_play_selection(&mut app, vec![two]);

    let gs = app.world().resource::<GameState>();
    assert!(gs.any_card_playable, "After a 2, anything goes");
    assert_eq!(gs.effective_rank, None);
    assert!(!gs.seven_active);
    assert_eq!(gs.current_player, 1, "Two doesn't burn — turn advances");
    assert_eq!(gs.cards_in_play.len(), 2, "Two sits on pile, no burn");
}

#[test]
fn three_is_transparent() {
    let mut app = test_app();
    enter_playing(&mut app);
    let nine = spawn_card(&mut app, Suit::Hearts, Rank::Nine);
    let three = spawn_card(&mut app, Suit::Clubs, Rank::Three);
    set_pile(&mut app, vec![nine], Some(Rank::Nine));
    set_hand(&mut app, 0, vec![three]);

    run_play_selection(&mut app, vec![three]);

    let gs = app.world().resource::<GameState>();
    assert_eq!(gs.effective_rank, Some(Rank::Nine), "Three preserves rank");
    assert!(!gs.any_card_playable);
    assert!(!gs.seven_active);
    assert_eq!(gs.current_player, 1);
}

#[test]
fn seven_caps_next_player() {
    let mut app = test_app();
    enter_playing(&mut app);
    let five = spawn_card(&mut app, Suit::Hearts, Rank::Five);
    let seven = spawn_card(&mut app, Suit::Clubs, Rank::Seven);
    set_pile(&mut app, vec![five], Some(Rank::Five));
    set_hand(&mut app, 0, vec![seven]);

    run_play_selection(&mut app, vec![seven]);

    let gs = app.world().resource::<GameState>();
    assert!(gs.seven_active, "Seven flips the cap flag on");
    assert_eq!(gs.effective_rank, Some(Rank::Seven));
    assert!(!gs.any_card_playable);
    assert_eq!(gs.current_player, 1);
}

#[test]
fn normal_rank_sets_effective_rank_and_advances() {
    let mut app = test_app();
    enter_playing(&mut app);
    let five = spawn_card(&mut app, Suit::Hearts, Rank::Five);
    let nine = spawn_card(&mut app, Suit::Clubs, Rank::Nine);
    set_pile(&mut app, vec![five], Some(Rank::Five));
    set_hand(&mut app, 0, vec![nine]);

    run_play_selection(&mut app, vec![nine]);

    let gs = app.world().resource::<GameState>();
    assert_eq!(gs.effective_rank, Some(Rank::Nine));
    assert!(!gs.seven_active);
    assert!(!gs.any_card_playable);
    assert_eq!(gs.current_player, 1);
}

#[test]
fn invalid_face_up_play_triggers_pickup_pending_and_keeps_turn() {
    // Face-up endgame: hand and draw pile both empty so the click handler
    // relaxes validation and lets the player stage any face-up. Player picks
    // a Four that bricks against the pile's King. play_selection pushes it
    // onto the pile and sets needs_to_pickup so the next click on the pile
    // sweeps everything (including the Four) back into hand.
    let mut app = test_app();
    enter_playing(&mut app);
    let king = spawn_card(&mut app, Suit::Hearts, Rank::King);
    let four = spawn_card(&mut app, Suit::Clubs, Rank::Four);
    let other_face_up = spawn_card(&mut app, Suit::Spades, Rank::Six);
    let backup = spawn_card(&mut app, Suit::Diamonds, Rank::Two);
    set_pile(&mut app, vec![king], Some(Rank::King));
    set_face_up(&mut app, 0, vec![four, other_face_up]);
    set_face_down(&mut app, 0, vec![backup]);
    // Hand + draw pile already empty by default → genuine face-up phase.

    run_play_selection(&mut app, vec![four]);

    let gs = app.world().resource::<GameState>();
    assert!(gs.needs_to_pickup, "Brick → pickup pending");
    assert_eq!(gs.current_player, 0, "Turn does NOT advance on brick");
    assert_eq!(gs.cards_in_play.len(), 2, "Bricked card joins the pile so pickup includes it");
}

#[test]
fn wild_twos_buff_burns_a_two() {
    let mut app = test_app();
    enter_playing(&mut app);
    give_buff(&mut app, 0, BuffKind::WildTwos);
    let pile = spawn_ranks(&mut app, &[Rank::Nine]);
    let two = spawn_card(&mut app, Suit::Clubs, Rank::Two);
    let backup = spawn_card(&mut app, Suit::Diamonds, Rank::Four);
    set_pile(&mut app, pile, Some(Rank::Nine));
    set_hand(&mut app, 0, vec![two]);
    set_face_down(&mut app, 0, vec![backup]);

    run_play_selection(&mut app, vec![two]);

    let gs = app.world().resource::<GameState>();
    assert!(gs.cards_in_play.is_empty(), "Wild Twos: Two burns the pile");
    assert_eq!(gs.current_player, 0, "Burn → same player goes again");
}

#[test]
fn ai_refills_hand_inline_after_play() {
    // AI seats refill their hand from the draw pile immediately (no animation
    // deferred). Setup: AI seat 1 has 1 card in hand, draw pile has 3, play
    // the card — hand should refill to target (3 by default).
    let mut app = test_app();
    enter_playing(&mut app);
    set_turn(&mut app, 1);
    let king = spawn_card(&mut app, Suit::Hearts, Rank::King);
    set_pile(&mut app, vec![king], Some(Rank::King));

    let ace = spawn_card(&mut app, Suit::Spades, Rank::Ace);
    set_hand(&mut app, 1, vec![ace]);
    let draw = spawn_ranks(&mut app, &[Rank::Four, Rank::Five, Rank::Six]);
    app.world_mut().resource_mut::<GameState>().draw_pile = draw;

    run_play_selection(&mut app, vec![ace]);

    let gs = app.world().resource::<GameState>();
    assert_eq!(gs.players[1].hand.len(), 3, "AI hand refills inline to target 3");
    assert!(!gs.pending_refill, "Only the human triggers pending_refill");
}

#[test]
fn human_play_defers_refill() {
    let mut app = test_app();
    enter_playing(&mut app);
    let king = spawn_card(&mut app, Suit::Hearts, Rank::King);
    set_pile(&mut app, vec![king], Some(Rank::King));

    let ace = spawn_card(&mut app, Suit::Spades, Rank::Ace);
    set_hand(&mut app, 0, vec![ace]);
    let draw = spawn_ranks(&mut app, &[Rank::Four, Rank::Five, Rank::Six]);
    app.world_mut().resource_mut::<GameState>().draw_pile = draw;

    run_play_selection(&mut app, vec![ace]);

    let gs = app.world().resource::<GameState>();
    assert!(gs.pending_refill, "Human refill is deferred for animation");
    assert!(gs.refill_timer > 0.0);
    assert_eq!(gs.players[0].hand.len(), 0, "Hand not refilled yet — that's draw_refill_system's job");
}
