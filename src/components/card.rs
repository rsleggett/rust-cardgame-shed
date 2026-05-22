use bevy::prelude::*;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Rank::Two   => "2",
            Rank::Three => "3",
            Rank::Four  => "4",
            Rank::Five  => "5",
            Rank::Six   => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine  => "9",
            Rank::Ten   => "10",
            Rank::Jack  => "J",
            Rank::Queen => "Q",
            Rank::King  => "K",
            Rank::Ace   => "A",
        };
        write!(f, "{s}")
    }
}

impl fmt::Display for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NotoSans-Regular doesn't cover U+2660-2667 (Miscellaneous Symbols block).
        // Using single letters until a suitable font is sourced.
        let s = match self {
            Suit::Hearts   => "♥",
            Suit::Diamonds => "♦",
            Suit::Clubs    => "♣",
            Suit::Spades   => "♠",
        };
        write!(f, "{s}")
    }
}

#[derive(Component, Clone)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
    pub is_face_up: bool,
    /// Whether rank/suit text should be visible. Face-up but stacked cards (play pile)
    /// set is_face_up=true but show_text=false so only the top card shows text.
    pub show_text: bool,
    /// Set by update_hovered_card; drives the yellow tint in update_card_visuals.
    pub is_hovered: bool,
    /// True when the card is staged for multi-play (click to toggle, Enter to confirm).
    pub is_selected: bool,
    /// Counts down from ~0.5 when the player clicks this card illegally; drives red flash.
    pub invalid_timer: f32,
}

impl Card {
    pub fn new(suit: Suit, rank: Rank) -> Self {
        Self {
            suit,
            rank,
            is_face_up: false,
            show_text: false,
            is_hovered: false,
            is_selected: false,
            invalid_timer: 0.0,
        }
    }


}

