//! Fixtures + helpers shared across integration tests. Lives in the
//! `tests/common/` subdirectory so cargo skips compiling it as a standalone
//! test binary — it's just an importable module each `tests/*.rs` declares
//! via `mod common;`.

#![allow(dead_code)] // each test file uses a different subset of helpers

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use shed::components::card::{Card, Rank, Suit};
use shed::components::game::{
    ActiveBuff, BuffKind, GamePhase, GameState, MatchState, Personality,
};

/// Build a fresh App seeded with `GameState`, `MatchState`, and four players
/// (human at seat 0, three Rob AIs at seats 1-3). Caller mutates the resources
/// from there to set phase, hands, pile, etc.
pub fn test_app() -> App {
    let mut app = App::new();
    app.insert_resource(GameState::new());
    app.insert_resource(MatchState::new(4, 10));
    {
        let mut gs = app.world_mut().resource_mut::<GameState>();
        gs.add_player("You".to_string(), Personality::Rob, vec![]);
        gs.add_player("AI1".to_string(), Personality::Rob, vec![]);
        gs.add_player("AI2".to_string(), Personality::Rob, vec![]);
        gs.add_player("AI3".to_string(), Personality::Rob, vec![]);
    }
    app
}

/// Spawn a `Card` entity and return its `Entity` handle. Cards are bare —
/// no Transform, no SpriteBundle. The play loop doesn't require visual
/// components (transforms.get fallback to Vec3::ZERO is exercised).
pub fn spawn_card(app: &mut App, suit: Suit, rank: Rank) -> Entity {
    app.world_mut().spawn(Card::new(suit, rank)).id()
}

/// Spawn a list of Hearts cards at the given ranks. Convenience for the
/// common "I want a hand of 5, 6, 7" setup.
pub fn spawn_ranks(app: &mut App, ranks: &[Rank]) -> Vec<Entity> {
    ranks.iter().map(|&r| spawn_card(app, Suit::Hearts, r)).collect()
}

/// Replace player `seat`'s hand with the given entities.
pub fn set_hand(app: &mut App, seat: usize, cards: Vec<Entity>) {
    app.world_mut().resource_mut::<GameState>().players[seat].hand = cards;
}

/// Replace player `seat`'s face-up cards with the given entities.
pub fn set_face_up(app: &mut App, seat: usize, cards: Vec<Entity>) {
    app.world_mut().resource_mut::<GameState>().players[seat].face_up_cards = cards;
}

/// Replace player `seat`'s face-down cards with the given entities.
pub fn set_face_down(app: &mut App, seat: usize, cards: Vec<Entity>) {
    app.world_mut().resource_mut::<GameState>().players[seat].face_down_cards = cards;
}

/// Push entities onto `cards_in_play` (the pile) and set `effective_rank`
/// atomically. Oldest first; the last entry is the visible top card.
///
/// `effective_rank` is required (not derived from the cards) because the rank
/// that the next player must beat isn't always the top card's rank — e.g. a
/// 3 on top is transparent, so `effective_rank` reflects whatever sat below.
/// Pass `None` for tests where the rank doesn't matter (pickup, swap, etc.).
///
/// Does NOT touch `seven_active` or `any_card_playable`; set those separately
/// if a test needs them.
pub fn set_pile(app: &mut App, cards: Vec<Entity>, effective_rank: Option<Rank>) {
    let mut gs = app.world_mut().resource_mut::<GameState>();
    gs.cards_in_play = cards;
    gs.current_card = gs.cards_in_play.last().copied();
    gs.effective_rank = effective_rank;
}

/// Add a buff to player `seat`. Useful for testing buff-conditional paths
/// (Counter-7, Wild Twos, Half Pickup, etc.).
pub fn give_buff(app: &mut App, seat: usize, kind: BuffKind) {
    app.world_mut().resource_mut::<GameState>().players[seat]
        .modifiers
        .push(ActiveBuff { kind, used_this_round: false });
}

/// Force phase to `Playing` (the default for most tests). Adjust separately
/// if a test needs Swap / Drafting / GameOver.
pub fn enter_playing(app: &mut App) {
    app.world_mut().resource_mut::<GameState>().phase = GamePhase::Playing;
}

/// Set `current_player`. Defaults to 0 in a fresh GameState.
pub fn set_turn(app: &mut App, seat: usize) {
    app.world_mut().resource_mut::<GameState>().current_player = seat;
}

/// Run `shed::systems::play::play_selection` as a one-shot system with the
/// given selection. Returns when the system has completed and command buffers
/// have been flushed.
pub fn run_play_selection(app: &mut App, selection: Vec<Entity>) {
    use shed::systems::play::play_selection;

    app.world_mut().run_system_once_with(
        selection,
        |In(selection): In<Vec<Entity>>,
         mut commands: Commands,
         mut game_state: ResMut<GameState>,
         cards: Query<&Card>,
         transforms: Query<&GlobalTransform>| {
            play_selection(&mut commands, &mut game_state, &cards, &transforms, &selection);
        },
    );
}
