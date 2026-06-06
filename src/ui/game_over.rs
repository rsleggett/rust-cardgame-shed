//! Game-over overlay (per-round + match-over). Press-any-key dismisses it and
//! triggers `restart_game_system`, which tears down round-scoped entities and
//! either starts the next round or seeds a fresh match.

use bevy::prelude::*;

use crate::components::card::Card;
use crate::components::game::{GamePhase, GameState, MatchState};
use crate::game_plugin::{add_match_players, MATCH_TARGET, PLAYER_COUNT};
use crate::rendering::card_constants::{CARD_HEIGHT, PLAY_PILE_X};
use crate::systems::swap::SwapState;
use crate::theme;
use crate::ui::pile_status::PileStatusText;
use crate::ui::score_hud::{ordinal, player_display_name};

/// First letter of a display name, for the row avatar monogram.
fn monogram(name: &str) -> String {
    name.chars().next().unwrap_or('?').to_uppercase().to_string()
}

#[derive(Component)]
pub(crate) struct GameOverScreen;

pub(crate) fn game_over_screen_system(
    mut commands: Commands,
    game_state: Res<GameState>,
    mut match_state: ResMut<MatchState>,
    screen_q: Query<Entity, With<GameOverScreen>>,
    asset_server: Res<AssetServer>,
) {
    if game_state.phase != GamePhase::GameOver || !screen_q.is_empty() {
        return;
    }

    // Award round points once. Idempotent — safe to call every frame, but
    // the screen_q.is_empty() guard above means we only get here once anyway.
    match_state.award_round(&game_state.finish_order);

    let total = game_state.players.len();
    let human_position = game_state.finish_order.iter().position(|&i| i == 0);
    let human_points_this_round = human_position
        .map(|p| MatchState::score_for_position(p, total))
        .unwrap_or(0);

    let title = if let Some(winner) = match_state.match_winner {
        if winner == 0 {
            "Match Won!".to_string()
        } else {
            format!("Match Over — {} wins", player_display_name(&game_state, winner))
        }
    } else {
        match human_position {
            Some(0) => "Round Won!".to_string(),
            Some(n) if n + 1 == total => "You're the Shed".to_string(),
            Some(n) => format!("You finished {}", ordinal(n + 1)),
            None => "Round Over".to_string(),
        }
    };

    let subtitle = if match_state.is_match_over() {
        format!("Final scores · target was {}", match_state.target)
    } else {
        format!(
            "+{} points this round · {} to {}",
            human_points_this_round, match_state.scores[0], match_state.target
        )
    };

    let cta = if match_state.is_match_over() {
        "Press any key for a new match"
    } else {
        "Press any key for the next round"
    };

    let ui_font = asset_server.load("fonts/Rubik-Regular.ttf");
    let pixel_font = asset_server.load("fonts/Silkscreen-Regular.ttf");

    let title_color = if matches!(human_position, Some(0)) || match_state.match_winner == Some(0) {
        theme::GOLD
    } else {
        theme::MAGENTA
    };

    commands.spawn((
        GameOverScreen,
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            background_color: theme::VEIL.into(),
            ..default()
        },
    )).with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            title,
            TextStyle { font: pixel_font.clone(), font_size: 48.0, color: title_color },
        ));

        parent.spawn(TextBundle::from_section(
            subtitle,
            TextStyle { font: ui_font.clone(), font_size: 16.0, color: theme::MUTED_TEXT },
        ));

        // Finish-order rows inside a felt panel.
        parent
            .spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(14.0)),
                    margin: UiRect::vertical(Val::Px(10.0)),
                    row_gap: Val::Px(6.0),
                    min_width: Val::Px(360.0),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                background_color: theme::PANEL.into(),
                border_color: theme::GOLD.with_alpha(0.35).into(),
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..default()
            })
            .with_children(|panel| {
                let max_score = match_state.scores.iter().copied().max().unwrap_or(0).max(match_state.target);
                for (rank_idx, &player_idx) in game_state.finish_order.iter().enumerate() {
                    let is_shed = rank_idx + 1 == total;
                    let label = if is_shed { "Shed".to_string() } else { ordinal(rank_idx + 1) };
                    let name = player_display_name(&game_state, player_idx);
                    let cumulative = match_state.scores.get(player_idx).copied().unwrap_or(0);
                    let gained = MatchState::score_for_position(rank_idx, total);
                    let seat = game_state
                        .players
                        .get(player_idx)
                        .map(|p| theme::seat_color(player_idx, p.personality))
                        .unwrap_or(theme::MUTED_TEXT);
                    let row_bg = if rank_idx == 0 {
                        theme::GOLD.with_alpha(0.12)
                    } else {
                        Color::srgba(1.0, 1.0, 1.0, 0.03)
                    };

                    panel
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(10.0),
                                padding: UiRect::all(Val::Px(6.0)),
                                ..default()
                            },
                            background_color: row_bg.into(),
                            border_radius: BorderRadius::all(Val::Px(6.0)),
                            ..default()
                        })
                        .with_children(|row| {
                            // Position label (seat-coloured pixel font).
                            row.spawn(TextBundle {
                                text: Text::from_section(
                                    label,
                                    TextStyle { font: pixel_font.clone(), font_size: 13.0, color: seat },
                                ),
                                style: Style { min_width: Val::Px(46.0), ..default() },
                                ..default()
                            });

                            // Avatar monogram chip.
                            row.spawn(NodeBundle {
                                style: Style {
                                    width: Val::Px(28.0),
                                    height: Val::Px(28.0),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                background_color: seat.with_alpha(0.30).into(),
                                border_radius: BorderRadius::all(Val::Px(14.0)),
                                ..default()
                            })
                            .with_children(|av| {
                                av.spawn(TextBundle::from_section(
                                    monogram(&name),
                                    TextStyle { font: pixel_font.clone(), font_size: 12.0, color: seat },
                                ));
                            });

                            // Name.
                            row.spawn(TextBundle {
                                text: Text::from_section(
                                    name,
                                    TextStyle { font: ui_font.clone(), font_size: 18.0, color: Color::WHITE },
                                ),
                                style: Style { min_width: Val::Px(90.0), ..default() },
                                ..default()
                            });

                            // "THE SHED" tag for the last-place finisher.
                            if is_shed {
                                row.spawn(NodeBundle {
                                    style: Style {
                                        padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                                        ..default()
                                    },
                                    background_color: theme::MAGENTA.into(),
                                    border_radius: BorderRadius::all(Val::Px(5.0)),
                                    ..default()
                                })
                                .with_children(|tag| {
                                    tag.spawn(TextBundle::from_section(
                                        "THE SHED",
                                        TextStyle { font: pixel_font.clone(), font_size: 9.0, color: Color::WHITE },
                                    ));
                                });
                            }

                            // Points this round.
                            let gained_color = if gained > 0 { theme::LIME } else { theme::MUTED_TEXT };
                            row.spawn(TextBundle {
                                text: Text::from_section(
                                    format!("+{}", gained),
                                    TextStyle { font: pixel_font.clone(), font_size: 13.0, color: gained_color },
                                ),
                                style: Style {
                                    min_width: Val::Px(36.0),
                                    margin: UiRect::left(Val::Auto),
                                    ..default()
                                },
                                ..default()
                            });

                            // Run-total bar (track + cyan→gold fill) plus the numeral.
                            row.spawn(NodeBundle {
                                style: Style {
                                    width: Val::Px(80.0),
                                    height: Val::Px(10.0),
                                    ..default()
                                },
                                background_color: Color::srgba(1.0, 1.0, 1.0, 0.10).into(),
                                border_radius: BorderRadius::all(Val::Px(5.0)),
                                ..default()
                            })
                            .with_children(|track| {
                                let frac = (cumulative as f32 / max_score as f32).clamp(0.0, 1.0);
                                let fill = if cumulative >= match_state.target { theme::GOLD } else { theme::CYAN };
                                track.spawn(NodeBundle {
                                    style: Style {
                                        width: Val::Percent(frac * 100.0),
                                        height: Val::Percent(100.0),
                                        ..default()
                                    },
                                    background_color: fill.into(),
                                    border_radius: BorderRadius::all(Val::Px(5.0)),
                                    ..default()
                                });
                            });
                            row.spawn(TextBundle::from_section(
                                format!("{}", cumulative),
                                TextStyle { font: pixel_font.clone(), font_size: 13.0, color: theme::GOLD },
                            ));
                        });
                }
            });

        // Chunky CTA prompt (lime fill, dark bottom edge).
        parent
            .spawn(NodeBundle {
                style: Style {
                    margin: UiRect::top(Val::Px(10.0)),
                    padding: UiRect::axes(Val::Px(18.0), Val::Px(10.0)),
                    border: UiRect::bottom(Val::Px(5.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                background_color: theme::LIME.into(),
                border_color: theme::chunky_shadow(theme::LIME).into(),
                border_radius: BorderRadius::all(Val::Px(12.0)),
                ..default()
            })
            .with_children(|btn| {
                btn.spawn(TextBundle::from_section(
                    cta,
                    TextStyle { font: ui_font, font_size: 16.0, color: Color::srgb(0.10, 0.06, 0.10) },
                ));
            });
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn restart_game_system(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    mut match_state: ResMut<MatchState>,
    mut swap_state: ResMut<SwapState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    card_q: Query<Entity, With<Card>>,
    screen_q: Query<Entity, With<GameOverScreen>>,
    status_q: Query<Entity, With<PileStatusText>>,
    asset_server: Res<AssetServer>,
) {
    if game_state.phase != GamePhase::GameOver { return; }
    // Any key, click, or tap advances — so the overlay is dismissable on a phone
    // that has no keyboard.
    let advance = keyboard.get_just_pressed().next().is_some()
        || mouse.just_pressed(MouseButton::Left)
        || touches.iter_just_pressed().next().is_some();
    if !advance { return; }

    *swap_state = SwapState::default();

    let match_was_over = match_state.is_match_over();
    // The previous round's Shed deals first next round (Shed punishment). Captured
    // before we reset MatchState in the match-over branch.
    let dealer = match_state.previous_shed.unwrap_or(0);

    // Despawn round-scoped entities. ScoreHud persists across restarts.
    for e in card_q.iter() { commands.entity(e).despawn_recursive(); }
    for e in screen_q.iter() { commands.entity(e).despawn_recursive(); }
    for e in status_q.iter() { commands.entity(e).despawn_recursive(); }

    // MatchState transition first so that `add_match_players` reads the
    // correct personas (a fresh roster on new-match, the same as before
    // on next-round).
    if match_was_over {
        *match_state = MatchState::new(PLAYER_COUNT, MATCH_TARGET);
        info!("Match reset — new opponents drawn");
    } else {
        match_state.start_next_round();
        info!("Round {} starting — seat {} plays first", match_state.round, dealer);
    }

    // Reset GameState for a fresh deal.
    *game_state = GameState::new();
    add_match_players(&mut game_state, &match_state);

    if !match_was_over {
        // Previous Shed plays first this round.
        game_state.current_player = dealer.min(PLAYER_COUNT.saturating_sub(1));
    }

    let ui_font = asset_server.load("fonts/Rubik-Regular.ttf");
    let suit_font = asset_server.load("fonts/NotoSansSymbols2-Regular.ttf");
    let pixel_font = asset_server.load("fonts/Silkscreen-Regular.ttf");
    game_state.prepare_dealing(&mut commands, ui_font, suit_font, pixel_font.clone());

    commands.spawn((
        PileStatusText,
        Text2dBundle {
            text: Text::from_section(
                "",
                TextStyle { font: pixel_font, font_size: 16.0, color: theme::GOLD },
            ),
            transform: Transform::from_xyz(PLAY_PILE_X, CARD_HEIGHT / 2.0 + 24.0, 600.0),
            ..default()
        },
    ));
}
