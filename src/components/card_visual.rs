//! Card visuals — the single card factory (`spawn_card_complete`) and the
//! per-frame updater (`update_card_visuals`). "Arcade Felt" skin: warm paper
//! faces with a big centre suit glyph, corner indices, coloured neon badges +
//! glow rings on the special cards (2/3/7/10), and a dark neon card back.

use bevy::prelude::*;
use bevy::sprite::Anchor;
use crate::components::card::{Card, Suit};
use crate::components::game::{BuffKind, GamePhase, GameState};
use crate::rendering::card_constants::{CARD_WIDTH, CARD_HEIGHT};
use crate::rules::can_play_card;
use crate::theme;

/// The neon card-back base colour (dark indigo). The lattice pattern in the
/// mockup needs a texture/shader, so it's approximated with a flat base + a
/// glowing centre disc + "S" emblem — flagged as a deferred fidelity gap.
const BACK_BASE: Color = Color::srgb(0.082, 0.071, 0.227); // #15123a
const BACK_DISC: Color = Color::srgb(0.227, 0.184, 0.478); // #3a2f7a
const BACK_EMBLEM_TEXT: Color = Color::srgb(0.812, 0.776, 1.0); // #cfc6ff

/// Glow ring shown behind a *playable* special card.
#[derive(Component)]
pub struct CardGlow;

/// Deeper drop-shadow shown when a card is staged (raised).
#[derive(Component)]
pub struct CardShadow;

/// Face elements (corner index, centre suit, special badge) — visible only when
/// the card shows its face.
#[derive(Component)]
pub struct FaceElement;

/// The rotated bottom-right corner index — like `FaceElement` but also hidden on
/// special cards (whose badge takes that corner).
#[derive(Component)]
pub struct BottomRightIndex;

/// Card-back art (centre disc + emblem) — visible only when the card is face-down.
#[derive(Component)]
pub struct BackElement;

#[derive(Bundle)]
pub struct CardBundle {
    pub card: Card,
    sprite_bundle: SpriteBundle,
}

impl CardBundle {
    pub fn new(card: Card, position: Vec3) -> Self {
        let card_color = if card.is_face_up { theme::CARD_PAPER } else { BACK_BASE };
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
    pixel_font: Handle<Font>,
) -> Entity {
    let rank_text = format!("{}", card.rank);
    let suit_symbol = format!("{}", card.suit);
    let is_red = matches!(card.suit, Suit::Hearts | Suit::Diamonds);
    let ink = theme::card_text_color(is_red);

    let big = (CARD_HEIGHT * 0.30).round();
    let corner_rank = (CARD_HEIGHT * 0.16).round();
    let corner_suit = (CARD_HEIGHT * 0.13).round();
    let half_w = CARD_WIDTH / 2.0;
    let half_h = CARD_HEIGHT / 2.0;

    let special = theme::special_color(card.rank);

    commands
        .spawn(CardBundle::new(card.clone(), position))
        .with_children(|parent| {
            // Glow ring (behind the card) — shown for playable specials.
            parent.spawn((
                CardGlow,
                SpriteBundle {
                    sprite: Sprite {
                        color: special.unwrap_or(theme::GOLD),
                        custom_size: Some(Vec2::new(CARD_WIDTH + 12.0, CARD_HEIGHT + 12.0)),
                        ..default()
                    },
                    transform: Transform::from_xyz(0.0, 0.0, -0.15),
                    visibility: Visibility::Hidden,
                    ..default()
                },
            ));

            // Staged-card deep shadow (behind, offset down).
            parent.spawn((
                CardShadow,
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::srgba(0.0, 0.0, 0.0, 0.45),
                        custom_size: Some(Vec2::new(CARD_WIDTH + 8.0, CARD_HEIGHT + 8.0)),
                        ..default()
                    },
                    transform: Transform::from_xyz(2.0, -8.0, -0.2),
                    visibility: Visibility::Hidden,
                    ..default()
                },
            ));

            // Subtle ink rim hugging the paper.
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::srgba(0.0, 0.0, 0.0, 0.12),
                    custom_size: Some(Vec2::new(CARD_WIDTH + 2.0, CARD_HEIGHT + 2.0)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, -0.1),
                ..default()
            });

            // ── Face elements ──────────────────────────────────────────────
            // Top-left corner: rank over suit.
            parent.spawn((
                FaceElement,
                Text2dBundle {
                    text: Text::from_section(
                        rank_text.clone(),
                        TextStyle { font: rank_font.clone(), font_size: corner_rank, color: ink },
                    ),
                    transform: Transform::from_xyz(-half_w + 8.0, half_h - 8.0, 0.5),
                    text_anchor: Anchor::TopLeft,
                    ..default()
                },
            ));
            parent.spawn((
                FaceElement,
                Text2dBundle {
                    text: Text::from_section(
                        suit_symbol.clone(),
                        TextStyle { font: suit_font.clone(), font_size: corner_suit, color: ink },
                    ),
                    transform: Transform::from_xyz(-half_w + 8.0, half_h - 8.0 - corner_rank, 0.5),
                    text_anchor: Anchor::TopLeft,
                    ..default()
                },
            ));

            // Big centre suit glyph.
            parent.spawn((
                FaceElement,
                Text2dBundle {
                    text: Text::from_section(
                        suit_symbol.clone(),
                        TextStyle { font: suit_font.clone(), font_size: big, color: ink },
                    ),
                    transform: Transform::from_xyz(0.0, 0.0, 0.4),
                    ..default()
                },
            ));

            // Bottom-right rotated index (hidden on specials).
            parent.spawn((
                FaceElement,
                BottomRightIndex,
                Text2dBundle {
                    text: Text::from_section(
                        rank_text.clone(),
                        TextStyle { font: rank_font.clone(), font_size: corner_rank, color: ink },
                    ),
                    transform: Transform::from_xyz(half_w - 8.0, -half_h + 8.0, 0.5)
                        .with_rotation(Quat::from_rotation_z(std::f32::consts::PI)),
                    text_anchor: Anchor::TopLeft,
                    ..default()
                },
            ));
            parent.spawn((
                FaceElement,
                BottomRightIndex,
                Text2dBundle {
                    text: Text::from_section(
                        suit_symbol.clone(),
                        TextStyle { font: suit_font.clone(), font_size: corner_suit, color: ink },
                    ),
                    transform: Transform::from_xyz(half_w - 8.0, -half_h + 8.0 + corner_rank, 0.5)
                        .with_rotation(Quat::from_rotation_z(std::f32::consts::PI)),
                    text_anchor: Anchor::TopLeft,
                    ..default()
                },
            ));

            // Special badge — coloured pill + word, bottom centre.
            if let (Some(color), Some(label)) =
                (special, theme::special_badge(card.rank))
            {
                parent
                    .spawn((
                        FaceElement,
                        SpriteBundle {
                            sprite: Sprite {
                                color,
                                custom_size: Some(Vec2::new(CARD_WIDTH - 18.0, 16.0)),
                                ..default()
                            },
                            transform: Transform::from_xyz(0.0, -half_h + 14.0, 0.5),
                            ..default()
                        },
                    ))
                    .with_children(|pill| {
                        pill.spawn(Text2dBundle {
                            text: Text::from_section(
                                label,
                                TextStyle {
                                    font: pixel_font.clone(),
                                    font_size: 9.0,
                                    color: Color::WHITE,
                                },
                            ),
                            transform: Transform::from_xyz(0.0, 0.0, 0.1),
                            ..default()
                        });
                    });
            }

            // ── Card-back art ──────────────────────────────────────────────
            parent.spawn((
                BackElement,
                SpriteBundle {
                    sprite: Sprite {
                        color: BACK_DISC,
                        custom_size: Some(Vec2::new(CARD_WIDTH * 0.46, CARD_WIDTH * 0.46)),
                        ..default()
                    },
                    transform: Transform::from_xyz(0.0, 0.0, 0.3),
                    visibility: Visibility::Hidden,
                    ..default()
                },
            ));
            parent.spawn((
                BackElement,
                Text2dBundle {
                    text: Text::from_section(
                        "S",
                        TextStyle { font: pixel_font.clone(), font_size: 20.0, color: BACK_EMBLEM_TEXT },
                    ),
                    transform: Transform::from_xyz(0.0, 0.0, 0.4),
                    visibility: Visibility::Hidden,
                    ..default()
                },
            ));
        })
        .id()
}

/// Per-frame: body colour by state, plus show/hide of face / back / glow /
/// shadow children. Suit ink colour is fixed at spawn, so this only toggles
/// visibility and recolours the card body.
#[allow(clippy::type_complexity)]
pub fn update_card_visuals(
    game_state: Res<GameState>,
    mut card_query: Query<(Entity, &Card, &mut Sprite)>,
    children_q: Query<&Children>,
    mut child_q: Query<(
        &mut Visibility,
        Option<&FaceElement>,
        Option<&BottomRightIndex>,
        Option<&BackElement>,
        Option<&CardGlow>,
        Option<&CardShadow>,
    )>,
) {
    // Human playability context for the glow ring.
    let human_turn =
        game_state.phase == GamePhase::Playing && game_state.current_player == 0;
    let human = game_state.players.first();
    let has_counter7 = human
        .map(|p| p.modifiers.iter().any(|b| b.kind == BuffKind::Counter7))
        .unwrap_or(false);

    for (entity, card, mut sprite) in card_query.iter_mut() {
        sprite.color = if card.invalid_timer > 0.0 {
            let intensity = (card.invalid_timer * 10.0).sin().abs();
            Color::srgb(1.0, 0.3 + 0.4 * intensity, 0.3 + 0.4 * intensity)
        } else if !card.is_face_up {
            BACK_BASE
        } else if card.is_hovered {
            // Warm the paper toward gold to signal hover.
            Color::srgb(1.0, 0.96, 0.80)
        } else {
            theme::CARD_PAPER
        };

        let face_shown = card.is_face_up && card.show_text;
        let back_shown = !card.is_face_up;
        let is_special = theme::special_color(card.rank).is_some();
        let glow_on = is_special
            && face_shown
            && human_turn
            && human.map(|p| p.hand.contains(&entity)).unwrap_or(false)
            && can_play_card(
                card,
                game_state.effective_rank,
                game_state.seven_active,
                game_state.any_card_playable,
                has_counter7,
            );
        let shadow_on = card.is_selected;

        let Ok(children) = children_q.get(entity) else { continue; };
        for &child in children.iter() {
            if let Ok((mut vis, face, br, back, glow, shadow)) = child_q.get_mut(child) {
                let show = if glow.is_some() {
                    glow_on
                } else if shadow.is_some() {
                    shadow_on
                } else if back.is_some() {
                    back_shown
                } else if br.is_some() {
                    face_shown && !is_special
                } else if face.is_some() {
                    face_shown
                } else {
                    true // the static ink rim
                };
                *vis = if show { Visibility::Inherited } else { Visibility::Hidden };
            }
        }
    }
}
