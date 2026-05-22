use bevy::prelude::*;
use bevy::sprite::Anchor;
use crate::components::card::{Card, Suit};
use crate::rendering::card_constants::{CARD_WIDTH, CARD_HEIGHT};

#[derive(Bundle)]
pub struct CardBundle {
    pub card: Card,
    sprite_bundle: SpriteBundle,
}

impl CardBundle {
    pub fn new(card: Card, position: Vec3) -> Self {
        let card_color = if !card.is_face_up {
            Color::srgb(0.0, 0.0, 1.0)
        } else {
            Color::srgb(0.95, 0.95, 0.95)
        };

        Self {
            card,
            sprite_bundle: SpriteBundle {
                sprite: Sprite {
                    color: card_color,
                    custom_size: Some(Vec2::new(CARD_WIDTH, CARD_HEIGHT)),
                    ..default()
                },
                transform: Transform::from_translation(position),
                ..default()
            },
        }
    }
}

pub fn spawn_card_complete(
    commands: &mut Commands,
    card: Card,
    position: Vec3,
    rank_font: Handle<Font>,
    suit_font: Handle<Font>,
) -> Entity {
    let rank_text = format!("{}", card.rank);
    let suit_symbol = format!("{}", card.suit);

    let text_color = if !card.is_face_up {
        Color::srgb(0.0, 0.0, 1.0)
    } else {
        match card.suit {
            Suit::Hearts | Suit::Diamonds => Color::srgb(1.0, 0.0, 0.0),
            Suit::Clubs | Suit::Spades => Color::srgb(0.0, 0.0, 0.0),
        }
    };

    commands
        .spawn(CardBundle::new(card, position))
        .with_children(|parent| {
            // Border
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::srgb(0.2, 0.2, 0.2),
                    custom_size: Some(Vec2::new(CARD_WIDTH + 4.0, CARD_HEIGHT + 4.0)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, -0.1),
                ..default()
            });

            // Rank — NotoSans
            parent.spawn(Text2dBundle {
                text: Text::from_section(
                    rank_text,
                    TextStyle { font: rank_font, font_size: 20.0, color: text_color },
                ),
                transform: Transform::from_xyz(-28.0, 40.0, 52.0),
                text_anchor: Anchor::TopLeft,
                ..default()
            });

            // Suit symbol — NotoSansSymbols2 (covers ♥♦♣♠)
            parent.spawn(Text2dBundle {
                text: Text::from_section(
                    suit_symbol,
                    TextStyle { font: suit_font, font_size: 20.0, color: text_color },
                ),
                transform: Transform::from_xyz(-28.0, 18.0, 52.0),
                text_anchor: Anchor::TopLeft,
                ..default()
            });
        })
        .id()
}

pub fn update_card_visuals(
    mut card_query: Query<(&Card, &mut Sprite, &Children)>,
    mut text_query: Query<(&mut Text, &mut Visibility)>,
) {
    for (card, mut sprite, children) in card_query.iter_mut() {
        sprite.color = if card.invalid_timer > 0.0 {
            let intensity = (card.invalid_timer * 10.0).sin().abs();
            Color::srgb(1.0, 0.3 + 0.4 * intensity, 0.3 + 0.4 * intensity)
        } else if !card.is_face_up {
            Color::srgb(0.0, 0.0, 1.0)
        } else if card.is_selected {
            Color::srgb(0.7, 0.95, 0.7)
        } else if card.is_hovered {
            Color::srgb(0.95, 0.95, 0.7)
        } else {
            Color::srgb(0.95, 0.95, 0.95)
        };

        let text_color = match card.suit {
            Suit::Hearts | Suit::Diamonds => Color::srgb(1.0, 0.0, 0.0),
            Suit::Clubs | Suit::Spades => Color::srgb(0.0, 0.0, 0.0),
        };
        let text_vis = if card.show_text { Visibility::Inherited } else { Visibility::Hidden };

        for &child in children.iter() {
            if let Ok((mut text, mut vis)) = text_query.get_mut(child) {
                *vis = text_vis;
                for section in text.sections.iter_mut() {
                    section.style.color = text_color;
                }
            }
        }
    }
}
