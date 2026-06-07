//! Persistent top header bar — the game name ("SHED") plus the current phase.
//! Screen-space (so it ignores the camera's design-rect letterboxing and always
//! hugs the real top edge of the canvas), spanning the full width in both
//! orientations. The seat avatars sit below it (the seat anchors leave clearance
//! at the top of the design rect) so the header never overlaps the AI players.

use bevy::prelude::*;

use crate::components::game::{GamePhase, GameState};
use crate::theme;

/// Root marker on the header bar.
#[derive(Component)]
pub(crate) struct GameHeader;

/// Marker on the phase-label text section so it can be rewritten each frame.
#[derive(Component)]
pub(crate) struct HeaderPhaseText;

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
                    height: Val::Px(44.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(14.0),
                    border: UiRect::bottom(Val::Px(2.0)),
                    ..default()
                },
                background_color: Color::srgba(0.02, 0.071, 0.047, 0.86).into(),
                border_color: theme::GOLD.with_alpha(0.45).into(),
                z_index: ZIndex::Global(45),
                ..default()
            },
        ))
        .with_children(|bar| {
            bar.spawn(TextBundle::from_section(
                "SHED",
                TextStyle { font: pixel_font, font_size: 26.0, color: theme::GOLD },
            ));
            bar.spawn((
                HeaderPhaseText,
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font: ui_font,
                        font_size: 18.0,
                        color: Color::srgba(1.0, 1.0, 1.0, 0.92),
                    },
                ),
            ));
        });
}

/// Rewrites the header's phase label every frame the phase changes.
pub(crate) fn update_header_phase(
    game_state: Res<GameState>,
    mut text_q: Query<&mut Text, With<HeaderPhaseText>>,
) {
    let Ok(mut text) = text_q.get_single_mut() else { return; };
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
