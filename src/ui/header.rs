//! Persistent top header bar — the game name ("SHED") plus the current phase,
//! and, during the Swap phase, the swap instructions. Screen-space (so it
//! ignores the camera's design-rect letterboxing and always hugs the real top
//! edge of the canvas), spanning the full width in both orientations.
//!
//! The bar auto-grows: a single compact line normally, expanding to two or three
//! lines while the swap hint is showing. The seat avatars sit below it (the seat
//! anchors leave clearance at the top of the design rect) so the header never
//! overlaps the AI players.

use bevy::prelude::*;

use crate::components::game::{GamePhase, GameState};
use crate::systems::swap::SwapState;
use crate::theme;

/// Root marker on the header bar.
#[derive(Component)]
pub(crate) struct GameHeader;

/// Marker on the phase-label text section so it can be rewritten each frame.
#[derive(Component)]
pub(crate) struct HeaderPhaseText;

/// Marker on the secondary hint line (swap instructions). Empty outside Swap.
#[derive(Component)]
pub(crate) struct HeaderHintText;

/// Spawns the persistent header at the top of the screen. Called once from
/// `setup_game`.
pub(crate) fn spawn_header(
    commands: &mut Commands,
    ui_font: Handle<Font>,
    pixel_font: Handle<Font>,
) {
    commands
        .spawn((
            GameHeader,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    // No fixed height — the column grows with its content so the
                    // swap hint can spill onto a 2nd/3rd line as needed.
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    row_gap: Val::Px(4.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                    border: UiRect::bottom(Val::Px(2.0)),
                    ..default()
                },
                background_color: Color::srgba(0.02, 0.071, 0.047, 0.92).into(),
                border_color: theme::GOLD.with_alpha(0.45).into(),
                z_index: ZIndex::Global(45),
                ..default()
            },
        ))
        .with_children(|bar| {
            // Title row: game name + current phase.
            bar.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(14.0),
                    ..default()
                },
                ..default()
            })
            .with_children(|row| {
                row.spawn(TextBundle::from_section(
                    "SHED",
                    TextStyle { font: pixel_font, font_size: 26.0, color: theme::GOLD },
                ));
                row.spawn((
                    HeaderPhaseText,
                    TextBundle::from_section(
                        "",
                        TextStyle {
                            font: ui_font.clone(),
                            font_size: 18.0,
                            color: Color::srgba(1.0, 1.0, 1.0, 0.92),
                        },
                    ),
                ));
            });

            // Secondary hint line — wraps to extra lines on a narrow screen.
            bar.spawn((
                HeaderHintText,
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font: ui_font,
                        font_size: 18.0,
                        color: Color::srgba(1.0, 1.0, 1.0, 0.95),
                    },
                )
                .with_text_justify(JustifyText::Center)
                .with_style(Style {
                    max_width: Val::Px(680.0),
                    ..default()
                }),
            ));
        });
}

/// Rewrites the header's phase label and (during Swap) the instruction line
/// every frame the content changes.
pub(crate) fn update_header(
    game_state: Res<GameState>,
    swap_state: Res<SwapState>,
    mut phase_q: Query<&mut Text, (With<HeaderPhaseText>, Without<HeaderHintText>)>,
    mut hint_q: Query<&mut Text, (With<HeaderHintText>, Without<HeaderPhaseText>)>,
) {
    if let Ok(mut text) = phase_q.get_single_mut() {
        let label = match game_state.phase {
            GamePhase::Dealing => "Dealing",
            GamePhase::Swap => "Swap Phase",
            GamePhase::Drafting => "Draft",
            GamePhase::Playing => "Playing",
            GamePhase::GameOver => "Round Over",
        };
        if text.sections[0].value != label {
            text.sections[0].value = label.to_string();
        }
    }

    if let Ok(mut text) = hint_q.get_single_mut() {
        let hint = if game_state.phase == GamePhase::Swap {
            if swap_state.human_done {
                "Waiting for opponents to finish swapping..."
            } else if swap_state.human_selected_hand.is_some() {
                "Now tap a glowing table card to swap it in."
            } else {
                "Tap a hand card, then a table card, to swap. Tap DONE SWAPPING when ready."
            }
        } else {
            ""
        };
        if text.sections[0].value != hint {
            text.sections[0].value = hint.to_string();
        }
    }
}
