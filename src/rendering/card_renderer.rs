use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy::sprite::{MaterialMesh2dBundle, Mesh2dHandle};
use crate::components::game::{GameState, GamePhase, MatchState};
use crate::components::card_visual::update_card_visuals;
use crate::rendering::card_constants::{CARD_WIDTH, CARD_HEIGHT, CARD_OVERLAP, Z_INDEX_STEP, PLAY_PILE_X, HAND_FAN_STEP, HAND_FAN_ANGLE, HAND_FAN_ARC};
use crate::systems::input::HoveredCard;
use crate::theme;

/// When set, the arcade "juice" (score pops, burn flashes) degrades to nothing
/// so the game runs still. Defaults off; future settings UI can flip it.
#[derive(Resource, Default)]
pub struct ReducedMotion(pub bool);

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
           .init_resource::<ReducedMotion>()
           .add_systems(Startup, setup)
           .add_systems(Update, (
               update_layout,
               update_card_animations,
               layout_cards,
               update_card_visuals,
               update_pickup_highlight,
               update_turn_chip,
               manage_seat_avatars,
               update_pile_badge,
               update_floating_text,
               update_burn_flash,
               detect_juice_events,
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
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
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
    // Felt-dark fills the letterbox bars around the design rect; the felt-base
    // table sits on top of it.
    commands.insert_resource(ClearColor(theme::FELT_DARK));

    // Felt table: a large base sprite (covers both the landscape and portrait
    // design rects) far behind everything, with a slightly darker vignette plate
    // on top for the rim feel. A true radial gradient needs a shader — deferred.
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: theme::FELT_BASE,
            custom_size: Some(Vec2::new(2600.0, 2600.0)),
            ..default()
        },
        transform: Transform::from_xyz(0.0, 0.0, -200.0),
        ..default()
    });
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: theme::FELT_INK.with_alpha(0.30),
            custom_size: Some(Vec2::new(2600.0, 2600.0)),
            ..default()
        },
        transform: Transform::from_xyz(0.0, 0.0, -199.0),
        ..default()
    });

    // Gold inset frame rim — a screen-space bordered node so it always hugs the
    // visible canvas regardless of orientation. FocusPolicy::Pass so it never
    // eats button clicks.
    commands.spawn(NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            // Inset on all four sides instead of width/height + margin, which
            // overflows the viewport and clips the bottom/right edges.
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            right: Val::Px(8.0),
            bottom: Val::Px(8.0),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        border_color: theme::GOLD.with_alpha(0.35).into(),
        border_radius: BorderRadius::all(Val::Px(14.0)),
        focus_policy: bevy::ui::FocusPolicy::Pass,
        ..default()
    });

    // Pile-count badge — a magenta pill parked top-right of the play pile,
    // showing the live `×N` stack size. Hidden when the pile is empty.
    let badge_font = asset_server.load("fonts/Silkscreen-Regular.ttf");
    commands
        .spawn((
            PileCountBadge,
            SpriteBundle {
                sprite: Sprite {
                    color: theme::MAGENTA,
                    custom_size: Some(Vec2::new(60.0, 34.0)),
                    ..default()
                },
                transform: Transform::from_xyz(
                    PLAY_PILE_X + CARD_WIDTH / 2.0 + 10.0,
                    CARD_HEIGHT / 2.0 - 4.0,
                    620.0,
                ),
                visibility: Visibility::Hidden,
                ..default()
            },
        ))
        .with_children(|badge| {
            badge.spawn((
                PileCountText,
                Text2dBundle {
                    text: Text::from_section(
                        "",
                        TextStyle { font: badge_font, font_size: 19.0, color: Color::WHITE },
                    ),
                    transform: Transform::from_xyz(0.0, 0.0, 0.1),
                    ..default()
                },
            ));
        });

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

    // "Whose turn" poker chip — a single round token repositioned below the
    // active seat's cards each frame by `update_turn_chip`. A solid chip reads
    // clearly distinct from the pulsing pickup highlight (the old seat glow was
    // too easily confused with it). Hidden until play begins.
    let chip_font = asset_server.load("fonts/Rubik-Regular.ttf");
    commands
        .spawn((
            TurnChip,
            SpatialBundle {
                transform: Transform::from_xyz(0.0, 0.0, 700.0),
                visibility: Visibility::Hidden,
                ..default()
            },
        ))
        .with_children(|chip| {
            // White rim, then the coloured disc on top.
            chip.spawn(MaterialMesh2dBundle {
                mesh: Mesh2dHandle(meshes.add(Circle::new(TURN_CHIP_RADIUS + 4.0))),
                material: materials.add(Color::srgb(0.96, 0.96, 0.96)),
                transform: Transform::from_xyz(0.0, 0.0, -0.2),
                ..default()
            });
            chip.spawn(MaterialMesh2dBundle {
                mesh: Mesh2dHandle(meshes.add(Circle::new(TURN_CHIP_RADIUS))),
                material: materials.add(Color::srgb(0.78, 0.12, 0.16)),
                transform: Transform::from_xyz(0.0, 0.0, -0.1),
                ..default()
            });
            chip.spawn((
                TurnChipLabel,
                Text2dBundle {
                    text: Text::from_section(
                        "Your turn",
                        TextStyle { font: chip_font, font_size: 16.0, color: Color::WHITE },
                    )
                    .with_justify(JustifyText::Center),
                    text_2d_bounds: bevy::text::Text2dBounds {
                        size: Vec2::new(TURN_CHIP_RADIUS * 1.7, TURN_CHIP_RADIUS * 2.0),
                    },
                    transform: Transform::from_xyz(0.0, 0.0, 0.1),
                    ..default()
                },
            ));
        });

    info!("Card renderer setup complete!");
}

/// Radius of the "whose turn" poker chip in design-space units.
const TURN_CHIP_RADIUS: f32 = 44.0;

/// The single "whose turn" poker chip. Repositioned below the active seat's
/// cards each frame; its label switches between "Your turn" and the active AI's
/// name.
#[derive(Component)]
pub struct TurnChip;

/// Marker on the chip's text child so the turn label can be rewritten.
#[derive(Component)]
pub struct TurnChipLabel;

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
                // Human hand sits near the bottom edge of the design rect (not the
                // live window) so it no longer drifts with the AutoMin scale. In
                // portrait it's lifted clear of the bottom-centre Play/Done button
                // (a ~52px screen-space widget) so the two don't overlap on a phone.
                let base = -layout.design_height / 2.0 + CARD_HEIGHT / 2.0;
                if layout.orientation == Orientation::Portrait {
                    base + 150.0
                } else {
                    base
                }
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

/// The "whose turn" poker chip is disabled for now: the seat avatars + the
/// active-seat gold ring already signal whose turn it is, and the chip overlapped
/// cards in some layouts. The chip entity is still spawned (hidden) so this can
/// be reverted by restoring the positioning logic from git history.
fn update_turn_chip(mut chip_q: Query<&mut Visibility, With<TurnChip>>) {
    if let Ok(mut vis) = chip_q.get_single_mut() {
        *vis = Visibility::Hidden;
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

// ── Seat avatars + mood tags + active-seat ring ────────────────────────────

/// Radius of an AI seat's avatar disc, in design-space units.
const AVATAR_RADIUS: f32 = 26.0;

/// Root marker for an AI seat's avatar cluster (disc + monogram + name + mood).
#[derive(Component)]
pub struct SeatAvatar(pub usize);

/// Gold glow ring behind a seat's avatar, shown only on that seat's turn.
#[derive(Component)]
pub struct SeatRing(pub usize);

/// Computes the design-space anchor for seat `seat`'s avatar cluster — sat above
/// the seat's card block, scaled with the seat.
fn avatar_anchor(seat: usize, layout: &Layout) -> Vec3 {
    let (table_x, face_y, _is_bottom) = seat_anchor(seat, layout.orientation);
    let scale = seat_scale(seat, layout.orientation);
    Vec3::new(table_x, face_y + 120.0 * scale, 650.0)
}

/// Spawns one avatar per AI seat the first time the roster exists, then keeps
/// every avatar parked above its seat and toggles the gold active-seat ring.
#[allow(clippy::too_many_arguments)]
fn manage_seat_avatars(
    mut commands: Commands,
    game_state: Res<GameState>,
    layout: Res<Layout>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut avatar_q: Query<(&SeatAvatar, &mut Transform)>,
    mut ring_q: Query<(&SeatRing, &mut Visibility)>,
) {
    // One-time spawn once the players are seated.
    if avatar_q.is_empty() && game_state.players.len() > 1 {
        let ui_font = asset_server.load("fonts/Rubik-Regular.ttf");
        let pixel_font = asset_server.load("fonts/Silkscreen-Regular.ttf");
        for seat in 1..game_state.players.len() {
            let player = &game_state.players[seat];
            let color = theme::seat_color(seat, player.personality);
            let mono = player
                .name
                .chars()
                .next()
                .unwrap_or('?')
                .to_uppercase()
                .to_string();
            let mood = theme::seat_mood(player.personality);
            let scale = seat_scale(seat, layout.orientation);
            let pos = avatar_anchor(seat, &layout);

            commands
                .spawn((
                    SeatAvatar(seat),
                    SpatialBundle {
                        transform: Transform::from_translation(pos)
                            .with_scale(Vec3::splat(scale)),
                        ..default()
                    },
                ))
                .with_children(|av| {
                    // Gold active-seat ring (hidden until it's this seat's turn).
                    av.spawn((
                        SeatRing(seat),
                        MaterialMesh2dBundle {
                            mesh: Mesh2dHandle(meshes.add(Circle::new(AVATAR_RADIUS + 7.0))),
                            material: materials.add(theme::GOLD),
                            transform: Transform::from_xyz(0.0, 0.0, -0.3),
                            visibility: Visibility::Hidden,
                            ..default()
                        },
                    ));
                    // White rim, seat-colour disc, monogram.
                    av.spawn(MaterialMesh2dBundle {
                        mesh: Mesh2dHandle(meshes.add(Circle::new(AVATAR_RADIUS + 3.0))),
                        material: materials.add(Color::srgb(0.96, 0.96, 0.96)),
                        transform: Transform::from_xyz(0.0, 0.0, -0.2),
                        ..default()
                    });
                    av.spawn(MaterialMesh2dBundle {
                        mesh: Mesh2dHandle(meshes.add(Circle::new(AVATAR_RADIUS))),
                        material: materials.add(color),
                        transform: Transform::from_xyz(0.0, 0.0, -0.1),
                        ..default()
                    });
                    av.spawn(Text2dBundle {
                        text: Text::from_section(
                            mono,
                            TextStyle { font: pixel_font.clone(), font_size: 20.0, color: Color::WHITE },
                        ),
                        transform: Transform::from_xyz(0.0, 0.0, 0.1),
                        ..default()
                    });
                    // Name + mood tag, stacked below the disc.
                    av.spawn(Text2dBundle {
                        text: Text::from_section(
                            player.name.clone(),
                            TextStyle { font: ui_font.clone(), font_size: 15.0, color: Color::WHITE },
                        ),
                        transform: Transform::from_xyz(0.0, -AVATAR_RADIUS - 14.0, 0.1),
                        ..default()
                    });
                    av.spawn(Text2dBundle {
                        text: Text::from_section(
                            mood,
                            TextStyle { font: pixel_font.clone(), font_size: 13.0, color: theme::GOLD },
                        ),
                        transform: Transform::from_xyz(0.0, -AVATAR_RADIUS - 34.0, 0.1),
                        ..default()
                    });
                });
        }
        return; // positions/ring handled next frame once entities exist
    }

    // Reposition every avatar above its seat (orientation may have flipped).
    for (avatar, mut transform) in avatar_q.iter_mut() {
        let scale = seat_scale(avatar.0, layout.orientation);
        transform.translation = avatar_anchor(avatar.0, &layout);
        transform.scale = Vec3::splat(scale);
    }

    // Light the gold ring on the active seat only.
    let active_seat = game_state.current_player;
    let playing = game_state.phase == GamePhase::Playing
        && !game_state.finish_order.contains(&active_seat);
    for (ring, mut vis) in ring_q.iter_mut() {
        *vis = if playing && ring.0 == active_seat {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

// ── Pile-count badge ───────────────────────────────────────────────────────

/// The magenta `×N` pill parked beside the play pile.
#[derive(Component)]
pub struct PileCountBadge;

/// Marker on the badge's text child.
#[derive(Component)]
pub struct PileCountText;

fn update_pile_badge(
    game_state: Res<GameState>,
    mut badge_q: Query<&mut Visibility, With<PileCountBadge>>,
    mut text_q: Query<&mut Text, With<PileCountText>>,
) {
    let count = game_state.cards_in_play.len();
    if let Ok(mut vis) = badge_q.get_single_mut() {
        *vis = if count > 0 { Visibility::Visible } else { Visibility::Hidden };
    }
    if let Ok(mut text) = text_q.get_single_mut() {
        let label = format!("\u{00D7}{}", count);
        if text.sections[0].value != label {
            text.sections[0].value = label;
        }
    }
}

// ── Juice: floating "+N" pops + burn flash ─────────────────────────────────

/// A short-lived Text2d that drifts up and fades out (the "+N!" score pop).
#[derive(Component)]
pub struct FloatingText {
    age: f32,
    ttl: f32,
}

/// A short-lived flash sprite over the pile when it burns.
#[derive(Component)]
pub struct BurnFlash {
    age: f32,
    ttl: f32,
}

fn update_floating_text(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Transform, &mut Text, &mut FloatingText)>,
) {
    for (entity, mut transform, mut text, mut float) in q.iter_mut() {
        float.age += time.delta_seconds();
        let frac = (float.age / float.ttl).clamp(0.0, 1.0);
        transform.translation.y += 60.0 * time.delta_seconds();
        let alpha = 1.0 - frac;
        for section in text.sections.iter_mut() {
            section.style.color = section.style.color.with_alpha(alpha);
        }
        if float.age >= float.ttl {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn update_burn_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Sprite, &mut BurnFlash)>,
) {
    for (entity, mut sprite, mut flash) in q.iter_mut() {
        flash.age += time.delta_seconds();
        let frac = (flash.age / flash.ttl).clamp(0.0, 1.0);
        sprite.color = sprite.color.with_alpha(0.7 * (1.0 - frac));
        if flash.age >= flash.ttl {
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// Watches round state for two presentational events without touching gameplay:
/// a new finisher (spawn a lime "+N!" pop at their seat) and the discard pile
/// growing (a burn just happened — flash the pile). Tracked via `Local` so we
/// only fire on the frame the count changes.
fn detect_juice_events(
    mut commands: Commands,
    game_state: Res<GameState>,
    layout: Res<Layout>,
    reduced: Res<ReducedMotion>,
    asset_server: Res<AssetServer>,
    mut last_finished: Local<usize>,
    mut last_discard: Local<usize>,
) {
    let finished = game_state.finish_order.len();
    let discard = game_state.discard_pile.len();

    // New round (counts reset) — resync without firing.
    if finished < *last_finished {
        *last_finished = finished;
    }
    if discard < *last_discard {
        *last_discard = discard;
    }

    if !reduced.0 {
        // One pop per newly-eliminated seat.
        if finished > *last_finished {
            let total = game_state.players.len();
            let pixel_font = asset_server.load("fonts/Silkscreen-Regular.ttf");
            for pos in *last_finished..finished {
                let seat = game_state.finish_order[pos];
                let pts = MatchState::score_for_position(pos, total);
                let label = if pts > 0 { format!("+{}!", pts) } else { "SHED!".to_string() };
                let (table_x, face_y, _) = seat_anchor(seat, layout.orientation);
                commands.spawn((
                    FloatingText { age: 0.0, ttl: 0.75 },
                    Text2dBundle {
                        text: Text::from_section(
                            label,
                            TextStyle { font: pixel_font.clone(), font_size: 22.0, color: theme::LIME },
                        ),
                        transform: Transform::from_xyz(table_x, face_y, 660.0)
                            .with_rotation(Quat::from_rotation_z((-8.0_f32).to_radians())),
                        ..default()
                    },
                ));
            }
        }

        // Pile burned (cards moved to discard).
        if discard > *last_discard {
            commands.spawn((
                BurnFlash { age: 0.0, ttl: 0.35 },
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::srgba(1.0, 0.85, 0.4, 0.7),
                        custom_size: Some(Vec2::new(CARD_WIDTH + 30.0, CARD_HEIGHT + 30.0)),
                        ..default()
                    },
                    transform: Transform::from_xyz(PLAY_PILE_X, 0.0, 615.0),
                    ..default()
                },
            ));
        }
    }

    *last_finished = finished;
    *last_discard = discard;
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