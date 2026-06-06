use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
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
        app.init_resource::<Layout>()
           .add_systems(Startup, setup)
           .add_systems(Update, (
               update_layout,
               update_card_animations,
               layout_cards,
               update_card_visuals,
               update_pickup_highlight,
               update_seat_highlight,
           ));
    }
}

/// Active screen orientation. Drives both the camera design rect and the
/// per-seat layout. `Landscape` is the original desktop layout (unchanged);
/// `Portrait` packs the three AIs into a compact top strip and gives the
/// bottom of a tall screen to the human's hand + the pile at a larger scale.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Orientation {
    Landscape,
    Portrait,
}

/// The active design rect + orientation. Every layout helper reads this instead
/// of raw window pixels, so the table is positioned in stable design-space units
/// and the camera's `AutoMin` scaling maps that rect onto whatever canvas the
/// player has. Swapped wholesale by `update_layout` when the aspect crosses
/// square.
#[derive(Resource, Clone, Copy, Debug)]
pub struct Layout {
    pub orientation: Orientation,
    pub design_width: f32,
    pub design_height: f32,
}

impl Default for Layout {
    fn default() -> Self {
        Self::for_orientation(Orientation::Landscape)
    }
}

impl Layout {
    pub fn for_orientation(orientation: Orientation) -> Self {
        match orientation {
            // Original native layout — desktop stays pixel-identical.
            Orientation::Landscape => Self { orientation, design_width: 1440.0, design_height: 900.0 },
            // ~9:16 tall rect. Narrower design width => AutoMin scales the table
            // up, so cards read large on a phone instead of shrinking to a band.
            Orientation::Portrait => Self { orientation, design_width: 720.0, design_height: 1280.0 },
        }
    }
}

fn setup(
    mut commands: Commands,
) {
    info!("Starting card renderer setup...");

    // The table is laid out in a design-space rect centred on the origin (see
    // `Layout`). `AutoMin` keeps at least that whole rect in view on any canvas,
    // scaling down (never clipping) on smaller/differently-shaped screens. On a
    // 1440-wide desktop the landscape scale is 1:1. `update_layout` swaps the
    // min_width/min_height when the screen turns portrait.
    let layout = Layout::default();
    let mut camera = Camera2dBundle::default();
    camera.projection.scaling_mode = ScalingMode::AutoMin {
        min_width: layout.design_width,
        min_height: layout.design_height,
    };
    commands.spawn(camera);
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

    // One active-seat highlight per seat — a soft glow behind that seat's card
    // block, shown only when it's that seat's turn. Positioned per-frame by
    // `update_seat_highlight` so it tracks the orientation-aware seat anchors.
    for seat in 0..4usize {
        commands.spawn((
            SeatHighlight { seat },
            SpriteBundle {
                sprite: Sprite {
                    color: Color::srgba(1.0, 0.85, 0.2, 0.0),
                    custom_size: Some(Vec2::new(CARD_WIDTH * 3.6, CARD_HEIGHT * 1.6)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 3.0),
                visibility: Visibility::Hidden,
                ..default()
            },
        ));
    }

    info!("Card renderer setup complete!");
}

/// Behind-the-seat glow marking whose turn it is during play.
#[derive(Component)]
pub struct SeatHighlight {
    pub seat: usize,
}

/// Recomputes the active screen orientation from the window aspect and, on a
/// flip, swaps the `Layout` design rect and the camera's `AutoMin` extents.
/// Only writes when the orientation actually changes, so it's a cheap aspect
/// compare every frame.
fn update_layout(
    windows: Query<&Window>,
    mut layout: ResMut<Layout>,
    mut projection: Query<&mut OrthographicProjection>,
) {
    let Ok(window) = windows.get_single() else { return; };
    let desired = if window.height() > window.width() {
        Orientation::Portrait
    } else {
        Orientation::Landscape
    };
    if layout.orientation == desired {
        return;
    }
    *layout = Layout::for_orientation(desired);
    if let Ok(mut proj) = projection.get_single_mut() {
        proj.scaling_mode = ScalingMode::AutoMin {
            min_width: layout.design_width,
            min_height: layout.design_height,
        };
    }
}

/// Card-set categories, matching the `card_index / 3` grouping used while
/// dealing: face-down (furthest from the pile), face-up, then the fanned hand.
pub(crate) const SET_FACE_DOWN: usize = 0;
pub(crate) const SET_FACE_UP: usize = 1;
pub(crate) const SET_HAND: usize = 2;

/// Per-seat layout anchor: `(table_x, face_y, is_bottom)`. `table_x` is the
/// centre of the seat's 3-column table block; `is_bottom` flips row/fan
/// directions for the human at the near edge. Landscape values are the original
/// desktop layout; Portrait packs the three AIs into a tight top strip and sits
/// the human low so the hand + pile own the roomy bottom.
pub(crate) fn seat_anchor(player_index: usize, orientation: Orientation) -> (f32, f32, bool) {
    match orientation {
        Orientation::Landscape => match player_index {
            0 => (   0.0, -200.0, true),  // human — bottom centre
            1 => (-440.0,  220.0, false), // AI 1  — top left
            2 => (   0.0,  220.0, false), // AI 2  — top centre
            3 => ( 440.0,  220.0, false), // AI 3  — top right
            _ => (   0.0,    0.0, true),
        },
        // Design rect 720x1280 => y in [-640, 640].
        Orientation::Portrait => match player_index {
            0 => (   0.0, -180.0, true),  // human — low; hand anchors to the bottom edge
            1 => (-235.0,  520.0, false), // AI 1  — top strip, left
            2 => (   0.0,  520.0, false), // AI 2  — top strip, centre
            3 => ( 235.0,  520.0, false), // AI 3  — top strip, right
            _ => (   0.0,    0.0, true),
        },
    }
}

/// Visual scale for a seat's cards. The human always renders full size; in
/// portrait the AIs shrink so three seats fit across the narrow top strip
/// without overlapping. Landscape is always 1.0, so desktop is unchanged.
pub(crate) fn seat_scale(player_index: usize, orientation: Orientation) -> f32 {
    match (orientation, player_index) {
        (Orientation::Portrait, 0) => 1.0, // human stays large
        (Orientation::Portrait, _) => 0.62,
        (Orientation::Landscape, _) => 1.0,
    }
}

/// The resting transform for a single card at index `idx` within its set for
/// `player_index`. `hand_count` is the number of cards the seat's hand will
/// hold (only used to centre the fan for `SET_HAND`). This is the single source
/// of truth for card positions — both `layout_cards` (per frame) and
/// `deal_next_card` (deal animation target) call it, so a dealt card flies
/// straight to where layout will keep it, with no snap when the animation ends.
///
/// Returns `(translation, rotation, scale)`. `scale` is the per-seat visual
/// scale (1.0 except shrunken portrait AIs) — callers apply it to the card's
/// `Transform.scale`. In landscape every seat scales 1.0, so the geometry is
/// byte-identical to the original layout.
pub(crate) fn card_resting_transform(
    player_index: usize,
    set_type: usize,
    idx: usize,
    hand_count: usize,
    layout: &Layout,
) -> (Vec3, Quat, f32) {
    let (table_x, face_y, is_bottom) = seat_anchor(player_index, layout.orientation);
    let scale = seat_scale(player_index, layout.orientation);
    let col_step = (CARD_WIDTH + CARD_OVERLAP) * scale; // px between column centres
    let row_offset = 30.0 * scale; // face-up/face-down vertical split

    match set_type {
        SET_FACE_DOWN => {
            let x = table_x + (idx as f32 - 1.0) * col_step;
            let y = if is_bottom { face_y - row_offset } else { face_y + row_offset };
            (Vec3::new(x, y, idx as f32 * Z_INDEX_STEP), Quat::IDENTITY, scale)
        }
        SET_FACE_UP => {
            let x = table_x + (idx as f32 - 1.0) * col_step;
            let y = if is_bottom { face_y + row_offset } else { face_y - row_offset };
            (Vec3::new(x, y, 100.0 + idx as f32 * Z_INDEX_STEP), Quat::IDENTITY, scale)
        }
        _ => {
            let hand_base_y = if is_bottom {
                // Human hand sits at the bottom edge of the design rect (not the
                // live window) so it no longer drifts with the AutoMin scale.
                -layout.design_height / 2.0 + CARD_HEIGHT / 2.0
            } else if layout.orientation == Orientation::Portrait {
                // AI hands ride just outside their face rows in the compact strip
                // rather than at the far top edge, so the seat reads as a unit.
                face_y + 90.0 * scale
            } else {
                layout.design_height / 2.0 - CARD_HEIGHT / 2.0
            };
            let offset = idx as f32 - (hand_count.saturating_sub(1)) as f32 / 2.0;
            let x = table_x + offset * HAND_FAN_STEP * scale;
            let y = hand_base_y - offset.abs() * HAND_FAN_ARC * scale;
            let angle = if is_bottom {
                -offset * HAND_FAN_ANGLE.to_radians()
            } else {
                offset * HAND_FAN_ANGLE.to_radians()
            };
            (Vec3::new(x, y, 200.0 + idx as f32 * Z_INDEX_STEP), Quat::from_rotation_z(angle), scale)
        }
    }
}

// System to layout cards based on game state
fn layout_cards(
    game_state: Res<GameState>,
    hovered: Res<HoveredCard>,
    layout: Res<Layout>,
    mut transform_query: Query<&mut Transform, Without<CardAnimation>>,
) {
    for (player_index, player) in game_state.players.iter().enumerate() {
        let is_bottom = seat_anchor(player_index, layout.orientation).2;

        // Face-down cards — further from the play area
        for (i, &card_entity) in player.face_down_cards.iter().take(3).enumerate() {
            if let Ok(mut t) = transform_query.get_mut(card_entity) {
                let (pos, rot, scale) = card_resting_transform(player_index, SET_FACE_DOWN, i, 0, &layout);
                t.translation = pos;
                t.rotation = rot;
                t.scale = Vec3::splat(scale);
            }
        }

        // Face-up cards — closer to the play area
        for (i, &card_entity) in player.face_up_cards.iter().take(3).enumerate() {
            if let Ok(mut t) = transform_query.get_mut(card_entity) {
                let (pos, rot, scale) = card_resting_transform(player_index, SET_FACE_UP, i, 0, &layout);
                t.translation = pos;
                t.rotation = rot;
                t.scale = Vec3::splat(scale);
            }
        }

        // Hand — fanned at the near window edge, centred over the table block
        let hand_count = player.hand.len();
        for (i, &card_entity) in player.hand.iter().enumerate() {
            if let Ok(mut t) = transform_query.get_mut(card_entity) {
                let (mut pos, rot, scale) =
                    card_resting_transform(player_index, SET_HAND, i, hand_count, &layout);
                // Raise hovered or selected cards toward the play area
                if hovered.0 == Some(card_entity)
                    || game_state.selected_cards.contains(&card_entity)
                {
                    pos.y += if is_bottom { 20.0 } else { -20.0 };
                }
                t.translation = pos;
                t.rotation = rot;
                t.scale = Vec3::splat(scale);
            }
        }
    }

    // Draw pile — centred on screen (always full scale)
    for (i, &card_entity) in game_state.draw_pile.iter().enumerate() {
        if let Ok(mut t) = transform_query.get_mut(card_entity) {
            t.translation = Vec3::new(0.0, 0.0, 400.0 - i as f32 * Z_INDEX_STEP);
            t.rotation = Quat::IDENTITY;
            t.scale = Vec3::ONE;
        }
    }

    // Play pile — full scale, so a card played from a shrunken portrait AI hand
    // returns to size when it lands.
    let base_z_play = 500.0;
    for (i, &card_entity) in game_state.cards_in_play.iter().enumerate() {
        if let Ok(mut t) = transform_query.get_mut(card_entity) {
            let z_offset = (game_state.cards_in_play.len() - i - 1) as f32 * Z_INDEX_STEP;
            t.translation = Vec3::new(PLAY_PILE_X, 0.0, base_z_play - z_offset);
            t.rotation = Quat::IDENTITY;
            t.scale = Vec3::ONE;
        }
    }

    // Discard pile (burned cards sit under the play pile)
    for (i, &card_entity) in game_state.discard_pile.iter().enumerate() {
        if let Ok(mut t) = transform_query.get_mut(card_entity) {
            t.translation = Vec3::new(PLAY_PILE_X, 0.0, 450.0 - i as f32 * Z_INDEX_STEP);
            t.rotation = Quat::IDENTITY;
            t.scale = Vec3::ONE;
        }
    }
}

/// Positions each seat's turn-glow at its (orientation-aware) anchor and shows
/// only the active seat's glow during play.
fn update_seat_highlight(
    game_state: Res<GameState>,
    layout: Res<Layout>,
    time: Res<Time>,
    mut query: Query<(&SeatHighlight, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    for (hl, mut transform, mut sprite, mut vis) in &mut query {
        let (table_x, face_y, _) = seat_anchor(hl.seat, layout.orientation);
        let scale = seat_scale(hl.seat, layout.orientation);
        transform.translation.x = table_x;
        transform.translation.y = face_y;
        transform.scale = Vec3::splat(scale);

        let active = game_state.phase == GamePhase::Playing
            && game_state.current_player == hl.seat
            && !game_state.finish_order.contains(&hl.seat);
        if active {
            *vis = Visibility::Visible;
            let alpha = 0.18 + 0.12 * (time.elapsed_seconds() * 3.0).sin();
            sprite.color = Color::srgba(1.0, 0.85, 0.2, alpha);
        } else {
            *vis = Visibility::Hidden;
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