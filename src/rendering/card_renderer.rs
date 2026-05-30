use bevy::prelude::*;
use crate::components::game::{GameState, GamePhase};
use crate::components::card_visual::update_card_visuals;
use crate::rendering::card_constants::{CARD_WIDTH, CARD_HEIGHT, CARD_OVERLAP, Z_INDEX_STEP, PLAY_PILE_X, HAND_FAN_STEP, HAND_FAN_ANGLE, HAND_FAN_ARC};
use crate::systems::input::HoveredCard;

/// Marker for the highlight sprite shown on the play pile when the player must pick up.
#[derive(Component)]
pub struct PickupHighlight;

#[derive(Component)]
pub struct CardAnimation {
    pub target_position: Vec3,
    pub start_position: Vec3,
    pub progress: f32,
    pub speed: f32,
}

pub struct CardRendererPlugin;

impl Plugin for CardRendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
           .add_systems(Update, (
               update_card_animations,
               layout_cards,
               update_card_visuals,
               update_pickup_highlight,
           ));
    }
}

fn setup(
    mut commands: Commands,
) {
    info!("Starting card renderer setup...");

    commands.spawn(Camera2dBundle::default());
    commands.insert_resource(ClearColor(Color::srgb(0.2, 0.5, 0.2)));

    // Spawn the pickup-highlight sprite behind the play pile (hidden by default)
    commands.spawn((
        PickupHighlight,
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgba(1.0, 0.8, 0.0, 0.0),
                custom_size: Some(Vec2::new(CARD_WIDTH + 24.0, CARD_HEIGHT + 24.0)),
                ..default()
            },
            transform: Transform::from_xyz(PLAY_PILE_X, 0.0, 490.0),
            visibility: Visibility::Hidden,
            ..default()
        },
    ));

    info!("Card renderer setup complete!");
}

/// Card-set categories, matching the `card_index / 3` grouping used while
/// dealing: face-down (furthest from the pile), face-up, then the fanned hand.
pub(crate) const SET_FACE_DOWN: usize = 0;
pub(crate) const SET_FACE_UP: usize = 1;
pub(crate) const SET_HAND: usize = 2;

/// Per-seat layout anchor: `(table_x, face_y, is_bottom)`. `table_x` is the
/// centre of the seat's 3-column table block; `is_bottom` flips row/fan
/// directions for the human at the near edge.
pub(crate) fn seat_anchor(player_index: usize) -> (f32, f32, bool) {
    match player_index {
        0 => (   0.0, -200.0, true),  // human — bottom centre
        1 => (-440.0,  220.0, false), // AI 1  — top left
        2 => (   0.0,  220.0, false), // AI 2  — top centre
        3 => ( 440.0,  220.0, false), // AI 3  — top right
        _ => (   0.0,    0.0, true),
    }
}

/// The resting transform for a single card at index `idx` within its set for
/// `player_index`. `hand_count` is the number of cards the seat's hand will
/// hold (only used to centre the fan for `SET_HAND`). This is the single source
/// of truth for card positions — both `layout_cards` (per frame) and
/// `deal_next_card` (deal animation target) call it, so a dealt card flies
/// straight to where layout will keep it, with no snap when the animation ends.
pub(crate) fn card_resting_transform(
    player_index: usize,
    set_type: usize,
    idx: usize,
    hand_count: usize,
    window_height: f32,
) -> (Vec3, Quat) {
    let (table_x, face_y, is_bottom) = seat_anchor(player_index);
    let col_step = CARD_WIDTH + CARD_OVERLAP; // px between column centres

    match set_type {
        SET_FACE_DOWN => {
            let x = table_x + (idx as f32 - 1.0) * col_step;
            let y = if is_bottom { face_y - 30.0 } else { face_y + 30.0 };
            (Vec3::new(x, y, idx as f32 * Z_INDEX_STEP), Quat::IDENTITY)
        }
        SET_FACE_UP => {
            let x = table_x + (idx as f32 - 1.0) * col_step;
            let y = if is_bottom { face_y + 30.0 } else { face_y - 30.0 };
            (Vec3::new(x, y, 100.0 + idx as f32 * Z_INDEX_STEP), Quat::IDENTITY)
        }
        _ => {
            let hand_base_y = if is_bottom {
                -window_height / 2.0 + CARD_HEIGHT / 2.0
            } else {
                window_height / 2.0 - CARD_HEIGHT / 2.0
            };
            let offset = idx as f32 - (hand_count.saturating_sub(1)) as f32 / 2.0;
            let x = table_x + offset * HAND_FAN_STEP;
            let y = hand_base_y - offset.abs() * HAND_FAN_ARC;
            let angle = if is_bottom {
                -offset * HAND_FAN_ANGLE.to_radians()
            } else {
                offset * HAND_FAN_ANGLE.to_radians()
            };
            (Vec3::new(x, y, 200.0 + idx as f32 * Z_INDEX_STEP), Quat::from_rotation_z(angle))
        }
    }
}

// System to layout cards based on game state
fn layout_cards(
    game_state: Res<GameState>,
    hovered: Res<HoveredCard>,
    mut transform_query: Query<&mut Transform, Without<CardAnimation>>,
    windows: Query<&Window>,
) {
    let window = windows.single();
    let window_height = window.height();

    for (player_index, player) in game_state.players.iter().enumerate() {
        let is_bottom = seat_anchor(player_index).2;

        // Face-down cards — further from the play area
        for (i, &card_entity) in player.face_down_cards.iter().take(3).enumerate() {
            if let Ok(mut t) = transform_query.get_mut(card_entity) {
                let (pos, rot) = card_resting_transform(player_index, SET_FACE_DOWN, i, 0, window_height);
                t.translation = pos;
                t.rotation = rot;
            }
        }

        // Face-up cards — closer to the play area
        for (i, &card_entity) in player.face_up_cards.iter().take(3).enumerate() {
            if let Ok(mut t) = transform_query.get_mut(card_entity) {
                let (pos, rot) = card_resting_transform(player_index, SET_FACE_UP, i, 0, window_height);
                t.translation = pos;
                t.rotation = rot;
            }
        }

        // Hand — fanned at the near window edge, centred over the table block
        let hand_count = player.hand.len();
        for (i, &card_entity) in player.hand.iter().enumerate() {
            if let Ok(mut t) = transform_query.get_mut(card_entity) {
                let (mut pos, rot) =
                    card_resting_transform(player_index, SET_HAND, i, hand_count, window_height);
                // Raise hovered or selected cards toward the play area
                if hovered.0 == Some(card_entity)
                    || game_state.selected_cards.contains(&card_entity)
                {
                    pos.y += if is_bottom { 20.0 } else { -20.0 };
                }
                t.translation = pos;
                t.rotation = rot;
            }
        }
    }

    // Draw pile — centred on screen
    for (i, &card_entity) in game_state.draw_pile.iter().enumerate() {
        if let Ok(mut t) = transform_query.get_mut(card_entity) {
            t.translation = Vec3::new(0.0, 0.0, 400.0 - i as f32 * Z_INDEX_STEP);
            t.rotation = Quat::IDENTITY;
        }
    }

    // Play pile
    let base_z_play = 500.0;
    for (i, &card_entity) in game_state.cards_in_play.iter().enumerate() {
        if let Ok(mut t) = transform_query.get_mut(card_entity) {
            let z_offset = (game_state.cards_in_play.len() - i - 1) as f32 * Z_INDEX_STEP;
            t.translation = Vec3::new(PLAY_PILE_X, 0.0, base_z_play - z_offset);
            t.rotation = Quat::IDENTITY;
        }
    }

    // Discard pile (burned cards sit under the play pile)
    for (i, &card_entity) in game_state.discard_pile.iter().enumerate() {
        if let Ok(mut t) = transform_query.get_mut(card_entity) {
            t.translation = Vec3::new(PLAY_PILE_X, 0.0, 450.0 - i as f32 * Z_INDEX_STEP);
            t.rotation = Quat::IDENTITY;
        }
    }
}

// System to update card animations
fn update_card_animations(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut CardAnimation)>,
) {
    for (entity, mut transform, mut animation) in query.iter_mut() {
        animation.progress = (animation.progress + animation.speed * time.delta_seconds()).min(1.0);
        transform.translation = animation.start_position.lerp(
            animation.target_position,
            animation.progress,
        );
        if animation.progress >= 1.0 {
            commands.entity(entity).remove::<CardAnimation>();
        }
    }
}

// Pulse the pickup highlight when the human player needs to pick up cards
fn update_pickup_highlight(
    game_state: Res<GameState>,
    time: Res<Time>,
    mut query: Query<(&mut Visibility, &mut Sprite), With<PickupHighlight>>,
) {
    let show = game_state.needs_to_pickup
        && game_state.current_player == 0
        && game_state.phase == GamePhase::Playing;

    for (mut vis, mut sprite) in &mut query {
        if show {
            *vis = Visibility::Visible;
            let alpha = 0.4 + 0.35 * (time.elapsed_seconds() * 4.0).sin();
            sprite.color = Color::srgba(1.0, 0.75, 0.0, alpha);
        } else {
            *vis = Visibility::Hidden;
        }
    }
}