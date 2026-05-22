use bevy::prelude::*;
use crate::components::game::{GameState, GamePhase};
use crate::components::card::{Card, Rank};
use crate::rendering::card_constants::{CARD_HEIGHT, CARD_WIDTH, PLAY_PILE_X, Z_INDEX_STEP};
use crate::rendering::card_renderer::{CardRendererPlugin, CardAnimation};


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

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CardRendererPlugin)
            .insert_resource(GameState::new())
            .insert_resource(DealTimer(Timer::from_seconds(0.5, TimerMode::Repeating)))
            .insert_resource(AITimer(Timer::from_seconds(1.5, TimerMode::Repeating)))
            .insert_resource(HoveredCard::default())
            .insert_resource(InvalidFeedbackTimer::default())
            .add_event::<InvalidCardClicked>()
            .add_systems(Startup, setup_game)
            .add_systems(Update, (
                update_game_state,
                update_hovered_card,
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
                game_over_screen_system,
                restart_game_system,
            ));
    }
}

fn setup_game(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    asset_server: Res<AssetServer>,
) {
    game_state.add_player("Player".to_string());
    game_state.add_player("AI 1".to_string());
    game_state.add_player("AI 2".to_string());
    game_state.add_player("AI 3".to_string());

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
            TextStyle { font, font_size: 18.0, color: Color::WHITE },
        ));
    });

    info!("Game setup complete! Ready to deal cards.");
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
    animating: Query<Entity, With<CardAnimation>>,
    time: Res<Time>,
    mut cards: Query<(Entity, &mut Card)>,
) {
    // The topmost pile card that has FINISHED animating — this one shows its text.
    // Using last() would switch text to the incoming card before it arrives visually.
    let top_visible = game_state.cards_in_play.iter().rev()
        .find(|&&e| !animating.contains(e))
        .copied();

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
                card.is_face_up = false;
                card.show_text = false;
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

    if let Some(player) = game_state.players.get(player_index) {
        let sources: &[&[Entity]] = if draw_pile_not_empty || !player.hand.is_empty() {
            &[&player.hand]
        } else if !player.face_up_cards.is_empty() {
            &[&player.face_up_cards]
        } else {
            &[&player.face_down_cards]
        };
        for &source in sources {
            for &card_entity in source {
                if let Ok(card) = cards.get(card_entity) {
                    if can_play_card(card, effective_rank, sa, acp) {
                        return true;
                    }
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
) -> bool {
    // 2, 3, and 10 are always playable
    if matches!(card.rank, Rank::Two | Rank::Three | Rank::Ten) {
        return true;
    }
    if any_card_playable {
        return true;
    }
    if seven_active {
        return (card.rank as u8) <= (Rank::Seven as u8);
    }
    if let Some(r) = effective_rank {
        (card.rank as u8) >= (r as u8)
    } else {
        true
    }
}


fn pickup_cards_in_play(game_state: &mut GameState, player_index: usize) {
    if let Some(player) = game_state.players.get_mut(player_index) {
        for &card_entity in &game_state.cards_in_play {
            player.hand.push(card_entity);
        }
        info!("Player {} picked up {} cards", player_index, game_state.cards_in_play.len());
    }
    game_state.cards_in_play.clear();
    game_state.current_card = None;
    game_state.effective_rank = None;
    game_state.seven_active = false;
    game_state.any_card_playable = false;
    game_state.selected_cards.clear();
}

fn play_card(
    commands: &mut Commands,
    game_state: &mut GameState,
    card_entity: Entity,
    _hand_index: usize,
    rank: Rank,
    start_pos: Vec3,
) {
    game_state.current_card = Some(card_entity);
    game_state.cards_in_play.push(card_entity);

    let target_z = 500.0 + game_state.cards_in_play.len() as f32 * Z_INDEX_STEP;
    commands.entity(card_entity).insert(CardAnimation {
        target_position: Vec3::new(PLAY_PILE_X, 0.0, target_z),
        start_position: start_pos,
        progress: 0.0,
        speed: 3.0,
    });

    let playing_player = game_state.current_player;

    // Remove from player's collection and refill hand
    if let Some(player) = game_state.players.get_mut(playing_player) {
        if let Some(pos) = player.face_up_cards.iter().position(|&e| e == card_entity) {
            player.face_up_cards.remove(pos);
        } else if let Some(pos) = player.face_down_cards.iter().position(|&e| e == card_entity) {
            player.face_down_cards.remove(pos);
        } else if let Some(pos) = player.hand.iter().position(|&e| e == card_entity) {
            player.hand.remove(pos);
        }
        while player.hand.len() < 3 && !game_state.draw_pile.is_empty() {
            if let Some(new_card) = game_state.draw_pile.pop() {
                player.hand.push(new_card);
            }
        }
    }

    match rank {
        Rank::Three => {
            // Transparent — effective_rank and special flags unchanged
        }
        Rank::Two => {
            game_state.seven_active = false;
            game_state.any_card_playable = true;
            game_state.effective_rank = None;
            info!("2 played — any card valid next");
        }
        Rank::Seven => {
            game_state.seven_active = true;
            game_state.any_card_playable = false;
            game_state.effective_rank = Some(Rank::Seven);
            info!("7 played — next must play ≤ 7");
        }
        Rank::Ten => {
            game_state.seven_active = false;
            game_state.any_card_playable = false;
            game_state.effective_rank = None;
            // Burn the pile; same player goes again
            game_state.discard_pile.extend(game_state.cards_in_play.drain(..));
            game_state.current_card = None;
            info!("10 played — pile burned, player {} goes again", playing_player);
            let player = &game_state.players[playing_player];
            if player.hand.is_empty() && player.face_up_cards.is_empty() && player.face_down_cards.is_empty() {
                game_state.phase = GamePhase::GameOver;
                game_state.winner = Some(playing_player);
                info!("Player {} wins!", playing_player);
            }
            return; // same player, no turn advance
        }
        _ => {
            game_state.seven_active = false;
            game_state.any_card_playable = false;
            game_state.effective_rank = Some(rank);
        }
    }

    // Win check
    let player = &game_state.players[playing_player];
    if player.hand.is_empty() && player.face_up_cards.is_empty() && player.face_down_cards.is_empty() {
        game_state.phase = GamePhase::GameOver;
        game_state.winner = Some(playing_player);
        info!("Player {} wins!", playing_player);
        return;
    }

    game_state.current_player = (playing_player + 1) % game_state.players.len();
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

fn handle_mouse_input(
    windows: Query<&Window>,
    mut game_state: ResMut<GameState>,
    cards: Query<&Card>,
    transforms: Query<&GlobalTransform>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut invalid_ev: EventWriter<InvalidCardClicked>,
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

    // Click play pile to pick up when required
    if game_state.needs_to_pickup && game_state.current_player == 0 {
        let in_pile = world_position.x >= PLAY_PILE_X - CARD_WIDTH / 2.0 - 12.0
            && world_position.x <= PLAY_PILE_X + CARD_WIDTH / 2.0 + 12.0
            && world_position.y >= -CARD_HEIGHT / 2.0 - 12.0
            && world_position.y <= CARD_HEIGHT / 2.0 + 12.0;
        if in_pile {
            let current_player_index = game_state.current_player;
            pickup_cards_in_play(&mut game_state, current_player_index);
            game_state.needs_to_pickup = false;
            game_state.current_player = (current_player_index + 1) % game_state.players.len();
            return;
        }
    }

    // Only stage cards on the human player's turn
    if game_state.current_player != 0 { return; }

    let current_player_index = game_state.current_player;
    let player = &game_state.players[current_player_index];
    let draw_pile_not_empty = !game_state.draw_pile.is_empty();
    let hand_not_empty = !player.hand.is_empty();
    let face_up_not_empty = !player.face_up_cards.is_empty();

    let mut cards_to_check: Vec<Entity> = Vec::new();
    if draw_pile_not_empty || hand_not_empty {
        cards_to_check.extend(player.hand.iter().copied());
    } else if face_up_not_empty {
        cards_to_check.extend(player.face_up_cards.iter().copied());
    } else {
        cards_to_check.extend(player.face_down_cards.iter().copied());
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

    if let Some(card_entity) = hit_entity {
        if let Ok(card) = cards.get(card_entity) {
            if !can_play_card(card, game_state.effective_rank, game_state.seven_active, game_state.any_card_playable) {
                // Give visual feedback that this card can't be played
                invalid_ev.send(InvalidCardClicked(card_entity));
                return;
            }
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

    // Human hand refill is deferred — draw_refill_system animates new cards after the
    // play animation completes. (AI uses play_card which refills immediately.)
    game_state.pending_refill = true;
    game_state.refill_timer = 0.45; // let the played card(s) reach the pile first

    // 4-of-a-kind check: did this play complete 4 consecutive same-rank cards at the top?
    let pile_len = game_state.cards_in_play.len();
    let top4_burn = pile_len >= 4 && {
        let top4 = &game_state.cards_in_play[pile_len - 4..];
        top4.iter().all(|&e| cards.get(e).map(|c| c.rank == rank).unwrap_or(false))
    };

    let burn = rank == Rank::Ten || top4_burn;

    if burn {
        game_state.seven_active = false;
        game_state.any_card_playable = false;
        game_state.effective_rank = None;
        game_state.discard_pile.extend(game_state.cards_in_play.drain(..));
        game_state.current_card = None;
        info!("{:?} burned the pile (4-of-a-kind or 10), player {} goes again", rank, playing_player);
        let player = &game_state.players[playing_player];
        if player.hand.is_empty() && player.face_up_cards.is_empty() && player.face_down_cards.is_empty() {
            game_state.phase = GamePhase::GameOver;
            game_state.winner = Some(playing_player);
        }
        return; // same player goes again
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

    // Win check
    let player = &game_state.players[playing_player];
    if player.hand.is_empty() && player.face_up_cards.is_empty() && player.face_down_cards.is_empty() {
        game_state.phase = GamePhase::GameOver;
        game_state.winner = Some(playing_player);
        return;
    }

    game_state.current_player = (playing_player + 1) % game_state.players.len();
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
        game_state.current_player = (current_player_index + 1) % game_state.players.len();
        info!("Player picked up cards");
    }
}

/// Smarter AI: plays lowest valid normal card first, saves specials (3, 10, 2) as fallbacks.
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
    if !ai_timer.0.tick(time.delta()).just_finished() {
        return;
    }

    if game_state.needs_to_pickup {
        let idx = game_state.current_player;
        pickup_cards_in_play(&mut game_state, idx);
        game_state.needs_to_pickup = false;
        game_state.current_player = (idx + 1) % game_state.players.len();
        info!("AI {} picked up cards", idx);
        return;
    }

    let effective_rank = game_state.effective_rank;
    let sa = game_state.seven_active;
    let acp = game_state.any_card_playable;
    let current_idx = game_state.current_player;
    let draw_pile_not_empty = !game_state.draw_pile.is_empty();

    // Collect candidate cards with priority: lowest normal → 3 → 10 → 2
    let player = &game_state.players[current_idx];
    let source: &[(Entity, usize, &str)] = &[];
    let _ = source; // we build it dynamically below

    // Determine which set to play from
    let (card_slice, base_idx, source_name): (&[Entity], usize, &str) =
        if draw_pile_not_empty || !player.hand.is_empty() {
            (&player.hand, 6, "hand")
        } else if !player.face_up_cards.is_empty() {
            (&player.face_up_cards, 0, "face_up")
        } else {
            (&player.face_down_cards, 3, "face_down")
        };

    // Clone so we can borrow game_state mutably afterwards
    let card_slice: Vec<Entity> = card_slice.to_vec();

    let mut best_normal: Option<(Entity, usize, Rank)> = None;
    let mut first_three: Option<(Entity, usize, Rank)> = None;
    let mut first_ten: Option<(Entity, usize, Rank)> = None;
    let mut first_two: Option<(Entity, usize, Rank)> = None;

    for (i, card_entity) in card_slice.iter().enumerate() {
        if let Ok(card) = cards.get(*card_entity) {
            if !can_play_card(card, effective_rank, sa, acp) {
                continue;
            }
            match card.rank {
                Rank::Three if first_three.is_none() => {
                    first_three = Some((*card_entity, base_idx + i, card.rank));
                }
                Rank::Ten if first_ten.is_none() => {
                    first_ten = Some((*card_entity, base_idx + i, card.rank));
                }
                Rank::Two if first_two.is_none() => {
                    first_two = Some((*card_entity, base_idx + i, card.rank));
                }
                r if !matches!(r, Rank::Three | Rank::Ten | Rank::Two) => {
                    if best_normal.is_none() || (r as u8) < (best_normal.unwrap().2 as u8) {
                        best_normal = Some((*card_entity, base_idx + i, r));
                    }
                }
                _ => {}
            }
        }
    }

    if let Some((entity, idx, rank)) = best_normal.or(first_three).or(first_ten).or(first_two) {
        let start_pos = transforms.get(entity).map(|t| t.translation()).unwrap_or(Vec3::ZERO);
        play_card(&mut commands, &mut game_state, entity, idx, rank, start_pos);
        info!("AI {} played {:?} from {}", current_idx, rank, source_name);
    } else {
        if !game_state.needs_to_pickup {
            game_state.needs_to_pickup = true;
            info!("AI {} needs to pick up cards", current_idx);
        }
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

    while game_state.players[0].hand.len() < 3 && !game_state.draw_pile.is_empty() {
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

fn game_over_screen_system(
    mut commands: Commands,
    game_state: Res<GameState>,
    screen_q: Query<Entity, With<GameOverScreen>>,
    asset_server: Res<AssetServer>,
) {
    if game_state.phase != GamePhase::GameOver || !screen_q.is_empty() {
        return;
    }

    let msg = match game_state.winner {
        Some(0) => "You Win!",
        Some(_) => "You Lose!",
        None    => "Game Over",
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
                row_gap: Val::Px(16.0),
                ..default()
            },
            background_color: Color::srgba(0.0, 0.0, 0.0, 0.6).into(),
            ..default()
        },
    )).with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            msg,
            TextStyle {
                font: font.clone(),
                font_size: 80.0,
                color: Color::WHITE,
            },
        ));
        parent.spawn(TextBundle::from_section(
            "Press any key to restart",
            TextStyle {
                font,
                font_size: 28.0,
                color: Color::srgba(1.0, 1.0, 1.0, 0.8),
            },
        ));
    });
}

fn restart_game_system(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    card_q: Query<Entity, With<Card>>,
    screen_q: Query<Entity, With<GameOverScreen>>,
    status_q: Query<Entity, With<PileStatusText>>,
    asset_server: Res<AssetServer>,
) {
    if game_state.phase != GamePhase::GameOver { return; }
    if keyboard.get_just_pressed().next().is_none() { return; }

    // Despawn all game entities
    for e in card_q.iter() { commands.entity(e).despawn_recursive(); }
    for e in screen_q.iter() { commands.entity(e).despawn_recursive(); }
    for e in status_q.iter() { commands.entity(e).despawn_recursive(); }

    // Reset and re-deal
    *game_state = GameState::new();
    game_state.add_player("Player".to_string());
    game_state.add_player("AI 1".to_string());
    game_state.add_player("AI 2".to_string());
    game_state.add_player("AI 3".to_string());

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

    info!("Game restarted");
}
