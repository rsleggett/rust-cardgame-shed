//! Consumable buff triggers: Mulligan swaps hand ↔ face-ups, Peek reveals the
//! human's face-downs for a few seconds. Each can be fired by keyboard (M / P)
//! or by clicking its mini-card in the bottom action bar.

use bevy::prelude::*;

use crate::components::game::{BuffKind, GamePhase, GameState};
use crate::theme;

/// Counts down while the human's face-down cards (and the top draw card) are
/// shown face-up because the human triggered Peek.
#[derive(Resource, Default)]
pub(crate) struct PeekRevealTimer(pub(crate) f32);

/// A clickable consumable mini-card in the bottom action bar.
#[derive(Component)]
pub(crate) struct ConsumableCard {
    pub kind: BuffKind,
}

// ── Core use-logic (shared by the keyboard + click handlers) ────────────────

/// Mulligan: once per round, on your turn, swap your hand with your face-ups.
fn use_mulligan(game_state: &mut GameState) -> bool {
    if game_state.phase != GamePhase::Playing || game_state.current_player != 0 {
        return false;
    }
    let player = &mut game_state.players[0];
    if !player.try_consume(BuffKind::Mulligan) {
        return false;
    }
    std::mem::swap(&mut player.hand, &mut player.face_up_cards);
    info!("Mulligan used: hand <-> face-up swapped");
    true
}

/// Peek: once per round, reveal your face-down cards for a few seconds.
fn use_peek(game_state: &mut GameState, peek_timer: &mut PeekRevealTimer) -> bool {
    if game_state.phase != GamePhase::Playing {
        return false;
    }
    let player = &mut game_state.players[0];
    if !player.try_consume(BuffKind::Peek) {
        return false;
    }
    peek_timer.0 = 3.0;
    info!("Peek used: revealing face-down cards for 3s");
    true
}

// ── Keyboard handlers ───────────────────────────────────────────────────────

pub(crate) fn handle_mulligan_key(
    mut game_state: ResMut<GameState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyM) {
        use_mulligan(&mut game_state);
    }
}

pub(crate) fn handle_peek_key(
    mut peek_timer: ResMut<PeekRevealTimer>,
    mut game_state: ResMut<GameState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyP) {
        use_peek(&mut game_state, &mut peek_timer);
    }
}

pub(crate) fn tick_peek_timer(mut peek_timer: ResMut<PeekRevealTimer>, time: Res<Time>) {
    if peek_timer.0 > 0.0 {
        peek_timer.0 = (peek_timer.0 - time.delta_seconds()).max(0.0);
    }
}

// ── Bottom action-bar mini-cards ────────────────────────────────────────────

/// Per-kind keyboard hint shown on the mini-card.
fn hotkey_hint(kind: BuffKind) -> &'static str {
    match kind {
        BuffKind::Mulligan => "M",
        BuffKind::Peek => "P",
        _ => "",
    }
}

/// Spawns the consumable mini-card row, parked at the bottom just right of the
/// centre Play/Done button. One card per consumable kind; each is hidden until
/// the human actually owns that buff (see `update_consumable_cards`).
pub(crate) fn spawn_consumable_bar(
    commands: &mut Commands,
    ui_font: Handle<Font>,
    pixel_font: Handle<Font>,
) {
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(16.0),
                left: Val::Percent(50.0),
                // Sit just to the right of the 128px-wide Play button (half = 64)
                // plus a gap, so the bar reads as button + consumables.
                margin: UiRect::left(Val::Px(80.0)),
                height: Val::Px(84.0),
                column_gap: Val::Px(10.0),
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|bar| {
            for kind in [BuffKind::Mulligan, BuffKind::Peek] {
                bar.spawn((
                    ConsumableCard { kind },
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(80.0),
                            height: Val::Px(84.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            row_gap: Val::Px(3.0),
                            border: UiRect::all(Val::Px(2.0)),
                            display: Display::None, // shown when owned + Playing
                            ..default()
                        },
                        background_color: theme::CARD_PAPER.into(),
                        border_color: theme::GOLD.into(),
                        border_radius: BorderRadius::all(Val::Px(7.0)),
                        ..default()
                    },
                ))
                .with_children(|card| {
                    card.spawn(TextBundle::from_section(
                        theme::buff_icon(kind),
                        TextStyle { font: pixel_font.clone(), font_size: 26.0, color: theme::CARD_INK },
                    ));
                    card.spawn(TextBundle::from_section(
                        kind.display_name(),
                        TextStyle { font: ui_font.clone(), font_size: 14.0, color: theme::CARD_INK },
                    ));
                    card.spawn(TextBundle::from_section(
                        format!("[{}]", hotkey_hint(kind)),
                        TextStyle { font: pixel_font.clone(), font_size: 12.0, color: theme::MUTED_TEXT },
                    ));
                });
            }
        });
}

/// Shows a mini-card only while the human owns that consumable and the round is
/// in play; greys it once it's been used this round.
pub(crate) fn update_consumable_cards(
    game_state: Res<GameState>,
    mut card_q: Query<(&ConsumableCard, &mut Style, &mut BackgroundColor, &mut BorderColor)>,
) {
    let human = game_state.players.first();
    let playing = game_state.phase == GamePhase::Playing;
    for (card, mut style, mut bg, mut border) in card_q.iter_mut() {
        let owned = human.map(|p| p.has_buff(card.kind)).unwrap_or(false);
        let used = human
            .and_then(|p| p.modifiers.iter().find(|b| b.kind == card.kind))
            .map(|b| b.used_this_round)
            .unwrap_or(false);

        style.display = if owned && playing { Display::Flex } else { Display::None };

        if used {
            *bg = theme::LOCKED_GREY.into();
            *border = theme::MUTED_TEXT.into();
        } else {
            *bg = theme::CARD_PAPER.into();
            *border = theme::GOLD.into();
        }
    }
}

/// Fires a consumable when its mini-card is clicked/tapped. The core use-logic
/// re-checks ownership / used state, so a click on a spent card is a no-op.
pub(crate) fn handle_consumable_click(
    mut game_state: ResMut<GameState>,
    mut peek_timer: ResMut<PeekRevealTimer>,
    interaction_q: Query<(&Interaction, &ConsumableCard), Changed<Interaction>>,
) {
    for (interaction, card) in &interaction_q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match card.kind {
            BuffKind::Mulligan => {
                use_mulligan(&mut game_state);
            }
            BuffKind::Peek => {
                use_peek(&mut game_state, &mut peek_timer);
            }
            _ => {}
        }
    }
}
