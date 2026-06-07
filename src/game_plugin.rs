//! Plugin wiring: resource registration, system scheduling, and one-time setup
//! of the table (initial deal, world-space UI, buttons, score widget). All
//! gameplay logic lives in `systems::*`; visuals + interactive widgets live
//! in `ui::*`; audio lives in `audio`.

use bevy::prelude::*;

use crate::audio::{setup_music, toggle_music_mute, MusicMuted};
use crate::components::game::{ActiveBuff, GameState, MatchState, Personality};
use crate::rendering::card_constants::{CARD_HEIGHT, PLAY_PILE_X};
use crate::rendering::card_renderer::CardRendererPlugin;
use crate::systems::ai_runner::{ai_player_system, AITimer};
use crate::systems::consumables::{
    handle_mulligan_key, handle_peek_key, tick_peek_timer, PeekRevealTimer,
};
use crate::systems::dealing::{deal_cards_system, draw_first_card_system, DealTimer};
use crate::systems::draft::{
    ai_draft_system, apply_picks_system, draft_screen_system, handle_draft_click,
    setup_draft_system, DraftState,
};
use crate::systems::input::{
    confirm_play_system, handle_invalid_card_event, handle_mouse_input, tick_last_click,
    update_hovered_card, HoveredCard, InvalidCardClicked, InvalidFeedbackTimer, LastClick,
};
use crate::systems::play::{
    check_valid_plays_system, draw_refill_system, handle_card_pickup_system,
};
use crate::systems::swap::{
    advance_swap_phase, ai_swap_system, handle_done_swap_button, handle_swap_input,
    update_swap_button_visibility, DoneSwapButton, SwapState,
};
use crate::systems::visuals::update_card_face_up_state;
use crate::theme;
use crate::ui::game_over::{game_over_screen_system, restart_game_system};
use crate::ui::pile_status::{update_pile_status_text, PileStatusText};
use crate::ui::play_button::{
    depress_buttons, handle_play_button, update_play_button_style, PlayButton,
};
use crate::ui::responsive::apply_responsive_layout;
use crate::ui::rules_panel::{spawn_rules_info_panel, update_rules_info_panel};
use crate::ui::score_hud::{spawn_score_hud, update_score_hud};

/// Number of seats at the table. Used both for player setup and for sizing
/// MatchState.scores. Change this and the seat-positioning code together.
pub(crate) const PLAYER_COUNT: usize = 4;
/// Cumulative points needed to win a match.
pub(crate) const MATCH_TARGET: u32 = 10;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CardRendererPlugin)
            .insert_resource(GameState::new())
            .insert_resource(MatchState::new(PLAYER_COUNT, MATCH_TARGET))
            .insert_resource(DraftState::default())
            .insert_resource(SwapState::default())
            .insert_resource(PeekRevealTimer::default())
            .insert_resource(DealTimer::new())
            .insert_resource(AITimer::new())
            .insert_resource(HoveredCard::default())
            .insert_resource(LastClick::default())
            .insert_resource(InvalidFeedbackTimer::default())
            .insert_resource(MusicMuted::default())
            .add_event::<InvalidCardClicked>()
            .add_systems(Startup, (setup_game, setup_music))
            .add_systems(Update, (
                update_hovered_card,
                tick_last_click,
                handle_mouse_input,
                handle_invalid_card_event,
                confirm_play_system,
                handle_play_button,
                update_play_button_style,
                deal_cards_system,
                update_card_face_up_state,
                draw_first_card_system,
                draw_refill_system,
                check_valid_plays_system,
                handle_card_pickup_system,
                ai_player_system,
                update_pile_status_text,
                update_score_hud,
                game_over_screen_system,
                restart_game_system,
            ))
            .add_systems(Update, (
                // Draft + consumables — grouped because the first set is at the 20-system limit.
                setup_draft_system,
                draft_screen_system,
                handle_draft_click,
                ai_draft_system,
                apply_picks_system,
                handle_mulligan_key,
                handle_peek_key,
                tick_peek_timer,
            ))
            .add_systems(Update, (
                // Swap phase systems — separate block since the first two are full.
                handle_swap_input,
                handle_done_swap_button,
                ai_swap_system,
                advance_swap_phase,
                update_swap_button_visibility,
                toggle_music_mute,
                update_rules_info_panel,
                apply_responsive_layout,
                depress_buttons,
            ));
    }
}

/// Seats the human at index 0, then one Player per persona on MatchState.
/// Used on startup, on next-round restart, and on new-match restart so the
/// labels and personalities always line up with `match_state.personas`. Per-seat
/// modifiers are restored from `match_state.persistent_modifiers` so that buffs
/// carry over between rounds — with every consumable's `used_this_round` reset
/// for the new round.
pub(crate) fn add_match_players(game_state: &mut GameState, match_state: &MatchState) {
    let mods_for = |seat: usize| -> Vec<ActiveBuff> {
        match_state
            .persistent_modifiers
            .get(seat)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|mut b| {
                b.used_this_round = false;
                b
            })
            .collect()
    };

    // Human's personality is a placeholder — the AI dispatcher never reads it.
    game_state.add_player("You".to_string(), Personality::Rob, mods_for(0));
    for (i, persona) in match_state.personas.iter().enumerate() {
        let seat = i + 1;
        game_state.add_player(
            persona.display_name.clone(),
            persona.personality,
            mods_for(seat),
        );
    }
}

fn setup_game(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    match_state: Res<MatchState>,
    asset_server: Res<AssetServer>,
) {
    add_match_players(&mut game_state, &match_state);

    let ui_font = asset_server.load("fonts/Rubik-Regular.ttf");
    let rank_font = asset_server.load("fonts/TitanOne-Regular.ttf");
    let suit_font = asset_server.load("fonts/NotoSansSymbols2-Regular.ttf");
    let pixel_font = asset_server.load("fonts/Silkscreen-Regular.ttf");
    game_state.prepare_dealing(&mut commands, rank_font, suit_font, pixel_font.clone());

    // Pile status text — world-space Text2d above the play pile (gold neon prompt)
    commands.spawn((
        PileStatusText,
        Text2dBundle {
            text: Text::from_section(
                "",
                TextStyle { font: pixel_font.clone(), font_size: 16.0, color: theme::GOLD },
            ),
            transform: Transform::from_xyz(PLAY_PILE_X, CARD_HEIGHT / 2.0 + 24.0, 600.0),
            ..default()
        },
    ));

    // "Play Cards" button — chunky lime, bottom-centre. Active styling toggled
    // by update_play_button_style.
    spawn_chunky_button(
        &mut commands,
        PlayButton,
        "PLAY >",
        theme::LIME,
        128.0,
        false,
        ui_font.clone(),
    );

    // "Done Swapping" button — shares the play button's slot, hidden by default.
    spawn_chunky_button(
        &mut commands,
        DoneSwapButton,
        "DONE SWAPPING",
        theme::CYAN,
        160.0,
        true,
        ui_font.clone(),
    );

    spawn_score_hud(&mut commands, ui_font.clone(), pixel_font.clone());
    spawn_rules_info_panel(&mut commands, ui_font, pixel_font);

    info!("Game setup complete! Ready to deal cards.");
}

/// Spawns a Balatro-style "chunky" button: bright fill, a solid darker bottom
/// edge (faked with a thick bottom border, since bevy_ui 0.14 has no box-shadow),
/// a translucent white rim (`Outline`), and rounded corners. Bottom-centre,
/// absolute-positioned. Shared by the Play and Done-Swapping buttons.
pub(crate) fn spawn_chunky_button<M: Component>(
    commands: &mut Commands,
    marker: M,
    label: &str,
    fill: Color,
    width: f32,
    hidden: bool,
    font: Handle<Font>,
) {
    commands
        .spawn((
            marker,
            ButtonBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(24.0),
                    left: Val::Percent(50.0),
                    margin: UiRect::left(Val::Px(-width / 2.0)),
                    width: Val::Px(width),
                    height: Val::Px(44.0),
                    border: UiRect::bottom(Val::Px(5.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    display: if hidden { Display::None } else { Display::Flex },
                    ..default()
                },
                background_color: fill.into(),
                border_color: theme::chunky_shadow(fill).into(),
                border_radius: BorderRadius::all(Val::Px(12.0)),
                ..default()
            },
            Outline {
                width: Val::Px(2.0),
                offset: Val::Px(0.0),
                color: Color::srgba(1.0, 1.0, 1.0, 0.35),
            },
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                label,
                TextStyle { font, font_size: 16.0, color: Color::srgb(0.10, 0.06, 0.10) },
            ));
        });
}
