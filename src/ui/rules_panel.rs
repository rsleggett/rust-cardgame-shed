//! Bottom-left always-visible reference panel: a static rules/specials legend
//! plus the human's currently drafted buffs (with descriptions). Buff strings
//! are reused from `BuffKind` so the panel stays in sync with the draft screen.

use bevy::prelude::*;

use crate::components::game::GameState;
use crate::theme;

/// Static rules + special-card legend. Wording mirrors the rule comments in
/// `rules.rs` and the CLAUDE.md "Shed Rules" section. The "SHED" title is the
/// pixel-font header section; this body is the ui-font section beneath it.
const RULES_LEGEND: &str = "Play a card >= the pile, or pick up.\n\
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
pub(crate) fn spawn_rules_info_panel(
    commands: &mut Commands,
    ui_font: Handle<Font>,
    pixel_font: Handle<Font>,
) {
    commands
        .spawn((
            RulesInfoPanel,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(24.0),
                    left: Val::Px(24.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    flex_direction: FlexDirection::Column,
                    max_width: Val::Px(400.0),
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
                RulesInfoText,
                TextBundle::from_sections([
                    // Title (gold, pixel font) then the legend/buffs body (ui font).
                    TextSection::new(
                        "SHED\n",
                        TextStyle { font: pixel_font, font_size: 14.0, color: theme::GOLD },
                    ),
                    TextSection::new(
                        "",
                        TextStyle {
                            font: ui_font,
                            font_size: 15.0,
                            color: Color::srgba(1.0, 1.0, 1.0, 0.95),
                        },
                    ),
                ]),
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

    text.sections[1].value = body;
}
