use bevy::prelude::*;
use crate::components::card::{Card, Rank, Suit};
use crate::rendering::card_renderer::CardAnimation;
use crate::components::card_visual::spawn_card_complete;

#[derive(Resource)]
pub struct GameState {
    pub current_player: usize,
    pub phase: GamePhase,
    pub players: Vec<Player>,
    pub draw_pile: Vec<Entity>,
    pub discard_pile: Vec<Entity>,
    pub dealing_in_progress: bool,
    pub cards_to_deal: Vec<(Entity, usize, usize, bool)>, // (card_entity, player_index, card_index, is_face_up)
    pub current_card: Option<Entity>, // The top card currently on the table
    pub cards_in_play: Vec<Entity>, // All cards currently in play (including current_card)
    pub needs_to_pickup: bool, // Indicates if the current player needs to pick up cards
    pub seven_active: bool,      // Next player must play ≤ 7
    pub any_card_playable: bool, // True after 2 is played; any card is valid next
    // Player indices in the order they emptied their stacks. Last entry is the "Shed".
    // GameOver fires once this contains every player.
    pub finish_order: Vec<usize>,
    // Set to true the moment the human (index 0) is eliminated. Drives AI tick speedup
    // and suppresses human input so they don't sit through the rest of the round.
    pub spectate_mode: bool,
    pub effective_rank: Option<Rank>, // Rank the next player must beat (None = any)
    pub selected_cards: Vec<Entity>,  // Cards staged for multi-play (human player only)
    pub pending_refill: bool,         // Human needs to draw replacement cards (deferred)
    pub refill_timer: f32,            // Seconds to wait before drawing (lets play anim complete)
}

pub struct Player {
    pub id: usize,
    pub name: String,
    pub face_up_cards: Vec<Entity>,
    pub face_down_cards: Vec<Entity>,
    pub hand: Vec<Entity>,
    // Cached "this player has emptied all three stacks". Mirrors membership in
    // GameState.finish_order — kept on Player for cheap reads in hot loops.
    pub eliminated: bool,
    /// Drives AI decision-making. Unused (but present) for the human seat.
    pub personality: Personality,
    /// Active buffs picked across all rounds of the current match. Reset only
    /// when a new match starts. `used_this_round` flips back to false on each
    /// round restart for consumables.
    pub modifiers: Vec<ActiveBuff>,
}

impl Player {
    pub fn has_buff(&self, kind: BuffKind) -> bool {
        self.modifiers.iter().any(|b| b.kind == kind)
    }
    /// True if the consumable was available and is now consumed.
    pub fn try_consume(&mut self, kind: BuffKind) -> bool {
        if let Some(b) = self.modifiers.iter_mut().find(|b| b.kind == kind) {
            if !b.used_this_round {
                b.used_this_round = true;
                return true;
            }
        }
        false
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum GamePhase {
    Dealing,
    /// Standard Shed pre-play: each player may swap any hand cards with their
    /// face-up cards. Ends when the human presses Done and the AI heuristics
    /// have finished.
    Swap,
    /// Each player picks one buff for the round before play starts.
    Drafting,
    Playing,
    GameOver,
}

/// One drafted modifier. Buffs persist across rounds; only `used_this_round`
/// resets each round (and only matters for consumables).
#[derive(Clone, Debug)]
pub struct ActiveBuff {
    pub kind: BuffKind,
    pub used_this_round: bool,
}

/// All possible draftable perks. Order in `ALL` is the draft pool; new entries
/// should be appended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuffKind {
    WildTwos,
    HotHand,
    Counter7,
    BigHand,
    WildKings,
    HalfPickup,
    Mulligan,
    Peek,
}

impl BuffKind {
    pub const ALL: &'static [BuffKind] = &[
        BuffKind::WildTwos,
        BuffKind::HotHand,
        BuffKind::Counter7,
        BuffKind::BigHand,
        BuffKind::WildKings,
        BuffKind::HalfPickup,
        BuffKind::Mulligan,
        BuffKind::Peek,
    ];

    pub fn is_consumable(self) -> bool {
        matches!(self, BuffKind::Mulligan | BuffKind::Peek)
    }

    pub fn display_name(self) -> &'static str {
        match self {
            BuffKind::WildTwos => "Wild Twos",
            BuffKind::HotHand => "Hot Hand",
            BuffKind::Counter7 => "Counter-7",
            BuffKind::BigHand => "Big Hand",
            BuffKind::WildKings => "Wild Kings",
            BuffKind::HalfPickup => "Half Pickup",
            BuffKind::Mulligan => "Mulligan",
            BuffKind::Peek => "Peek",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            BuffKind::WildTwos => "Your 2s also burn the pile.",
            BuffKind::HotHand => "Your same-rank triples (3+) burn the pile.",
            BuffKind::Counter7 => "You ignore the 'play 7 or lower' restriction.",
            BuffKind::BigHand => "Your hand refills to 4 cards instead of 3.",
            BuffKind::WildKings => "Your Kings also burn the pile.",
            BuffKind::HalfPickup => "When you pick up the pile, half of it is discarded instead.",
            BuffKind::Mulligan => "Once per round (M): swap your hand with your face-up cards.",
            BuffKind::Peek => "Once per round (P): reveal your face-down cards for 3 seconds.",
        }
    }
}

/// One of three AI play styles. Picked per-AI-seat at match start; persists
/// across rounds within the match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Personality {
    /// Gung-ho + smart: bundles same-rank plays, proactive with burns.
    Rob,
    /// Cunning: never bundles, hoards specials.
    Mike,
    /// Chaotic: random rank choice, often wastes specials by bundling them.
    Dave,
}

/// One AI opponent's identity for the duration of a match.
#[derive(Clone, Debug)]
pub struct AiPersona {
    pub personality: Personality,
    /// Already disambiguated when duplicates were drawn (e.g. "Dave 2").
    pub display_name: String,
}

/// Per-match state that survives across rounds. Cleared only when a new match
/// starts (after someone hits `target`).
#[derive(Resource)]
pub struct MatchState {
    pub round: u32,
    pub target: u32,
    pub scores: Vec<u32>,
    /// One persona per AI seat (length = player_count - 1, excluding the human).
    /// Reused across rounds within a match; regenerated on `MatchState::new`.
    pub personas: Vec<AiPersona>,
    /// Player index of the previous round's "Shed" — they play first next round.
    pub previous_shed: Option<usize>,
    /// True once the current/just-finished round has been scored. Reset on
    /// `start_next_round`. Prevents double-awarding from per-frame systems.
    pub current_round_scored: bool,
    /// Set once any player's cumulative score reaches `target`.
    pub match_winner: Option<usize>,
    /// One Vec per seat. Snapshot of `Player.modifiers` between rounds so that
    /// rebuilding `GameState` doesn't wipe accumulated buffs. Empty on new match.
    pub persistent_modifiers: Vec<Vec<ActiveBuff>>,
}

impl MatchState {
    pub fn new(player_count: usize, target: u32) -> Self {
        let ai_count = player_count.saturating_sub(1);
        Self {
            round: 1,
            target,
            scores: vec![0; player_count],
            personas: Self::generate_personas(ai_count),
            previous_shed: None,
            current_round_scored: false,
            match_winner: None,
            persistent_modifiers: vec![Vec::new(); player_count],
        }
    }

    /// Draws `count` personalities at random (with replacement) from the pool
    /// and labels each with a display name. Duplicates get a numeric suffix in
    /// pick order — e.g. drawing Dave, Mike, Dave produces "Dave", "Mike",
    /// "Dave 2". Uses the same `rand::random` source as `prepare_dealing` to
    /// stay dependency-free.
    pub fn generate_personas(count: usize) -> Vec<AiPersona> {
        let pool = [Personality::Rob, Personality::Mike, Personality::Dave];
        let mut counts = [0u32; 3];
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            let pick = pool[(rand::random::<f32>() * pool.len() as f32) as usize % pool.len()];
            let slot = match pick {
                Personality::Rob => 0,
                Personality::Mike => 1,
                Personality::Dave => 2,
            };
            counts[slot] += 1;
            let base = match pick {
                Personality::Rob => "Rob",
                Personality::Mike => "Mike",
                Personality::Dave => "Dave",
            };
            let display_name = if counts[slot] == 1 {
                base.to_string()
            } else {
                format!("{} {}", base, counts[slot])
            };
            out.push(AiPersona {
                personality: pick,
                display_name,
            });
        }
        out
    }

    /// Points for a given finish position. For N players: 1st = N-1, 2nd = N-2,
    /// …, Shed = 0. With 4 players that's the familiar 3/2/1/0.
    pub fn score_for_position(position: usize, total: usize) -> u32 {
        if position + 1 >= total {
            0
        } else {
            (total - 1 - position) as u32
        }
    }

    /// Award points for the just-finished round. Idempotent: no-op if already
    /// scored this round. Returns true if scoring occurred.
    pub fn award_round(&mut self, finish_order: &[usize]) -> bool {
        if self.current_round_scored || finish_order.is_empty() {
            return false;
        }
        let total = finish_order.len();
        for (position, &player_idx) in finish_order.iter().enumerate() {
            if player_idx < self.scores.len() {
                self.scores[player_idx] += Self::score_for_position(position, total);
            }
        }
        self.previous_shed = finish_order.last().copied();
        self.current_round_scored = true;
        if self.match_winner.is_none() {
            // Highest cumulative score among players who have reached target.
            self.match_winner = self
                .scores
                .iter()
                .enumerate()
                .filter(|(_, &s)| s >= self.target)
                .max_by_key(|(_, &s)| s)
                .map(|(i, _)| i);
        }
        true
    }

    pub fn start_next_round(&mut self) {
        self.round += 1;
        self.current_round_scored = false;
    }

    pub fn is_match_over(&self) -> bool {
        self.match_winner.is_some()
    }
}

impl GameState {
    pub fn new() -> Self {
        Self {
            current_player: 0,
            phase: GamePhase::Dealing,
            players: Vec::new(),
            draw_pile: Vec::new(),
            discard_pile: Vec::new(),
            dealing_in_progress: false,
            cards_to_deal: Vec::new(),
            current_card: None,
            cards_in_play: Vec::new(),
            needs_to_pickup: false,
            seven_active: false,
            any_card_playable: false,
            finish_order: Vec::new(),
            spectate_mode: false,
            effective_rank: None,
            selected_cards: Vec::new(),
            pending_refill: false,
            refill_timer: 0.0,
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {

    pub fn add_player(
        &mut self,
        name: String,
        personality: Personality,
        modifiers: Vec<ActiveBuff>,
    ) -> usize {
        let id = self.players.len();
        self.players.push(Player {
            id,
            name,
            face_up_cards: Vec::new(),
            face_down_cards: Vec::new(),
            hand: Vec::new(),
            eliminated: false,
            personality,
            modifiers,
        });
        id
    }

    /// If `player_index` has emptied all three stacks, mark them eliminated and
    /// push to `finish_order`. When only one player remains, that player is the
    /// "Shed" — also pushed to `finish_order` and the game transitions to GameOver.
    /// Returns true if the player was just eliminated (the caller may need to
    /// advance the turn in burn/"go again" paths).
    pub fn check_and_eliminate(&mut self, player_index: usize) -> bool {
        {
            let p = &self.players[player_index];
            if p.eliminated
                || !p.hand.is_empty()
                || !p.face_up_cards.is_empty()
                || !p.face_down_cards.is_empty()
            {
                return false;
            }
        }
        self.players[player_index].eliminated = true;
        self.finish_order.push(player_index);
        if player_index == 0 {
            self.spectate_mode = true;
        }

        if self.finish_order.len() + 1 == self.players.len() {
            if let Some(shed) = (0..self.players.len()).find(|i| !self.players[*i].eliminated) {
                self.finish_order.push(shed);
            }
            self.phase = GamePhase::GameOver;
        }
        true
    }

    /// Advance `current_player` to the next non-eliminated seat. No-op once the
    /// game is over. Safe because GameOver fires before fewer than 2 active
    /// players remain.
    pub fn advance_to_next_active(&mut self) {
        if self.phase == GamePhase::GameOver {
            return;
        }
        let len = self.players.len();
        for _ in 0..len {
            self.current_player = (self.current_player + 1) % len;
            if !self.players[self.current_player].eliminated {
                return;
            }
        }
    }

    pub fn prepare_dealing(&mut self, commands: &mut Commands, font: Handle<Font>, suit_font: Handle<Font>) {
        // Create a standard deck of 52 cards
        let mut deck = Vec::new();
        for suit in [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades] {
            for rank in [
                Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Six,
                Rank::Seven, Rank::Eight, Rank::Nine, Rank::Ten, Rank::Jack,
                Rank::Queen, Rank::King, Rank::Ace,
            ] {
                deck.push(Card::new(suit, rank));
            }
        }

        // Shuffle the deck (simple implementation)
        for i in (1..deck.len()).rev() {
            let j = (i as f32 * rand::random::<f32>()) as usize;
            deck.swap(i, j);
        }

        // Spawn all cards in the center of the screen
        let center_position = Vec3::new(0.0, 0.0, 0.0);
        
        // First, spawn all cards in the center
        for card in deck {
            let card_entity = spawn_card_complete(commands, card, center_position, font.clone(), suit_font.clone());
            self.draw_pile.push(card_entity);
        }
        
        // Prepare the dealing sequence
        self.dealing_in_progress = true;
        self.cards_to_deal.clear();
        
        // For each player, deal their cards in the correct order
        for player_index in 0..self.players.len() {
            // Calculate the starting index for this player's cards
            let player_start = player_index * 9; // 9 cards per player (3 face-down + 3 face-up + 3 hand)
            
            // Deal face-down cards (first 3)
            for i in 0..3 {
                if let Some(&card_entity) = self.draw_pile.get(player_start + i) {
                    self.cards_to_deal.push((card_entity, player_index, i, false));
                }
            }
            
            // Deal face-up cards (next 3)
            for i in 0..3 {
                if let Some(&card_entity) = self.draw_pile.get(player_start + 3 + i) {
                    self.cards_to_deal.push((card_entity, player_index, 3 + i, true));
                }
            }
            
            // Deal hand cards (last 3)
            for i in 0..3 {
                if let Some(&card_entity) = self.draw_pile.get(player_start + 6 + i) {
                    // Hand cards are face-up only for the human player (index 0)
                    let is_face_up = player_index == 0;
                    self.cards_to_deal.push((card_entity, player_index, 6 + i, is_face_up));
                }
            }
        }
        
        // Remove the cards to be dealt from the draw pile
        for (card_entity, _, _, _) in &self.cards_to_deal {
            if let Some(pos) = self.draw_pile.iter().position(|&e| e == *card_entity) {
                self.draw_pile.remove(pos);
            }
        }
    }
    
    pub fn deal_next_card(&mut self, commands: &mut Commands, cards: &Query<&Card>) -> bool {
        if self.cards_to_deal.is_empty() {
            self.dealing_in_progress = false;
            // Standard Shed: swap → drafting → playing. The swap systems hand
            // off to Drafting once both the human and AI heuristics are done.
            self.phase = GamePhase::Swap;
            return false;
        }
        
        let (card_entity, player_index, card_index, is_face_up) = self.cards_to_deal.remove(0);
        
        // Determine which set of cards this belongs to (0-2: face-down, 3-5: face-up, 6-8: hand)
        let set_index = card_index % 3; // Position within the set (0, 1, or 2)
        let set_type = card_index / 3; // Which set (0: face-down, 1: face-up, 2: hand)
        
        // Add the card to the appropriate player's collection
        match set_type {
            0 => self.players[player_index].face_down_cards.push(card_entity),
            1 => self.players[player_index].face_up_cards.push(card_entity),
            2 => self.players[player_index].hand.push(card_entity),
            _ => unreachable!(),
        }
        
        // Calculate the target position
        let y_position = if player_index == 0 {
            -200.0 // Bottom player (Player 1)
        } else {
            200.0 // Top player (Player 2)
        };
        
        // Position cards in appropriate rows
        let row_offset = match set_type {
            0 => -30.0, // Face-down cards
            1 => 30.0,  // Face-up cards
            2 => 90.0,  // Hand cards
            _ => unreachable!(),
        };
        
        let x_position = (set_index as f32 - 1.0) * (100.0 + 20.0); // Center the cards
        
        // Set z-index based on card type and position
        let z_index = match set_type {
            0 => 0.0 + (set_index as f32 * 0.1),    // Face-down cards
            1 => 100.0 + (set_index as f32 * 0.1),  // Face-up cards
            2 => 200.0 + (set_index as f32 * 0.1),  // Hand cards
            _ => unreachable!(),
        };
        
        // Add animation component
        commands.entity(card_entity)
            .insert(CardAnimation {
                target_position: Vec3::new(x_position, y_position + row_offset, z_index),
                start_position: Vec3::new(0.0, 0.0, 0.0), // Start from center
                progress: 0.0,
                speed: 2.0, // Adjust speed as needed
            });
        
        // Update the card's face-up state
        if let Ok(card) = cards.get(card_entity) {
            commands.entity(card_entity).insert(Card {
                is_face_up, // Use the value from the tuple, not set_type
                ..*card
            });
        }

        true
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    // ── MatchState::score_for_position ────────────────────────────────────

    #[test]
    fn score_for_position_four_player() {
        assert_eq!(MatchState::score_for_position(0, 4), 3); // 1st
        assert_eq!(MatchState::score_for_position(1, 4), 2);
        assert_eq!(MatchState::score_for_position(2, 4), 1);
        assert_eq!(MatchState::score_for_position(3, 4), 0); // Shed
    }

    #[test]
    fn score_for_position_three_player() {
        assert_eq!(MatchState::score_for_position(0, 3), 2);
        assert_eq!(MatchState::score_for_position(1, 3), 1);
        assert_eq!(MatchState::score_for_position(2, 3), 0);
    }

    #[test]
    fn score_for_position_out_of_range_is_zero() {
        assert_eq!(MatchState::score_for_position(5, 4), 0);
    }

    // ── MatchState::award_round ───────────────────────────────────────────

    #[test]
    fn award_round_assigns_points_and_marks_previous_shed() {
        let mut ms = MatchState::new(4, 10);
        // Finish order: player 0 first, 2, 1, 3 last (Shed)
        assert!(ms.award_round(&[0, 2, 1, 3]));
        assert_eq!(ms.scores, vec![3, 1, 2, 0]);
        assert_eq!(ms.previous_shed, Some(3));
        assert!(ms.current_round_scored);
        assert!(ms.match_winner.is_none()); // target 10 not yet reached
    }

    #[test]
    fn award_round_is_idempotent_within_a_round() {
        let mut ms = MatchState::new(4, 10);
        assert!(ms.award_round(&[0, 1, 2, 3]));
        // Second call same round: no-op, scores unchanged.
        assert!(!ms.award_round(&[0, 1, 2, 3]));
        assert_eq!(ms.scores, vec![3, 2, 1, 0]);
    }

    #[test]
    fn award_round_no_op_on_empty_finish_order() {
        let mut ms = MatchState::new(4, 10);
        assert!(!ms.award_round(&[]));
        assert_eq!(ms.scores, vec![0, 0, 0, 0]);
        assert!(!ms.current_round_scored);
    }

    #[test]
    fn award_round_sets_match_winner_when_target_reached() {
        // Low target so a single round crosses it.
        let mut ms = MatchState::new(4, 3);
        ms.award_round(&[0, 1, 2, 3]); // player 0 gets 3 points
        assert_eq!(ms.match_winner, Some(0));
        assert!(ms.is_match_over());
    }

    #[test]
    fn award_round_picks_higher_scorer_on_simultaneous_target_cross() {
        // Both p0 and p1 start at 2 with target 3, then both cross on this
        // round (p1 gains 2 → 4, p0 gains 1 → 3). Must pick p1 because the
        // impl uses max_by_key on cumulative score.
        let mut ms = MatchState::new(3, 3);
        ms.scores[0] = 2;
        ms.scores[1] = 2;
        ms.award_round(&[1, 0, 2]); // p1 first → +2, p0 second → +1, p2 third → +0
        assert_eq!(ms.scores, vec![3, 4, 0]);
        assert_eq!(ms.match_winner, Some(1));
    }

    #[test]
    fn start_next_round_clears_scored_flag_and_bumps_round() {
        let mut ms = MatchState::new(4, 10);
        ms.award_round(&[0, 1, 2, 3]);
        let prev_round = ms.round;
        ms.start_next_round();
        assert_eq!(ms.round, prev_round + 1);
        assert!(!ms.current_round_scored);
        // Scores persist across rounds.
        assert_eq!(ms.scores, vec![3, 2, 1, 0]);
    }

    // ── MatchState::generate_personas ─────────────────────────────────────

    #[test]
    fn generate_personas_yields_requested_count() {
        let personas = MatchState::generate_personas(3);
        assert_eq!(personas.len(), 3);
    }

    #[test]
    fn generate_personas_disambiguates_duplicates() {
        // 9 draws from a 3-personality pool guarantees duplicates by pigeonhole.
        // Property: for every base name, the display names should be exactly
        // "Base", "Base 2", "Base 3", … in occurrence order.
        let personas = MatchState::generate_personas(9);
        let mut groups: std::collections::HashMap<&str, Vec<&str>> = Default::default();
        for p in &personas {
            let base = p.display_name.split(' ').next().unwrap();
            groups.entry(base).or_default().push(&p.display_name);
        }
        for (base, names) in &groups {
            for (i, &name) in names.iter().enumerate() {
                let expected = if i == 0 { base.to_string() } else { format!("{} {}", base, i + 1) };
                assert_eq!(name, expected);
            }
        }
    }

    // ── Player::has_buff / try_consume ────────────────────────────────────

    fn make_player_with(buffs: Vec<ActiveBuff>) -> Player {
        Player {
            id: 0,
            name: "test".to_string(),
            face_up_cards: Vec::new(),
            face_down_cards: Vec::new(),
            hand: Vec::new(),
            eliminated: false,
            personality: Personality::Rob,
            modifiers: buffs,
        }
    }

    #[test]
    fn has_buff_returns_false_when_missing() {
        let p = make_player_with(vec![]);
        assert!(!p.has_buff(BuffKind::Mulligan));
    }

    #[test]
    fn has_buff_returns_true_when_present() {
        let p = make_player_with(vec![ActiveBuff {
            kind: BuffKind::Mulligan,
            used_this_round: false,
        }]);
        assert!(p.has_buff(BuffKind::Mulligan));
    }

    #[test]
    fn try_consume_returns_false_when_missing() {
        let mut p = make_player_with(vec![]);
        assert!(!p.try_consume(BuffKind::Mulligan));
    }

    #[test]
    fn try_consume_first_call_succeeds_then_locks() {
        let mut p = make_player_with(vec![ActiveBuff {
            kind: BuffKind::Mulligan,
            used_this_round: false,
        }]);
        assert!(p.try_consume(BuffKind::Mulligan));
        assert!(p.modifiers[0].used_this_round);
        // Second call same round → false (already used).
        assert!(!p.try_consume(BuffKind::Mulligan));
    }
}
