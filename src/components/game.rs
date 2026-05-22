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
    pub winner: Option<usize>,   // Index of winning player, set when phase → GameOver
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
    pub hand: Vec<Entity>, // New field for the player's hand
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum GamePhase {
    Dealing,
    Playing,
    GameOver,
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
            winner: None,
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

    pub fn add_player(&mut self, name: String) -> usize {
        let id = self.players.len();
        self.players.push(Player {
            id,
            name,
            face_up_cards: Vec::new(),
            face_down_cards: Vec::new(),
            hand: Vec::new(), // Initialize empty hand
        });
        id
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
            self.phase = GamePhase::Playing;
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