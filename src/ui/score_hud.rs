//! Top-right round/score widget. Always visible; text is rewritten each frame.
//! Also hosts the shared display-name helpers used by the game-over screen.

use bevy::prelude::*;

use crate::components::game::{GameState, MatchState};
use crate::theme;

/// Marker on the always-visible score widget (top-right of the screen).
#[derive(Component)]
pub(crate) struct ScoreHud;

/// Marker on the inner Text node of the score widget. Updated each frame.
#[derive(Component)]
pub(crate) struct ScoreHudText;

/// Spawns the top-right round/score widget. Persists across restarts; its text
/// is rewritten each frame by `update_score_hud`.
pub(crate) fn spawn_score_hud(
    commands: &mut Commands,
    ui_font: Handle<Font>,
    pixel_font: Handle<Font>,
) {
    commands
        .spawn((
            ScoreHud,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(24.0),
                    right: Val::Px(24.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    min_width: Val::Px(170.0),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                background_color: theme::PANEL.into(),
                border_color: theme::GOLD.with_alpha(0.35).into(),
                border_radius: BorderRadius::all(Val::Px(9.0)),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                ScoreHudText,
                TextBundle::from_sections([
                    // Header (gold, pixel font) then the per-seat body (ui font).
                    TextSection::new(
                        "",
                        TextStyle { font: pixel_font, font_size: 13.0, color: theme::GOLD },
                    ),
                    TextSection::new(
                        "",
                        TextStyle {
                            font: ui_font,
                            font_size: 14.0,
                            color: Color::srgba(1.0, 1.0, 1.0, 0.95),
                        },
                    ),
                ]),
            ));
        });
}

pub(crate) fn ordinal(n: usize) -> String {
    match n {
        1 => "1st".into(),
        2 => "2nd".into(),
        3 => "3rd".into(),
        _ => format!("{}th", n),
    }
}

pub(crate) fn player_display_name(game_state: &GameState, player_idx: usize) -> String {
    if player_idx == 0 {
        "You".to_string()
    } else {
        game_state
            .players
            .get(player_idx)
            .map(|p| p.name.clone())
            .unwrap_or_default()
    }
}

/// Refreshes the round/score widget each frame. Cheap — single text section.
pub(crate) fn update_score_hud(
    match_state: Res<MatchState>,
    game_state: Res<GameState>,
    mut text_q: Query<&mut Text, With<ScoreHudText>>,
) {
    // Scores/round/buffs only change when GameState or MatchState does. Skip the
    // string allocations + Text re-layout on the (many) idle frames in between.
    if !match_state.is_changed() && !game_state.is_changed() {
        return;
    }

    let Ok(mut text) = text_q.get_single_mut() else { return; };

    let header = format!("Round {} · First to {}", match_state.round, match_state.target);
    let mut body = String::new();
    for i in 0..match_state.scores.len() {
        let name = player_display_name(&game_state, i);
        let score = match_state.scores[i];
        let marker = if Some(i) == match_state.match_winner { " *" } else { "" };
        body.push_str(&format!("\n{:<7} {:>3}{}", name, score, marker));

        // Active buffs, indented under the score row. Consumables suffix with
        // · (ready) or ✗ (used this round).
        if let Some(player) = game_state.players.get(i) {
            if !player.modifiers.is_empty() {
                let mut parts = Vec::with_capacity(player.modifiers.len());
                for b in &player.modifiers {
                    let label = b.kind.display_name();
                    if b.kind.is_consumable() {
                        let mark = if b.used_this_round { "x" } else { "*" };
                        parts.push(format!("{}{}", label, mark));
                    } else {
                        parts.push(label.to_string());
                    }
                }
                body.push_str(&format!("\n         {}", parts.join(", ")));
            }
        }
    }
    text.sections[0].value = header;
    text.sections[1].value = body;
}
