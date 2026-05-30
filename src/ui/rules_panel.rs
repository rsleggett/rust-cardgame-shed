//! Bottom-left always-visible reference panel: a static rules/specials legend
//! plus the human's currently drafted buffs (with descriptions). Buff strings
//! are reused from `BuffKind` so the panel stays in sync with the draft screen.

use bevy::prelude::*;

use crate::components::game::GameState;

/// Static rules + special-card legend. Wording mirrors the rule comments in
/// `rules.rs` and the CLAUDE.md "Shed Rules" section.
const RULES_LEGEND: &str = "SHED\n\
Play a card >= the pile, or pick up.\n\
Empty hand -> face-up -> face-down to get out.\n\
Last player left is the Shed.\n\
\n\
Specials\n\
2   reset - play anything next\n\
3   invisible - pile unchanged\n\
7   next must play 7 or lower\n\
10  burns the pile - go again\n\
4x  same rank burns the pile";

/// Marker on the always-visible rules panel (bottom-left of the screen).
#[derive(Component)]
pub(crate) struct RulesInfoPanel;

/// Marker on the inner Text node of the rules panel. Updated each frame.
#[derive(Component)]
pub(crate) struct RulesInfoText;

/// Spawns the bottom-left rules/buffs panel. Persists across restarts; its text
/// is rewritten each frame by `update_rules_info_panel`.
pub(crate) fn spawn_rules_info_panel(commands: &mut Commands, font: Handle<Font>) {
    commands
        .spawn((
            RulesInfoPanel,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(12.0),
                    left: Val::Px(12.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    flex_direction: FlexDirection::Column,
                    max_width: Val::Px(400.0),
                    ..default()
                },
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.45).into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                RulesInfoText,
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font,
                        font_size: 17.0,
                        color: Color::srgba(1.0, 1.0, 1.0, 0.95),
                    },
                ),
            ));
        });
}

/// Refreshes the rules panel each frame: static legend + the human's active
/// buffs with their descriptions. Cheap single-section rebuild.
pub(crate) fn update_rules_info_panel(
    game_state: Res<GameState>,
    mut text_q: Query<&mut Text, With<RulesInfoText>>,
) {
    let Ok(mut text) = text_q.get_single_mut() else { return; };

    let mut body = String::from(RULES_LEGEND);
    body.push_str("\n\nYour buffs");

    let human_buffs = game_state.players.first().map(|p| &p.modifiers);
    match human_buffs {
        Some(mods) if !mods.is_empty() => {
            for b in mods {
                let mark = if b.kind.is_consumable() {
                    if b.used_this_round { "x" } else { "*" }
                } else {
                    ""
                };
                body.push_str(&format!("\n{}{}", b.kind.display_name(), mark));
                body.push_str(&format!("\n  {}", b.kind.description()));
            }
        }
        _ => body.push_str("\n(none yet - draft one each round)"),
    }

    text.sections[0].value = body;
}
