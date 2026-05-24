use std::time::Duration;

use bevy::audio::{PlaybackSettings, Volume};
use bevy::prelude::*;
use crate::components::game::{
    ActiveBuff, BuffKind, GamePhase, GameState, MatchState, Personality,
};
use crate::components::card::{Card, Rank};
use crate::rendering::card_constants::{CARD_HEIGHT, CARD_WIDTH, PLAY_PILE_X, Z_INDEX_STEP};
use crate::rendering::card_renderer::{CardRendererPlugin, CardAnimation};

const AI_TICK_NORMAL: f32 = 1.5;
const AI_TICK_SPECTATE: f32 = 0.3;
/// Seconds between dealing each card during round setup. 36 cards × this value
/// is the total deal time.
const DEAL_INTERVAL: f32 = 0.15;

/// Number of seats at the table. Used both for player setup and for sizing
/// MatchState.scores. Change this and the seat-positioning code together.
const PLAYER_COUNT: usize = 4;
/// Cumulative points needed to win a match.
const MATCH_TARGET: u32 = 10;


#[derive(Resource)]
struct DealTimer(Timer);

#[derive(Resource)]
struct AITimer(Timer);

#[derive(Component)]
struct GameOverScreen;

#[derive(Component)]
struct PileStatusText;

#[derive(Component)]
struct PlayButton;

/// Bottom-centre button visible only during the Swap phase. Click → human is
/// done swapping. Shares the play button's slot via mutually exclusive
/// visibility.
#[derive(Component)]
struct DoneSwapButton;

/// Marker on the single looping background-music entity. Used by the mute
/// toggle to find the audio sink.
#[derive(Component)]
struct BackgroundMusic;

/// Current mute state for the background music. Persists across mute toggles
/// so we restore the player's preference after a track restarts.
#[derive(Resource, Default)]
struct MusicMuted(bool);

const MUSIC_VOLUME: f32 = 0.35;

/// Marker on the always-visible score widget (top-right of the screen).
#[derive(Component)]
struct ScoreHud;

/// Marker on the inner Text node of the score widget. Updated each frame.
#[derive(Component)]
struct ScoreHudText;

/// Marker on the full-screen draft overlay (one per round).
#[derive(Component)]
struct DraftScreen;

/// Marker on each clickable buff row inside the draft overlay.
#[derive(Component)]
struct DraftOption(BuffKind);

/// Counts down while the human's face-down cards (and the top draw card) are
/// shown face-up because the human triggered Peek.
#[derive(Resource, Default)]
struct PeekRevealTimer(f32);

/// Per-round draft state: one pool per seat, one optional pick per seat.
/// Re-populated by `setup_draft_system` whenever phase enters Drafting.
#[derive(Resource, Default)]
struct DraftState {
    pub pools: Vec<Vec<BuffKind>>,
    pub picks: Vec<Option<BuffKind>>,
}

impl DraftState {
    fn reset(&mut self, player_count: usize) {
        self.pools.clear();
        self.pools.resize_with(player_count, Vec::new);
        self.picks.clear();
        self.picks.resize(player_count, None);
    }

    fn all_picked(&self) -> bool {
        !self.picks.is_empty() && self.picks.iter().all(Option::is_some)
    }
}

/// Fires when the player clicks a card that cannot legally be played right now.
#[derive(Event)]
struct InvalidCardClicked(Entity);

/// Counts down while the pile-status text is highlighted in orange (invalid-play feedback).
#[derive(Resource, Default)]
struct InvalidFeedbackTimer(f32);

/// Which card the human is hovering (or None). Written by update_hovered_card,
/// read by layout_cards and update_card_visuals to raise/tint the card.
#[derive(Resource, Default)]
pub struct HoveredCard(pub Option<Entity>);

/// Tracks the most recently clicked card and how long ago. A second click on
/// the same card within `DOUBLE_CLICK_WINDOW` skips staging and plays it
/// directly. Cleared once the window lapses to avoid stale match-ups.
#[derive(Resource, Default)]
struct LastClick {
    entity: Option<Entity>,
    age: f32,
}

const DOUBLE_CLICK_WINDOW: f32 = 0.3;

/// Transient per-round state for the Swap phase. Reset on exit so the next
/// round begins with a clean slate.
#[derive(Resource, Default)]
struct SwapState {
    /// The human's currently-staged hand card waiting for a face-up partner.
    human_selected_hand: Option<Entity>,
    /// Which AIs have completed their swap heuristic this round. Indexed by
    /// seat - 1 (human is seat 0).
    ai_done: [bool; PLAYER_COUNT - 1],
    /// Set when the human clicks the Done Swapping button.
    human_done: bool,
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CardRendererPlugin)
            .insert_resource(GameState::new())
            .insert_resource(MatchState::new(PLAYER_COUNT, MATCH_TARGET))
            .insert_resource(DraftState::default())
            .insert_resource(SwapState::default())
            .insert_resource(PeekRevealTimer::default())
            .insert_resource(DealTimer(Timer::from_seconds(DEAL_INTERVAL, TimerMode::Repeating)))
            .insert_resource(AITimer(Timer::from_seconds(AI_TICK_NORMAL, TimerMode::Repeating)))
            .insert_resource(HoveredCard::default())
            .insert_resource(LastClick::default())
            .insert_resource(InvalidFeedbackTimer::default())
            .insert_resource(MusicMuted::default())
            .add_event::<InvalidCardClicked>()
            .add_systems(Startup, (setup_game, setup_music))
            .add_systems(Update, (
                update_game_state,
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
            ));
    }
}

/// Seats the human at index 0, then one Player per persona on MatchState.
/// Used on startup, on next-round restart, and on new-match restart so the
/// labels and personalities always line up with `match_state.personas`. Per-seat
/// modifiers are restored from `match_state.persistent_modifiers` so that buffs
/// carry over between rounds — with every consumable's `used_this_round` reset
/// for the new round.
fn add_match_players(game_state: &mut GameState, match_state: &MatchState) {
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

    let font = asset_server.load("fonts/NotoSans-Regular.ttf");
    let suit_font = asset_server.load("fonts/NotoSansSymbols2-Regular.ttf");
    game_state.prepare_dealing(&mut commands, font.clone(), suit_font);

    // Pile status text — world-space Text2d above the play pile
    commands.spawn((
        PileStatusText,
        Text2dBundle {
            text: Text::from_section(
                "",
                TextStyle { font: font.clone(), font_size: 16.0, color: Color::WHITE },
            ),
            transform: Transform::from_xyz(PLAY_PILE_X, CARD_HEIGHT / 2.0 + 24.0, 600.0),
            ..default()
        },
    ));

    // "Play Cards" button — absolute-positioned near the bottom centre of the screen
    commands.spawn((
        PlayButton,
        ButtonBundle {
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(12.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-64.0)), // centre the 128px button
                width: Val::Px(128.0),
                height: Val::Px(40.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            background_color: Color::srgb(0.25, 0.25, 0.25).into(),
            ..default()
        },
    )).with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            "Play Cards",
            TextStyle { font: font.clone(), font_size: 18.0, color: Color::WHITE },
        ));
    });

    // "Done Swapping" button — shares the play button's slot. Hidden by
    // default; update_swap_button_visibility toggles based on phase.
    commands.spawn((
        DoneSwapButton,
        ButtonBundle {
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(12.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-72.0)), // centre the 144px button
                width: Val::Px(144.0),
                height: Val::Px(40.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            background_color: Color::srgb(0.15, 0.55, 0.15).into(),
            ..default()
        },
    )).with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            "Done Swapping",
            TextStyle { font: font.clone(), font_size: 16.0, color: Color::WHITE },
        ));
    });

    spawn_score_hud(&mut commands, font);

    info!("Game setup complete! Ready to deal cards.");
}

/// Spawns the background music sink on startup. The OGG asset is optional —
/// if `assets/music/lofi_loop.ogg` is absent Bevy logs an asset-load warning
/// and the game runs silently. See scripts/download-music.sh.
fn setup_music(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        BackgroundMusic,
        AudioBundle {
            source: asset_server.load("music/lofi_loop.ogg"),
            settings: PlaybackSettings::LOOP.with_volume(Volume::new(MUSIC_VOLUME)),
        },
    ));
}

/// Ctrl+M toggles background-music mute. Bound under a modifier so the bare
/// M key continues to consume Mulligan during play without ambiguity.
fn toggle_music_mute(
    keys: Res<ButtonInput<KeyCode>>,
    mut muted: ResMut<MusicMuted>,
    sinks: Query<&AudioSink, With<BackgroundMusic>>,
) {
    let ctrl_held = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if !ctrl_held || !keys.just_pressed(KeyCode::KeyM) {
        return;
    }
    muted.0 = !muted.0;
    if let Ok(sink) = sinks.get_single() {
        sink.set_volume(if muted.0 { 0.0 } else { MUSIC_VOLUME });
    }
}

/// Spawns the top-right round/score widget. Persists across restarts; its text
/// is rewritten each frame by `update_score_hud`.
fn spawn_score_hud(commands: &mut Commands, font: Handle<Font>) {
    commands
        .spawn((
            ScoreHud,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(12.0),
                    right: Val::Px(12.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    min_width: Val::Px(170.0),
                    ..default()
                },
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.45).into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                ScoreHudText,
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font,
                        font_size: 14.0,
                        color: Color::srgba(1.0, 1.0, 1.0, 0.95),
                    },
                ),
            ));
        });
}

fn update_game_state(_game_state: Res<GameState>) {}

fn deal_cards_system(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
    mut deal_timer: ResMut<DealTimer>,
    cards: Query<&Card>,
) {
    if game_state.dealing_in_progress && deal_timer.0.tick(time.delta()).just_finished() {
        game_state.deal_next_card(&mut commands, &cards);
    }
}

fn update_card_face_up_state(
    game_state: Res<GameState>,
    hovered: Res<HoveredCard>,
    peek_timer: Res<PeekRevealTimer>,
    animating: Query<Entity, With<CardAnimation>>,
    time: Res<Time>,
    mut cards: Query<(Entity, &mut Card)>,
) {
    // The topmost pile card that has FINISHED animating — this one shows its text.
    // Using last() would switch text to the incoming card before it arrives visually.
    let top_visible = game_state.cards_in_play.iter().rev()
        .find(|&&e| !animating.contains(e))
        .copied();

    let peek_active = peek_timer.0 > 0.0;

    for (card_entity, mut card) in cards.iter_mut() {
        card.is_hovered = hovered.0 == Some(card_entity);
        card.is_selected = game_state.selected_cards.contains(&card_entity);
        if card.invalid_timer > 0.0 {
            card.invalid_timer = (card.invalid_timer - time.delta_seconds()).max(0.0);
        }

        let mut is_in_player_hand = false;

        for (player_index, player) in game_state.players.iter().enumerate() {
            if player.face_up_cards.contains(&card_entity) {
                is_in_player_hand = true;
                card.is_face_up = true;
                card.show_text = true;
                break;
            }
            if player.face_down_cards.contains(&card_entity) {
                is_in_player_hand = true;
                // Peek reveals the human's face-down cards for a few seconds.
                let reveal = peek_active && player_index == 0;
                card.is_face_up = reveal;
                card.show_text = reveal;
                break;
            }
            if player.hand.contains(&card_entity) {
                is_in_player_hand = true;
                let face_up = player_index == 0; // only human sees their hand
                card.is_face_up = face_up;
                card.show_text = face_up;
                break;
            }
        }

        if !is_in_player_hand {
            let in_play = game_state.cards_in_play.contains(&card_entity);
            card.is_face_up = in_play;
            // Show text only on the topmost card that has finished its animation
            card.show_text = in_play && Some(card_entity) == top_visible;
        }
    }
}

fn draw_first_card_system(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    cards: Query<&Card>,
) {
    if game_state.phase == GamePhase::Playing
        && game_state.current_card.is_none()
        && !game_state.draw_pile.is_empty()
    {
        if let Some(card_entity) = game_state.draw_pile.pop() {
            // Record the effective rank so the first player knows what to beat
            if let Ok(card) = cards.get(card_entity) {
                game_state.effective_rank = Some(card.rank);
            }
            game_state.current_card = Some(card_entity);
            game_state.cards_in_play.push(card_entity);

            commands.entity(card_entity).insert(CardAnimation {
                target_position: Vec3::new(PLAY_PILE_X, 0.0, 500.0),
                start_position: Vec3::new(0.0, 0.0, 400.0),
                progress: 0.0,
                speed: 2.0,
            });

            info!("First card drawn and placed on table");
        }
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn has_valid_play(game_state: &GameState, cards: &Query<&Card>, player_index: usize) -> bool {
    let sa = game_state.seven_active;
    let acp = game_state.any_card_playable;
    let effective_rank = game_state.effective_rank;
    let draw_pile_not_empty = !game_state.draw_pile.is_empty();
    let has_counter7 = game_state
        .players
        .get(player_index)
        .map(|p| p.has_buff(BuffKind::Counter7))
        .unwrap_or(false);

    if let Some(player) = game_state.players.get(player_index) {
        let source: &[Entity] = if draw_pile_not_empty || !player.hand.is_empty() {
            &player.hand
        } else if !player.face_up_cards.is_empty() {
            &player.face_up_cards
        } else {
            // Face-down phase: player must blind-flip; can't preempt with pickup.
            return !player.face_down_cards.is_empty();
        };
        for &card_entity in source {
            if let Ok(card) = cards.get(card_entity) {
                if can_play_card(card, effective_rank, sa, acp, has_counter7) {
                    return true;
                }
            }
        }
    }
    false
}

fn can_play_card(
    card: &Card,
    effective_rank: Option<Rank>,
    seven_active: bool,
    any_card_playable: bool,
    has_counter7: bool,
) -> bool {
    // 2, 3, and 10 are always playable
    if matches!(card.rank, Rank::Two | Rank::Three | Rank::Ten) {
        return true;
    }
    if any_card_playable {
        return true;
    }
    if seven_active && !has_counter7 {
        return (card.rank as u8) <= (Rank::Seven as u8);
    }
    if let Some(r) = effective_rank {
        (card.rank as u8) >= (r as u8)
    } else {
        true
    }
}

/// Per-player hand size. Big Hand drafted? You refill to 4.
fn target_hand_size(player: &crate::components::game::Player) -> usize {
    if player.has_buff(BuffKind::BigHand) { 4 } else { 3 }
}

fn pickup_cards_in_play(game_state: &mut GameState, player_index: usize) {
    let half_pickup = game_state
        .players
        .get(player_index)
        .map(|p| p.has_buff(BuffKind::HalfPickup))
        .unwrap_or(false);
    let pile_len = game_state.cards_in_play.len();
    let to_hand = if half_pickup {
        // Keep the most recent half (rounded up). Oldest cards are discarded.
        pile_len.div_ceil(2)
    } else {
        pile_len
    };
    let to_discard = pile_len - to_hand;

    let mut drained = std::mem::take(&mut game_state.cards_in_play).into_iter();
    for _ in 0..to_discard {
        if let Some(e) = drained.next() {
            game_state.discard_pile.push(e);
        }
    }
    if let Some(player) = game_state.players.get_mut(player_index) {
        for e in drained {
            player.hand.push(e);
        }
        if half_pickup {
            info!(
                "Player {} picked up {} (Half Pickup: {} discarded)",
                player_index, to_hand, to_discard
            );
        } else {
            info!("Player {} picked up {} cards", player_index, to_hand);
        }
    }
    game_state.current_card = None;
    game_state.effective_rank = None;
    game_state.seven_active = false;
    game_state.any_card_playable = false;
    game_state.selected_cards.clear();
}

// ── systems ───────────────────────────────────────────────────────────────────

fn update_hovered_card(
    mut hovered: ResMut<HoveredCard>,
    game_state: Res<GameState>,
    transforms: Query<&GlobalTransform>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
) {
    // Only track hover on the human player's turn
    if game_state.phase != GamePhase::Playing || game_state.current_player != 0 {
        hovered.0 = None;
        return;
    }

    let (camera, camera_transform) = camera_q.single();
    let window = windows.single();

    let Some(cursor_pos) = window.cursor_position() else {
        hovered.0 = None;
        return;
    };
    let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        hovered.0 = None;
        return;
    };

    let player = &game_state.players[0];
    let draw_pile_not_empty = !game_state.draw_pile.is_empty();
    let source: &[Entity] = if draw_pile_not_empty || !player.hand.is_empty() {
        &player.hand
    } else if !player.face_up_cards.is_empty() {
        &player.face_up_cards
    } else {
        &player.face_down_cards
    };

    // Find the topmost (last in slice = highest z) card under the cursor
    let mut found = None;
    for &entity in source {
        if let Ok(t) = transforms.get(entity) {
            let pos = t.translation().truncate();
            if world_pos.x >= pos.x - CARD_WIDTH / 2.0
                && world_pos.x <= pos.x + CARD_WIDTH / 2.0
                && world_pos.y >= pos.y - CARD_HEIGHT / 2.0
                && world_pos.y <= pos.y + CARD_HEIGHT / 2.0
            {
                found = Some(entity); // later entries overwrite — highest z wins
            }
        }
    }
    hovered.0 = found;
}

fn tick_last_click(mut last_click: ResMut<LastClick>, time: Res<Time>) {
    if last_click.entity.is_some() {
        last_click.age += time.delta_seconds();
        if last_click.age > DOUBLE_CLICK_WINDOW * 2.0 {
            last_click.entity = None;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_swap_input(
    windows: Query<&Window>,
    mut game_state: ResMut<GameState>,
    mut swap_state: ResMut<SwapState>,
    transforms: Query<&GlobalTransform>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    button_q: Query<&Interaction, With<DoneSwapButton>>,
) {
    if game_state.phase != GamePhase::Swap || swap_state.human_done {
        return;
    }
    if !mouse_button_input.just_pressed(MouseButton::Left) {
        return;
    }
    // The Done button overlaps the hand fan area; drop swap input when the
    // cursor is over the button so a Done click can't also swap a card.
    if button_q
        .iter()
        .any(|i| matches!(i, Interaction::Pressed | Interaction::Hovered))
    {
        return;
    }

    let (camera, camera_transform) = camera_q.single();
    let window = windows.single();
    let Some(cursor_pos) = window.cursor_position() else { return; };
    let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else { return; };

    let hand: Vec<Entity> = game_state.players[0].hand.clone();
    let face_up: Vec<Entity> = game_state.players[0].face_up_cards.clone();

    let hit_in = |source: &[Entity]| -> Option<Entity> {
        let mut found = None;
        for &e in source {
            if let Ok(t) = transforms.get(e) {
                let p = t.translation().truncate();
                if world_pos.x >= p.x - CARD_WIDTH / 2.0
                    && world_pos.x <= p.x + CARD_WIDTH / 2.0
                    && world_pos.y >= p.y - CARD_HEIGHT / 2.0
                    && world_pos.y <= p.y + CARD_HEIGHT / 2.0
                {
                    found = Some(e);
                }
            }
        }
        found
    };

    if let Some(hand_hit) = hit_in(&hand) {
        if swap_state.human_selected_hand == Some(hand_hit) {
            swap_state.human_selected_hand = None;
            game_state.selected_cards.retain(|&e| e != hand_hit);
        } else {
            game_state.selected_cards.clear();
            swap_state.human_selected_hand = Some(hand_hit);
            game_state.selected_cards.push(hand_hit);
        }
        return;
    }

    if let Some(fu_hit) = hit_in(&face_up) {
        let Some(hand_card) = swap_state.human_selected_hand.take() else { return; };
        if let Some(player) = game_state.players.get_mut(0) {
            if let (Some(h_pos), Some(f_pos)) = (
                player.hand.iter().position(|&e| e == hand_card),
                player.face_up_cards.iter().position(|&e| e == fu_hit),
            ) {
                player.hand[h_pos] = fu_hit;
                player.face_up_cards[f_pos] = hand_card;
            }
        }
        game_state.selected_cards.clear();
    }
}

fn handle_done_swap_button(
    game_state: Res<GameState>,
    mut swap_state: ResMut<SwapState>,
    interaction_q: Query<&Interaction, (Changed<Interaction>, With<DoneSwapButton>)>,
) {
    if game_state.phase != GamePhase::Swap {
        return;
    }
    for interaction in &interaction_q {
        if *interaction == Interaction::Pressed {
            swap_state.human_done = true;
        }
    }
}

/// Each AI greedily swaps any hand card whose rank exceeds a face-up card's
/// rank, picking the biggest gain each iteration until no improvement remains.
/// Runs once per round entry into Swap; the `ai_done` flags make subsequent
/// frames no-ops.
fn ai_swap_system(
    mut game_state: ResMut<GameState>,
    mut swap_state: ResMut<SwapState>,
    cards: Query<&Card>,
) {
    if game_state.phase != GamePhase::Swap {
        return;
    }
    let n_players = game_state.players.len();
    for ai_idx in 1..n_players {
        let slot = ai_idx - 1;
        if swap_state.ai_done.get(slot).copied().unwrap_or(true) {
            continue;
        }
        loop {
            let mut best: Option<(usize, usize, u8)> = None; // (hand_idx, fu_idx, gain)
            {
                let player = &game_state.players[ai_idx];
                for (h_idx, &h_e) in player.hand.iter().enumerate() {
                    let Ok(h_card) = cards.get(h_e) else { continue; };
                    for (f_idx, &f_e) in player.face_up_cards.iter().enumerate() {
                        let Ok(f_card) = cards.get(f_e) else { continue; };
                        let hr = h_card.rank as u8;
                        let fr = f_card.rank as u8;
                        if hr > fr {
                            let gain = hr - fr;
                            if best.map_or(true, |(_, _, g)| gain > g) {
                                best = Some((h_idx, f_idx, gain));
                            }
                        }
                    }
                }
            }
            let Some((h_idx, f_idx, _)) = best else { break; };
            let player = &mut game_state.players[ai_idx];
            let h_e = player.hand[h_idx];
            let f_e = player.face_up_cards[f_idx];
            player.hand[h_idx] = f_e;
            player.face_up_cards[f_idx] = h_e;
        }
        swap_state.ai_done[slot] = true;
    }
}

fn advance_swap_phase(
    mut game_state: ResMut<GameState>,
    mut swap_state: ResMut<SwapState>,
) {
    if game_state.phase != GamePhase::Swap {
        return;
    }
    let all_ai_done = swap_state.ai_done.iter().all(|&d| d);
    if swap_state.human_done && all_ai_done {
        game_state.phase = GamePhase::Drafting;
        game_state.selected_cards.clear();
        *swap_state = SwapState::default();
    }
}

fn update_swap_button_visibility(
    game_state: Res<GameState>,
    mut play_q: Query<&mut Style, (With<PlayButton>, Without<DoneSwapButton>)>,
    mut swap_q: Query<&mut Style, (With<DoneSwapButton>, Without<PlayButton>)>,
) {
    let in_swap = game_state.phase == GamePhase::Swap;
    for mut style in play_q.iter_mut() {
        style.display = if in_swap { Display::None } else { Display::Flex };
    }
    for mut style in swap_q.iter_mut() {
        style.display = if in_swap { Display::Flex } else { Display::None };
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_mouse_input(
    mut commands: Commands,
    windows: Query<&Window>,
    mut game_state: ResMut<GameState>,
    cards: Query<&Card>,
    transforms: Query<&GlobalTransform>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut invalid_ev: EventWriter<InvalidCardClicked>,
    mut last_click: ResMut<LastClick>,
) {
    if game_state.phase != GamePhase::Playing {
        return;
    }
    if !mouse_button_input.just_pressed(MouseButton::Left) {
        return;
    }

    let (camera, camera_transform) = camera_q.single();
    let window = windows.single();

    let Some(cursor_position) = window.cursor_position() else { return; };
    let Some(world_position) = camera.viewport_to_world_2d(camera_transform, cursor_position) else { return; };

    // Click play pile to pick up when required. While pickup is pending, no
    // other card interaction is allowed — the player must pick up before
    // flipping a face-down or staging anything else.
    if game_state.needs_to_pickup && game_state.current_player == 0 {
        let in_pile = world_position.x >= PLAY_PILE_X - CARD_WIDTH / 2.0 - 12.0
            && world_position.x <= PLAY_PILE_X + CARD_WIDTH / 2.0 + 12.0
            && world_position.y >= -CARD_HEIGHT / 2.0 - 12.0
            && world_position.y <= CARD_HEIGHT / 2.0 + 12.0;
        if in_pile {
            let current_player_index = game_state.current_player;
            pickup_cards_in_play(&mut game_state, current_player_index);
            game_state.needs_to_pickup = false;
            game_state.advance_to_next_active();
        }
        return;
    }

    // Only stage cards on the human player's turn
    if game_state.current_player != 0 { return; }

    let current_player_index = game_state.current_player;
    let player = &game_state.players[current_player_index];
    let draw_pile_not_empty = !game_state.draw_pile.is_empty();
    let hand_not_empty = !player.hand.is_empty();
    let face_up_not_empty = !player.face_up_cards.is_empty();

    let playing_from_face_down = !draw_pile_not_empty && !hand_not_empty && !face_up_not_empty;
    let playing_from_face_up = !draw_pile_not_empty && !hand_not_empty && face_up_not_empty;

    let mut cards_to_check: Vec<Entity> = Vec::new();
    if playing_from_face_down {
        cards_to_check.extend(player.face_down_cards.iter().copied());
    } else if playing_from_face_up {
        cards_to_check.extend(player.face_up_cards.iter().copied());
    } else {
        cards_to_check.extend(player.hand.iter().copied());
    }

    // Find the topmost card under the cursor (same strategy as update_hovered_card:
    // iterate all candidates and let later/higher-z entries overwrite earlier ones).
    let mut hit_entity: Option<Entity> = None;
    for &card_entity in &cards_to_check {
        if let Ok(transform) = transforms.get(card_entity) {
            let card_pos = transform.translation().truncate();
            if world_position.x >= card_pos.x - CARD_WIDTH / 2.0
                && world_position.x <= card_pos.x + CARD_WIDTH / 2.0
                && world_position.y >= card_pos.y - CARD_HEIGHT / 2.0
                && world_position.y <= card_pos.y + CARD_HEIGHT / 2.0
            {
                hit_entity = Some(card_entity); // later (higher-z) entries overwrite
            }
        }
    }

    let Some(card_entity) = hit_entity else { return; };

    // Face-down: blind immediate play. Validity is resolved by play_selection,
    // which routes an invalid play to a pickup instead of flashing red — the
    // player can't know what the card is before flipping it.
    if playing_from_face_down {
        play_selection(&mut commands, &mut game_state, &cards, &transforms, &[card_entity]);
        // Don't seed last_click — face-down already plays on a single click.
        last_click.entity = None;
        return;
    }

    let Ok(card) = cards.get(card_entity) else { return; };

    // Face-up endgame: any face-up may be staged. An invalid attempt at confirm
    // time pushes the staged cards onto the pile and triggers pickup, matching
    // physical Shed rules (you commit to a face-up; if it bricks, you eat it
    // along with the pile).
    if !playing_from_face_up {
        let has_counter7 = game_state.players[0].has_buff(BuffKind::Counter7);
        if !can_play_card(
            card,
            game_state.effective_rank,
            game_state.seven_active,
            game_state.any_card_playable,
            has_counter7,
        ) {
            invalid_ev.send(InvalidCardClicked(card_entity));
            // An invalid click still resets the double-click tracker so the
            // next click on a different card starts a fresh window.
            last_click.entity = None;
            return;
        }
    }

    // Double-click on the same card within the window plays it directly,
    // bypassing the staging step. Works for hand plays (re-validated above)
    // and face-up endgame plays (play_selection handles invalid via pickup).
    let is_double_click =
        last_click.entity == Some(card_entity) && last_click.age < DOUBLE_CLICK_WINDOW;
    if is_double_click {
        game_state.selected_cards.clear();
        play_selection(&mut commands, &mut game_state, &cards, &transforms, &[card_entity]);
        last_click.entity = None;
        return;
    }
    last_click.entity = Some(card_entity);
    last_click.age = 0.0;

    let card_rank = card.rank;
    if game_state.selected_cards.contains(&card_entity) {
        // Deselect
        game_state.selected_cards.retain(|&e| e != card_entity);
    } else {
        let sel_rank = game_state.selected_cards.first()
            .and_then(|&e| cards.get(e).ok())
            .map(|c| c.rank);
        if sel_rank.is_none() || sel_rank == Some(card_rank) {
            if game_state.selected_cards.len() < 4 {
                game_state.selected_cards.push(card_entity);
            }
        } else {
            // Different rank — start fresh selection
            game_state.selected_cards = vec![card_entity];
        }
    }
}

/// Applies red flash to the clicked card and highlights the pile status text.
fn handle_invalid_card_event(
    mut events: EventReader<InvalidCardClicked>,
    mut cards: Query<&mut Card>,
    mut feedback: ResMut<InvalidFeedbackTimer>,
) {
    for InvalidCardClicked(entity) in events.read() {
        if let Ok(mut card) = cards.get_mut(*entity) {
            card.invalid_timer = 0.5;
        }
        feedback.0 = 2.0;
    }
}

/// Confirms (Enter) or cancels (Escape) the staged multi-card play.
fn confirm_play_system(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    cards: Query<&Card>,
    transforms: Query<&GlobalTransform>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if game_state.phase != GamePhase::Playing || game_state.current_player != 0 { return; }

    if keyboard.just_pressed(KeyCode::Escape) {
        game_state.selected_cards.clear();
        return;
    }
    if !keyboard.just_pressed(KeyCode::Enter) { return; }
    if game_state.selected_cards.is_empty() { return; }

    let selection = std::mem::take(&mut game_state.selected_cards);
    play_selection(&mut commands, &mut game_state, &cards, &transforms, &selection);
}

/// Plays all selected cards at once. Handles 4-of-a-kind burn and all rank effects.
///
/// May be invoked with a card that turns out to be illegal in two cases:
/// blind face-down flips, and face-up endgame plays where the click handler
/// relaxes validation. Both route an invalid attempt to pickup rather than
/// flashing red — the cards still ride the animation onto the pile so they
/// travel back to the player's hand with the rest of the stack.
fn play_selection(
    commands: &mut Commands,
    game_state: &mut GameState,
    cards: &Query<&Card>,
    transforms: &Query<&GlobalTransform>,
    selection: &[Entity],
) {
    if selection.is_empty() { return; }

    let rank = match cards.get(selection[0]) {
        Ok(c) => c.rank,
        Err(_) => return,
    };
    let playing_player = game_state.current_player;

    // Push all to play pile with animations
    for &entity in selection {
        game_state.cards_in_play.push(entity);
        game_state.current_card = Some(entity);
        let target_z = 500.0 + game_state.cards_in_play.len() as f32 * Z_INDEX_STEP;
        let start_pos = transforms.get(entity).map(|t| t.translation()).unwrap_or(Vec3::ZERO);
        commands.entity(entity).insert(CardAnimation {
            target_position: Vec3::new(PLAY_PILE_X, 0.0, target_z),
            start_position: start_pos,
            progress: 0.0,
            speed: 3.0,
        });
        // Remove from player's collection
        if let Some(player) = game_state.players.get_mut(playing_player) {
            if let Some(pos) = player.hand.iter().position(|&e| e == entity) {
                player.hand.remove(pos);
            } else if let Some(pos) = player.face_up_cards.iter().position(|&e| e == entity) {
                player.face_up_cards.remove(pos);
            } else if let Some(pos) = player.face_down_cards.iter().position(|&e| e == entity) {
                player.face_down_cards.remove(pos);
            }
        }
    }

    // Validate against the pile state captured before this play. If the play
    // is illegal (blind face-down brick, or staged face-up that bricks), the
    // cards remain on the pile and the player picks the stack up. Skip refill,
    // burn, rank effects, and turn advance — the pickup flow takes over.
    let has_counter7 = game_state.players[playing_player].has_buff(BuffKind::Counter7);
    let first_card_valid = cards.get(selection[0]).map(|c| {
        can_play_card(
            c,
            game_state.effective_rank,
            game_state.seven_active,
            game_state.any_card_playable,
            has_counter7,
        )
    }).unwrap_or(false);

    if !first_card_valid {
        game_state.needs_to_pickup = true;
        info!("Player {} bricked the play — pickup pending", playing_player);
        return;
    }

    // Human refill is deferred so the new card animates in after the played
    // card lands; AI refills inline (no animation needed for an unseen hand).
    let refill_target = target_hand_size(&game_state.players[playing_player]);
    if playing_player == 0 {
        game_state.pending_refill = true;
        game_state.refill_timer = 0.45;
    } else {
        while game_state.players[playing_player].hand.len() < refill_target
            && !game_state.draw_pile.is_empty()
        {
            if let Some(new_card) = game_state.draw_pile.pop() {
                game_state.players[playing_player].hand.push(new_card);
            }
        }
    }

    // Burn check: Ten always burns. With Hot Hand the top-3 threshold replaces
    // the standard top-4. Wild Twos / Wild Kings extend the burn list to extra
    // ranks for the playing player only.
    let hot_hand = game_state.players[playing_player].has_buff(BuffKind::HotHand);
    let wild_twos = game_state.players[playing_player].has_buff(BuffKind::WildTwos);
    let wild_kings = game_state.players[playing_player].has_buff(BuffKind::WildKings);
    let threshold = if hot_hand { 3 } else { 4 };
    let pile_len = game_state.cards_in_play.len();
    let same_top_burn = pile_len >= threshold && {
        let top = &game_state.cards_in_play[pile_len - threshold..];
        top.iter().all(|&e| cards.get(e).map(|c| c.rank == rank).unwrap_or(false))
    };
    let burn = rank == Rank::Ten
        || same_top_burn
        || (rank == Rank::Two && wild_twos)
        || (rank == Rank::King && wild_kings);

    if burn {
        game_state.seven_active = false;
        game_state.any_card_playable = false;
        game_state.effective_rank = None;
        game_state.discard_pile.extend(game_state.cards_in_play.drain(..));
        game_state.current_card = None;
        info!("{:?} burned the pile (4-of-a-kind or 10), player {} goes again", rank, playing_player);
        if game_state.check_and_eliminate(playing_player) {
            info!("Player {} finished {}", playing_player, game_state.finish_order.len());
            game_state.advance_to_next_active();
        }
        return;
    }

    match rank {
        Rank::Three => { /* transparent — effective_rank and flags unchanged */ }
        Rank::Two => {
            game_state.seven_active = false;
            game_state.any_card_playable = true;
            game_state.effective_rank = None;
        }
        Rank::Seven => {
            game_state.seven_active = true;
            game_state.any_card_playable = false;
            game_state.effective_rank = Some(Rank::Seven);
        }
        _ => {
            game_state.seven_active = false;
            game_state.any_card_playable = false;
            game_state.effective_rank = Some(rank);
        }
    }

    if game_state.check_and_eliminate(playing_player) {
        info!("Player {} finished {}", playing_player, game_state.finish_order.len());
    }
    game_state.advance_to_next_active();
}


fn check_valid_plays_system(
    mut game_state: ResMut<GameState>,
    cards: Query<&Card>,
) {
    if game_state.phase != GamePhase::Playing || game_state.current_card.is_none() {
        return;
    }
    let current_player_index = game_state.current_player;
    if !game_state.needs_to_pickup && !has_valid_play(&game_state, &cards, current_player_index) {
        game_state.needs_to_pickup = true;
        info!("Player {} needs to pick up cards", current_player_index);
    }
}

fn handle_card_pickup_system(
    mut game_state: ResMut<GameState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if !game_state.needs_to_pickup || game_state.current_player != 0 {
        return;
    }
    if keyboard.just_pressed(KeyCode::Space) {
        let current_player_index = game_state.current_player;
        pickup_cards_in_play(&mut game_state, current_player_index); // also clears selected_cards
        game_state.needs_to_pickup = false;
        game_state.advance_to_next_active();
        info!("Player picked up cards");
    }
}

/// Dispatches the active AI's move through `ai::choose_play`, then routes the
/// selected cards through `play_selection` (same path as human plays). Each AI
/// gets one tick per turn; if it has no legal play it marks itself for pickup
/// and the pickup branch above handles it on the next tick.
fn ai_player_system(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    cards: Query<&Card>,
    transforms: Query<&GlobalTransform>,
    time: Res<Time>,
    mut ai_timer: ResMut<AITimer>,
) {
    if game_state.phase != GamePhase::Playing || game_state.current_player == 0 {
        return;
    }
    // Keep the AI tick rate aligned with spectate mode: snappy after the human
    // is out, normal otherwise. Re-checked every frame so restarts reset for free.
    let want_secs = if game_state.spectate_mode { AI_TICK_SPECTATE } else { AI_TICK_NORMAL };
    if (ai_timer.0.duration().as_secs_f32() - want_secs).abs() > 0.01 {
        ai_timer.0.set_duration(Duration::from_secs_f32(want_secs));
    }
    if !ai_timer.0.tick(time.delta()).just_finished() {
        return;
    }

    if game_state.needs_to_pickup {
        let idx = game_state.current_player;
        pickup_cards_in_play(&mut game_state, idx);
        game_state.needs_to_pickup = false;
        game_state.advance_to_next_active();
        info!("AI {} picked up cards", idx);
        return;
    }

    let effective_rank = game_state.effective_rank;
    let sa = game_state.seven_active;
    let acp = game_state.any_card_playable;
    let current_idx = game_state.current_player;
    let draw_pile_not_empty = !game_state.draw_pile.is_empty();

    // Pick the active source pile (hand → face_up → face_down) using the same
    // priority human play uses.
    let player = &game_state.players[current_idx];
    let (source, from_face_down): (Vec<Entity>, bool) =
        if draw_pile_not_empty || !player.hand.is_empty() {
            (player.hand.clone(), false)
        } else if !player.face_up_cards.is_empty() {
            (player.face_up_cards.clone(), false)
        } else {
            (player.face_down_cards.clone(), true)
        };

    // Filter to legal plays; personality logic chooses among these. Face-down
    // candidates aren't filtered — the AI flips blind too, and play_selection
    // routes a brick to pickup.
    let has_counter7 = game_state.players[current_idx].has_buff(BuffKind::Counter7);
    let candidates: Vec<Entity> = if from_face_down {
        source
    } else {
        source
            .into_iter()
            .filter(|e| {
                cards
                    .get(*e)
                    .map(|c| can_play_card(c, effective_rank, sa, acp, has_counter7))
                    .unwrap_or(false)
            })
            .collect()
    };

    let personality = game_state.players[current_idx].personality;
    let selection = crate::ai::choose_play(personality, &candidates, &cards, from_face_down);

    if selection.is_empty() {
        if !game_state.needs_to_pickup {
            game_state.needs_to_pickup = true;
            info!("AI {} ({:?}) needs to pick up cards", current_idx, personality);
        }
    } else {
        info!(
            "AI {} ({:?}) playing {} card(s)",
            current_idx,
            personality,
            selection.len()
        );
        play_selection(&mut commands, &mut game_state, &cards, &transforms, &selection);
    }
}

/// Draws replacement cards into the human's hand with animation after play completes.
fn draw_refill_system(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
    windows: Query<&Window>,
) {
    if !game_state.pending_refill || game_state.phase != GamePhase::Playing { return; }

    // Count down the delay
    if game_state.refill_timer > 0.0 {
        game_state.refill_timer = (game_state.refill_timer - time.delta_seconds()).max(0.0);
        return;
    }

    let window_height = windows.single().height();
    let hand_base_y = -window_height / 2.0 + CARD_HEIGHT / 2.0;
    let refill_target = target_hand_size(&game_state.players[0]);

    while game_state.players[0].hand.len() < refill_target && !game_state.draw_pile.is_empty() {
        let new_card = game_state.draw_pile.pop().unwrap();
        let hand_idx = game_state.players[0].hand.len();
        game_state.players[0].hand.push(new_card);

        // Approximate target at the hand fan centre — layout_cards snaps to exact pos
        // once the animation finishes (1-frame, imperceptible).
        let target_z = 200.0 + hand_idx as f32 * Z_INDEX_STEP;
        commands.entity(new_card).insert(CardAnimation {
            target_position: Vec3::new(0.0, hand_base_y, target_z),
            start_position: Vec3::new(0.0, 0.0, 390.0),
            progress: 0.0,
            speed: 2.5,
        });
    }

    game_state.pending_refill = false;
}

fn handle_play_button(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    cards: Query<&Card>,
    transforms: Query<&GlobalTransform>,
    interaction_q: Query<&Interaction, (Changed<Interaction>, With<PlayButton>)>,
) {
    for interaction in &interaction_q {
        if *interaction == Interaction::Pressed
            && game_state.phase == GamePhase::Playing
            && game_state.current_player == 0
            && !game_state.selected_cards.is_empty()
        {
            let selection = std::mem::take(&mut game_state.selected_cards);
            play_selection(&mut commands, &mut game_state, &cards, &transforms, &selection);
        }
    }
}

fn update_play_button_style(
    game_state: Res<GameState>,
    mut button_q: Query<&mut BackgroundColor, With<PlayButton>>,
) {
    let active = game_state.phase == GamePhase::Playing
        && game_state.current_player == 0
        && !game_state.selected_cards.is_empty();
    for mut bg in button_q.iter_mut() {
        *bg = if active {
            Color::srgb(0.15, 0.55, 0.15).into()
        } else {
            Color::srgb(0.25, 0.25, 0.25).into()
        };
    }
}

fn update_pile_status_text(
    game_state: Res<GameState>,
    time: Res<Time>,
    mut feedback: ResMut<InvalidFeedbackTimer>,
    mut text_q: Query<&mut Text, With<PileStatusText>>,
) {
    let Ok(mut text) = text_q.get_single_mut() else { return; };

    if feedback.0 > 0.0 {
        feedback.0 = (feedback.0 - time.delta_seconds()).max(0.0);
    }

    let msg = if game_state.phase != GamePhase::Playing {
        String::new()
    } else if game_state.cards_in_play.is_empty() {
        "Play anything".to_string()
    } else if game_state.any_card_playable {
        "Play anything".to_string()
    } else if game_state.seven_active {
        "Play 7 or lower".to_string()
    } else if let Some(rank) = game_state.effective_rank {
        format!("Play {} or higher", rank)
    } else {
        "Play anything".to_string()
    };

    text.sections[0].value = msg;
    text.sections[0].style.color = if feedback.0 > 0.0 {
        Color::srgb(1.0, 0.5, 0.0) // orange highlight on invalid play
    } else {
        Color::WHITE
    };
}

fn ordinal(n: usize) -> String {
    match n {
        1 => "1st".into(),
        2 => "2nd".into(),
        3 => "3rd".into(),
        _ => format!("{}th", n),
    }
}

fn player_display_name(game_state: &GameState, player_idx: usize) -> String {
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
fn update_score_hud(
    match_state: Res<MatchState>,
    game_state: Res<GameState>,
    mut text_q: Query<&mut Text, With<ScoreHudText>>,
) {
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
    text.sections[0].value = format!("{}{}", header, body);
}

fn game_over_screen_system(
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

// ── draft systems ─────────────────────────────────────────────────────────────

/// One-time per round: populate the draft pools as soon as phase enters
/// Drafting. Idempotent — the `pools.is_empty()` guard keeps it from re-running
/// every frame while the human is reading their options.
fn setup_draft_system(
    mut draft_state: ResMut<DraftState>,
    game_state: Res<GameState>,
    match_state: Res<MatchState>,
) {
    if game_state.phase != GamePhase::Drafting {
        return;
    }
    if !draft_state.pools.is_empty() {
        return;
    }
    draft_state.reset(game_state.players.len());
    for seat in 0..game_state.players.len() {
        // Previous round's Shed gets a bigger pool; everyone else gets 3.
        let size = if match_state.previous_shed == Some(seat) { 5 } else { 3 };
        draft_state.pools[seat] = roll_pool(&game_state.players[seat].modifiers, size);
    }
}

/// Pick `size` distinct BuffKinds at random, excluding kinds the player already
/// owns. Falls back gracefully (returns fewer kinds) once the catalogue is
/// exhausted — rare in a 5-round match with 8 buffs.
fn roll_pool(owned: &[ActiveBuff], size: usize) -> Vec<BuffKind> {
    let mut available: Vec<BuffKind> = BuffKind::ALL
        .iter()
        .copied()
        .filter(|k| !owned.iter().any(|b| b.kind == *k))
        .collect();
    // Fisher-Yates using the same rand source the rest of the project uses.
    for i in (1..available.len()).rev() {
        let j = (rand::random::<f32>() * (i + 1) as f32) as usize;
        if j <= i {
            available.swap(i, j);
        }
    }
    available.into_iter().take(size).collect()
}

/// AIs pick instantly and randomly from their pool. Personality-aware picks
/// could be a follow-up.
fn ai_draft_system(
    mut draft_state: ResMut<DraftState>,
    game_state: Res<GameState>,
) {
    if game_state.phase != GamePhase::Drafting {
        return;
    }
    if draft_state.picks.is_empty() {
        return; // setup hasn't run yet
    }
    for seat in 1..game_state.players.len() {
        if draft_state.picks[seat].is_some() {
            continue;
        }
        let pool = &draft_state.pools[seat];
        if pool.is_empty() {
            // Player has every buff already — skip silently.
            draft_state.picks[seat] = Some(BuffKind::Mulligan);
            continue;
        }
        let idx = (rand::random::<f32>() * pool.len() as f32) as usize;
        let pick = pool[idx.min(pool.len() - 1)];
        draft_state.picks[seat] = Some(pick);
        info!(
            "AI seat {} ({:?}) picked buff: {}",
            seat,
            game_state.players[seat].personality,
            pick.display_name()
        );
    }
}

/// Spawn the full-screen draft overlay when entering Drafting. Stays up until
/// `apply_picks_system` despawns it (after every seat has chosen). The overlay
/// only shows the human's options; AI picks happen invisibly in the background.
fn draft_screen_system(
    mut commands: Commands,
    game_state: Res<GameState>,
    match_state: Res<MatchState>,
    draft_state: Res<DraftState>,
    screen_q: Query<Entity, With<DraftScreen>>,
    asset_server: Res<AssetServer>,
) {
    if game_state.phase != GamePhase::Drafting {
        return;
    }
    if !screen_q.is_empty() {
        return;
    }
    if draft_state.pools.is_empty() {
        return; // setup hasn't run yet
    }
    let human_pool = draft_state.pools.first().cloned().unwrap_or_default();
    if human_pool.is_empty() {
        return; // nothing to choose — apply_picks will fill it automatically next frame
    }

    let font = asset_server.load("fonts/NotoSans-Regular.ttf");
    let header = if human_pool.len() >= 5 {
        format!(
            "Round {} · Shed bonus — pick 1 of {}",
            match_state.round,
            human_pool.len()
        )
    } else {
        format!("Round {} · Pick a perk", match_state.round)
    };

    commands
        .spawn((
            DraftScreen,
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
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.7).into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                header,
                TextStyle {
                    font: font.clone(),
                    font_size: 36.0,
                    color: Color::WHITE,
                },
            ));
            parent.spawn(TextBundle {
                text: Text::from_section(
                    "Click a perk to add it to your run",
                    TextStyle {
                        font: font.clone(),
                        font_size: 14.0,
                        color: Color::srgba(1.0, 1.0, 1.0, 0.6),
                    },
                ),
                style: Style {
                    margin: UiRect::bottom(Val::Px(8.0)),
                    ..default()
                },
                ..default()
            });

            for &kind in &human_pool {
                parent
                    .spawn((
                        DraftOption(kind),
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(460.0),
                                padding: UiRect::all(Val::Px(12.0)),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(4.0),
                                align_items: AlignItems::FlexStart,
                                ..default()
                            },
                            background_color: Color::srgba(0.15, 0.15, 0.18, 0.95).into(),
                            ..default()
                        },
                    ))
                    .with_children(|row| {
                        let name = if kind.is_consumable() {
                            format!("{}  (consumable)", kind.display_name())
                        } else {
                            kind.display_name().to_string()
                        };
                        row.spawn(TextBundle::from_section(
                            name,
                            TextStyle {
                                font: font.clone(),
                                font_size: 20.0,
                                color: Color::srgb(1.0, 0.9, 0.4),
                            },
                        ));
                        row.spawn(TextBundle::from_section(
                            kind.description(),
                            TextStyle {
                                font: font.clone(),
                                font_size: 14.0,
                                color: Color::srgba(1.0, 1.0, 1.0, 0.85),
                            },
                        ));
                    });
            }
        });
}

fn handle_draft_click(
    mut draft_state: ResMut<DraftState>,
    game_state: Res<GameState>,
    mut interaction_q: Query<
        (&Interaction, &DraftOption, &mut BackgroundColor),
        Changed<Interaction>,
    >,
) {
    if game_state.phase != GamePhase::Drafting {
        return;
    }
    let already_picked = draft_state
        .picks
        .first()
        .copied()
        .flatten()
        .is_some();
    for (interaction, option, mut bg) in &mut interaction_q {
        match *interaction {
            Interaction::Pressed => {
                if !already_picked && !draft_state.picks.is_empty() {
                    draft_state.picks[0] = Some(option.0);
                    info!("You picked buff: {}", option.0.display_name());
                }
                *bg = Color::srgba(0.30, 0.55, 0.30, 0.98).into();
            }
            Interaction::Hovered => *bg = Color::srgba(0.25, 0.25, 0.30, 0.98).into(),
            Interaction::None => *bg = Color::srgba(0.15, 0.15, 0.18, 0.95).into(),
        }
    }
}

/// Finalize: push each seat's pick into Player.modifiers, snapshot to MatchState
/// so the buff survives the round-end teardown, despawn the overlay, and flip
/// the phase to Playing so the rest of the game wakes up.
fn apply_picks_system(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    mut match_state: ResMut<MatchState>,
    mut draft_state: ResMut<DraftState>,
    screen_q: Query<Entity, With<DraftScreen>>,
) {
    if game_state.phase != GamePhase::Drafting {
        return;
    }
    if !draft_state.all_picked() {
        return;
    }
    for seat in 0..game_state.players.len() {
        if let Some(kind) = draft_state.picks[seat] {
            // Skip duplicates so consumables can't be double-charged. The draft
            // pool already filters owned kinds, but the AI fallback path can
            // still pick a duplicate if the catalogue is exhausted.
            if !game_state.players[seat].has_buff(kind) {
                game_state.players[seat].modifiers.push(ActiveBuff {
                    kind,
                    used_this_round: false,
                });
            }
        }
    }
    // Snapshot for next-round carry-over.
    for seat in 0..game_state.players.len() {
        if seat < match_state.persistent_modifiers.len() {
            match_state.persistent_modifiers[seat] = game_state.players[seat].modifiers.clone();
        }
    }
    for e in screen_q.iter() {
        commands.entity(e).despawn_recursive();
    }
    draft_state.pools.clear();
    draft_state.picks.clear();
    game_state.phase = GamePhase::Playing;
    info!("Draft complete — entering Playing");
}

// ── consumable activation ─────────────────────────────────────────────────────

fn handle_mulligan_key(
    mut game_state: ResMut<GameState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if game_state.phase != GamePhase::Playing {
        return;
    }
    if game_state.current_player != 0 {
        return;
    }
    if !keyboard.just_pressed(KeyCode::KeyM) {
        return;
    }
    let player = &mut game_state.players[0];
    if !player.try_consume(BuffKind::Mulligan) {
        return;
    }
    std::mem::swap(&mut player.hand, &mut player.face_up_cards);
    info!("Mulligan used: hand <-> face-up swapped");
}

fn handle_peek_key(
    mut peek_timer: ResMut<PeekRevealTimer>,
    mut game_state: ResMut<GameState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if game_state.phase != GamePhase::Playing {
        return;
    }
    if !keyboard.just_pressed(KeyCode::KeyP) {
        return;
    }
    let player = &mut game_state.players[0];
    if !player.try_consume(BuffKind::Peek) {
        return;
    }
    peek_timer.0 = 3.0;
    info!("Peek used: revealing face-down cards for 3s");
}

fn tick_peek_timer(mut peek_timer: ResMut<PeekRevealTimer>, time: Res<Time>) {
    if peek_timer.0 > 0.0 {
        peek_timer.0 = (peek_timer.0 - time.delta_seconds()).max(0.0);
    }
}

#[allow(clippy::too_many_arguments)]
fn restart_game_system(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    mut match_state: ResMut<MatchState>,
    mut swap_state: ResMut<SwapState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    card_q: Query<Entity, With<Card>>,
    screen_q: Query<Entity, With<GameOverScreen>>,
    status_q: Query<Entity, With<PileStatusText>>,
    asset_server: Res<AssetServer>,
) {
    if game_state.phase != GamePhase::GameOver { return; }
    if keyboard.get_just_pressed().next().is_none() { return; }

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
