//! Game-over overlay (per-round + match-over). Press-any-key dismisses it and
//! triggers `restart_game_system`, which tears down round-scoped entities and
//! either starts the next round or seeds a fresh match.

use bevy::prelude::*;

use crate::components::card::Card;
use crate::components::game::{GamePhase, GameState, MatchState};
use crate::game_plugin::{add_match_players, MATCH_TARGET, PLAYER_COUNT};
use crate::rendering::card_constants::{CARD_HEIGHT, PLAY_PILE_X};
use crate::systems::swap::SwapState;
use crate::ui::pile_status::PileStatusText;
use crate::ui::score_hud::{ordinal, player_display_name};

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

    let font = asset_server.load("fonts/NotoSans-Regular.ttf");

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
                row_gap: Val::Px(10.0),
                ..default()
            },
            background_color: Color::srgba(0.0, 0.0, 0.0, 0.6).into(),
            ..default()
        },
    )).with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            title,
            TextStyle {
                font: font.clone(),
                font_size: 64.0,
                color: Color::WHITE,
            },
        ));

        parent.spawn(TextBundle::from_section(
            subtitle,
            TextStyle {
                font: font.clone(),
                font_size: 18.0,
                color: Color::srgba(1.0, 1.0, 1.0, 0.75),
            },
        ));

        for (rank_idx, &player_idx) in game_state.finish_order.iter().enumerate() {
            let label = if rank_idx + 1 == total {
                "Shed".to_string()
            } else {
                ordinal(rank_idx + 1)
            };
            let name = player_display_name(&game_state, player_idx);
            let cumulative = match_state.scores.get(player_idx).copied().unwrap_or(0);
            let gained = MatchState::score_for_position(rank_idx, total);
            let line_color = if player_idx == 0 {
                Color::srgb(1.0, 0.85, 0.3)
            } else {
                Color::srgba(1.0, 1.0, 1.0, 0.85)
            };
            parent.spawn(TextBundle::from_section(
                format!("{} — {}   +{}   ({} total)", label, name, gained, cumulative),
                TextStyle {
                    font: font.clone(),
                    font_size: 26.0,
                    color: line_color,
                },
            ));
        }

        parent.spawn(TextBundle {
            text: Text::from_section(
                cta,
                TextStyle {
                    font,
                    font_size: 22.0,
                    color: Color::srgba(1.0, 1.0, 1.0, 0.7),
                },
            ),
            style: Style {
                margin: UiRect::top(Val::Px(16.0)),
                ..default()
            },
            ..default()
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

    let font = asset_server.load("fonts/NotoSans-Regular.ttf");
    let suit_font = asset_server.load("fonts/NotoSansSymbols2-Regular.ttf");
    game_state.prepare_dealing(&mut commands, font.clone(), suit_font);

    commands.spawn((
        PileStatusText,
        Text2dBundle {
            text: Text::from_section(
                "",
                TextStyle { font, font_size: 16.0, color: Color::WHITE },
            ),
            transform: Transform::from_xyz(PLAY_PILE_X, CARD_HEIGHT / 2.0 + 24.0, 600.0),
            ..default()
        },
    ));
}
