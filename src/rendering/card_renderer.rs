use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy::sprite::{MaterialMesh2dBundle, Mesh2dHandle};
use crate::components::card::Card;
use crate::components::game::{GameState, GamePhase, MatchState};
use crate::components::card_visual::{update_card_visuals, CardLift};
use crate::rendering::card_constants::{CARD_WIDTH, CARD_HEIGHT, CARD_OVERLAP, Z_INDEX_STEP, PLAY_PILE_X, HAND_FAN_STEP, HAND_FAN_ANGLE, HAND_FAN_ARC, ACTION_BAR_CLEARANCE};
use crate::systems::input::HoveredCard;
use crate::theme;

/// When set, the arcade "juice" (score pops, burn flashes) degrades to nothing
/// so the game runs still. Defaults off; future settings UI can flip it.
#[derive(Resource, Default)]
pub struct ReducedMotion(pub bool);

/// Marker for the highlight sprite shown on the play pile when the player must pick up.
#[derive(Component)]
pub struct PickupHighlight;

/// Easing applied to a `CardAnimation`'s progress before the position lerp.
/// `EaseOutBack` is the default "arcade" feel — a springy ease-out with a slight
/// overshoot past the target that settles back. `EaseOut` is the no-overshoot
/// variant for motions where landing past the mark reads wrong (a card flying
/// into the fanned hand). `Linear` is the legacy straight glide.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AnimCurve {
    Linear,
    EaseOut,
    #[default]
    EaseOutBack,
}

impl AnimCurve {
    /// Maps a linear progress `t` in [0,1] through the curve. `EaseOutBack` can
    /// briefly exceed 1.0 (the overshoot) before returning to exactly 1.0 at t=1.
    pub fn apply(self, t: f32) -> f32 {
        match self {
            AnimCurve::Linear => t,
            // Standard ease-out cubic.
            AnimCurve::EaseOut => 1.0 - (1.0 - t).powi(3),
            // Ease-out-back (Penner): overshoots then settles. The 1.70158
            // constant is the classic ~10% overshoot.
            AnimCurve::EaseOutBack => {
                const C1: f32 = 1.70158;
                const C3: f32 = C1 + 1.0;
                let p = t - 1.0;
                1.0 + C3 * p.powi(3) + C1 * p.powi(2)
            }
        }
    }
}

#[derive(Component)]
pub struct CardAnimation {
    pub target_position: Vec3,
    pub start_position: Vec3,
    pub progress: f32,
    pub speed: f32,
    pub curve: AnimCurve,
}

impl CardAnimation {
    /// A springy ease-out-back tween (the arcade default) from `start` to
    /// `target`. `speed` is 1/duration_seconds — e.g. 5.0 ≈ 200ms.
    pub fn springy(start: Vec3, target: Vec3, speed: f32) -> Self {
        Self { start_position: start, target_position: target, progress: 0.0, speed, curve: AnimCurve::EaseOutBack }
    }

    /// A no-overshoot ease-out tween — for landings where overshoot reads wrong
    /// (cards settling into the fanned hand).
    pub fn smooth(start: Vec3, target: Vec3, speed: f32) -> Self {
        Self { start_position: start, target_position: target, progress: 0.0, speed, curve: AnimCurve::EaseOut }
    }
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
               animate_card_lift,
               layout_cards,
               update_card_visuals,
               update_pickup_highlight,
               update_turn_chip,
               manage_seat_avatars,
               update_pile_badge,
               update_floating_text,
               update_pile_pulse,
               detect_juice_events,
               pop_active_ring,
               update_ring_pop,
               toggle_reduced_motion,
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

    /// World-space x of the play-pile centre. Landscape keeps the original
    /// off-centre anchor; portrait centres the pile so it dominates the narrow
    /// screen (the draw pile slides left to make room — see `draw_pile_x`).
    pub fn play_pile_x(&self) -> f32 {
        match self.orientation {
            Orientation::Landscape => PLAY_PILE_X,
            Orientation::Portrait => 0.0,
        }
    }

    /// World-space x of the draw pile. Centred in landscape; shifted left in
    /// portrait so the centred play pile has room.
    pub fn draw_pile_x(&self) -> f32 {
        match self.orientation {
            Orientation::Landscape => 0.0,
            Orientation::Portrait => -210.0,
        }
    }

    /// Visual scale for the play/discard pile (and its highlight/flash). Enlarged
    /// in portrait so the pile reads large on a phone; 1.0 on desktop.
    pub fn pile_scale(&self) -> f32 {
        match self.orientation {
            Orientation::Landscape => 1.0,
            Orientation::Portrait => 1.25,
        }
    }

    /// World-space centre of the "burn pit" — where burned cards sweep to and
    /// the discard pile rests, offset from the play pile so a burn visibly
    /// clears the table instead of vanishing under the next card. Sits in open
    /// felt to the side of the pile in both orientations. (x, y only; callers
    /// supply the z.)
    pub fn burn_pit(&self) -> Vec2 {
        match self.orientation {
            Orientation::Landscape => Vec2::new(PLAY_PILE_X + 250.0, 175.0),
            Orientation::Portrait => Vec2::new(225.0, 235.0),
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
        // Design rect 720x1280 => y in [-640, 640]. AI strip sits a little lower
        // than the top edge so the persistent header bar (screen-space, ~top 76
        // design px) clears the avatars above each seat.
        Orientation::Portrait => match player_index {
            0 => (   0.0, -250.0, true),  // human — low; clears the centred pile
            1 => (-235.0,  445.0, false), // AI 1  — top strip, left
            2 => (   0.0,  445.0, false), // AI 2  — top strip, centre
            3 => ( 235.0,  445.0, false), // AI 3  — top strip, right
            _ => (   0.0,    0.0, true),
        },
    }
}

/// Visual scale for a seat's cards. The human always renders full size; in
/// portrait the AIs shrink so three seats fit across the narrow top strip
/// without overlapping. Landscape is always 1.0, so desktop is unchanged.
pub(crate) fn seat_scale(player_index: usize, orientation: Orientation) -> f32 {
    match (orientation, player_index) {
        (Orientation::Portrait, 0) => 1.25, // human enlarged so the hand reads big on a phone
        (Orientation::Portrait, _) => 0.62,
        (Orientation::Landscape, _) => 1.0,
    }
}

/// Visual scale for a seat's *avatar cluster* (disc + name + mood). Deliberately
/// decoupled from `seat_scale`: the portrait AIs render their cards small to fit
/// the narrow top strip, but their avatars/names shouldn't shrink as hard or
/// they become unreadable on a phone. Landscape stays 1.0 (desktop unchanged).
pub(crate) fn avatar_scale(player_index: usize, orientation: Orientation) -> f32 {
    match (orientation, player_index) {
        (Orientation::Portrait, _) => 0.85,
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
                // live window) so it no longer drifts with the AutoMin scale, then
                // is lifted by ACTION_BAR_CLEARANCE so the bottom action bar
                // (Play/Done button + consumable mini-cards) sits below it without
                // overlap at any hand size. Portrait lifts further for the taller
                // phone layout.
                let base = -layout.design_height / 2.0 + CARD_HEIGHT / 2.0;
                if layout.orientation == Orientation::Portrait {
                    base + 150.0 + ACTION_BAR_CLEARANCE
                } else {
                    base + ACTION_BAR_CLEARANCE
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
    layout: Res<Layout>,
    lift_q: Query<&CardLift>,
    mut transform_query: Query<&mut Transform, Without<CardAnimation>>,
) {
    for (player_index, player) in game_state.players.iter().enumerate() {
        let is_bottom = seat_anchor(player_index, layout.orientation).2;

        // Face-down cards — further from the play area
        for (i, &card_entity) in player.face_down_cards.iter().take(3).enumerate() {
            if let Ok(mut t) = transform_query.get_mut(card_entity) {
                let (pos, rot, scale) = card_resting_transform(player_index, SET_FACE_DOWN, i, 0, &layout);
                t.set_if_neq(Transform { translation: pos, rotation: rot, scale: Vec3::splat(scale) });
            }
        }

        // Face-up cards — closer to the play area
        for (i, &card_entity) in player.face_up_cards.iter().take(3).enumerate() {
            if let Ok(mut t) = transform_query.get_mut(card_entity) {
                let (pos, rot, scale) = card_resting_transform(player_index, SET_FACE_UP, i, 0, &layout);
                t.set_if_neq(Transform { translation: pos, rotation: rot, scale: Vec3::splat(scale) });
            }
        }

        // Hand — fanned at the near window edge, centred over the table block
        let hand_count = player.hand.len();
        for (i, &card_entity) in player.hand.iter().enumerate() {
            if let Ok(mut t) = transform_query.get_mut(card_entity) {
                let (mut pos, rot, scale) =
                    card_resting_transform(player_index, SET_HAND, i, hand_count, &layout);
                // Springy raise for hovered/staged cards — the eased lift is
                // maintained per-card by animate_card_lift; we just add it here,
                // toward the play area (sign flips for the bottom human seat).
                let lift = lift_q.get(card_entity).map(|l| l.pos).unwrap_or(0.0);
                pos.y += if is_bottom { lift } else { -lift };
                t.set_if_neq(Transform { translation: pos, rotation: rot, scale: Vec3::splat(scale) });
            }
        }
    }

    let pile_x = layout.play_pile_x();
    let pile_scale = layout.pile_scale();

    // Draw pile — centred in landscape, shifted left in portrait (always full scale).
    let draw_x = layout.draw_pile_x();
    for (i, &card_entity) in game_state.draw_pile.iter().enumerate() {
        if let Ok(mut t) = transform_query.get_mut(card_entity) {
            let translation = Vec3::new(draw_x, 0.0, 400.0 - i as f32 * Z_INDEX_STEP);
            t.set_if_neq(Transform { translation, rotation: Quat::IDENTITY, scale: Vec3::ONE });
        }
    }

    // Play pile — pile_scale so a card played from a shrunken portrait AI hand
    // settles at the pile's display size when it lands.
    let base_z_play = 500.0;
    for (i, &card_entity) in game_state.cards_in_play.iter().enumerate() {
        if let Ok(mut t) = transform_query.get_mut(card_entity) {
            let z_offset = (game_state.cards_in_play.len() - i - 1) as f32 * Z_INDEX_STEP;
            let translation = Vec3::new(pile_x, 0.0, base_z_play - z_offset);
            t.set_if_neq(Transform { translation, rotation: Quat::IDENTITY, scale: Vec3::splat(pile_scale) });
        }
    }

    // Discard pile rests in the "burn pit" beside the play pile, so a burn
    // sweeps the cards off the stack into a visible corner instead of hiding
    // them under the next card.
    let pit = layout.burn_pit();
    for (i, &card_entity) in game_state.discard_pile.iter().enumerate() {
        if let Ok(mut t) = transform_query.get_mut(card_entity) {
            let translation = Vec3::new(pit.x, pit.y, 450.0 - i as f32 * Z_INDEX_STEP);
            t.set_if_neq(Transform { translation, rotation: Quat::IDENTITY, scale: Vec3::splat(pile_scale) });
        }
    }
}

/// The "whose turn" poker chip is disabled for now: the seat avatars + the
/// active-seat gold ring already signal whose turn it is, and the chip overlapped
/// cards in some layouts. The chip entity is still spawned (hidden) so this can
/// be reverted by restoring the positioning logic from git history.
fn update_turn_chip(mut chip_q: Query<&mut Visibility, With<TurnChip>>) {
    if let Ok(mut vis) = chip_q.get_single_mut() {
        // Spawned hidden; only write if something flipped it (it never does).
        // Guarding avoids dirtying the chip + its mesh children every frame.
        if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    }
}

// System to update card animations. Progress advances linearly with time, but
// is mapped through the per-animation `curve` before the position lerp so cards
// land with a snappy ease-out (and a slight overshoot, for `EaseOutBack`).
// Reduced motion snaps straight to the target.
fn update_card_animations(
    mut commands: Commands,
    time: Res<Time>,
    reduced: Res<ReducedMotion>,
    mut query: Query<(Entity, &mut Transform, &mut CardAnimation)>,
) {
    for (entity, mut transform, mut animation) in query.iter_mut() {
        if reduced.0 {
            transform.translation = animation.target_position;
            commands.entity(entity).remove::<CardAnimation>();
            continue;
        }
        animation.progress = (animation.progress + animation.speed * time.delta_seconds()).min(1.0);
        let eased = animation.curve.apply(animation.progress);
        transform.translation = animation.start_position.lerp(
            animation.target_position,
            eased,
        );
        if animation.progress >= 1.0 {
            commands.entity(entity).remove::<CardAnimation>();
        }
    }
}

/// Target vertical lift (design-space px) for a hovered or staged hand card.
pub(crate) const CARD_LIFT_RAISE: f32 = 22.0;

/// Drives each card's `CardLift` spring toward its hover/stage target so the
/// raise eases in with a slight overshoot rather than snapping. A card is
/// "raised" when it's the hovered card or part of the staged selection. Reduced
/// motion snaps straight to the target.
fn animate_card_lift(
    time: Res<Time>,
    hovered: Res<HoveredCard>,
    game_state: Res<GameState>,
    reduced: Res<ReducedMotion>,
    mut q: Query<(Entity, &mut CardLift)>,
) {
    let dt = time.delta_seconds().min(1.0 / 30.0); // clamp for spring stability on hitches
    // Underdamped spring (damping < critical) for a small settle-overshoot.
    const K: f32 = 520.0; // stiffness
    const D: f32 = 28.0; // damping
    const REST: f32 = 0.01; // px / px·s⁻¹ below which we treat the spring as settled
    for (entity, mut lift) in q.iter_mut() {
        let raised = hovered.0 == Some(entity) || game_state.selected_cards.contains(&entity);
        let target = if raised { CARD_LIFT_RAISE } else { 0.0 };
        if reduced.0 {
            if lift.pos != target || lift.vel != 0.0 {
                lift.pos = target;
                lift.vel = 0.0;
            }
            continue;
        }
        // Settled at rest with nothing to chase → skip the write entirely, so the
        // ~52 idle cards stay out of change detection (and out of layout_cards'
        // dirtying). Only the 1–3 cards actually lifting integrate each frame.
        if !raised && lift.pos.abs() < REST && lift.vel.abs() < REST {
            if lift.pos != 0.0 || lift.vel != 0.0 {
                lift.pos = 0.0;
                lift.vel = 0.0;
            }
            continue;
        }
        let accel = (target - lift.pos) * K - lift.vel * D;
        lift.vel += accel * dt;
        lift.pos += lift.vel * dt;
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

/// Transient pop animation on a `SeatRing`: when a seat becomes active its ring
/// scales in from large and settles with a slight overshoot. Removed when done.
#[derive(Component)]
pub struct RingPop {
    age: f32,
}

const RING_POP_TTL: f32 = 0.34;

/// On a turn change, kick the newly-active seat's ring into a scale-pop. The
/// human (seat 0) has no avatar/ring, so only AI seats pop — fine, the human's
/// turn is signalled by the interactive hand. Honors reduced motion.
fn pop_active_ring(
    mut commands: Commands,
    game_state: Res<GameState>,
    reduced: Res<ReducedMotion>,
    ring_q: Query<(Entity, &SeatRing)>,
    mut last_active: Local<usize>,
) {
    let active = game_state.current_player;
    if active == *last_active {
        return;
    }
    *last_active = active;
    let playing = game_state.phase == GamePhase::Playing
        && !game_state.finish_order.contains(&active);
    if !playing || reduced.0 {
        return;
    }
    for (entity, ring) in ring_q.iter() {
        if ring.0 == active {
            commands.entity(entity).insert(RingPop { age: 0.0 });
        }
    }
}

/// Drives the `RingPop` scale from ~1.5 down to 1.0 with an ease-out-back
/// overshoot, then removes itself.
fn update_ring_pop(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Transform, &mut RingPop)>,
) {
    for (entity, mut transform, mut pop) in q.iter_mut() {
        pop.age += time.delta_seconds();
        let frac = (pop.age / RING_POP_TTL).clamp(0.0, 1.0);
        let eased = AnimCurve::EaseOutBack.apply(frac);
        // 1.5 → 1.0; EaseOutBack dips past 1.0 then settles (the overshoot).
        transform.scale = Vec3::splat(1.5 + (1.0 - 1.5) * eased);
        if pop.age >= RING_POP_TTL {
            transform.scale = Vec3::ONE;
            commands.entity(entity).remove::<RingPop>();
        }
    }
}

/// Computes the design-space anchor for seat `seat`'s avatar cluster — sat above
/// the seat's card block, scaled with the seat.
fn avatar_anchor(seat: usize, layout: &Layout) -> Vec3 {
    let (table_x, face_y, _is_bottom) = seat_anchor(seat, layout.orientation);
    let scale = avatar_scale(seat, layout.orientation);
    Vec3::new(table_x, face_y + 100.0 * scale, 650.0)
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
            let scale = avatar_scale(seat, layout.orientation);
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
                            TextStyle { font: pixel_font.clone(), font_size: 22.0, color: Color::WHITE },
                        ),
                        transform: Transform::from_xyz(0.0, 0.0, 0.1),
                        ..default()
                    });
                    // Name + mood tag, stacked below the disc.
                    av.spawn(Text2dBundle {
                        text: Text::from_section(
                            player.name.clone(),
                            TextStyle { font: ui_font.clone(), font_size: 23.0, color: Color::WHITE },
                        ),
                        transform: Transform::from_xyz(0.0, -AVATAR_RADIUS - 18.0, 0.1),
                        ..default()
                    });
                    av.spawn(Text2dBundle {
                        text: Text::from_section(
                            mood,
                            TextStyle { font: pixel_font.clone(), font_size: 16.0, color: theme::GOLD },
                        ),
                        transform: Transform::from_xyz(0.0, -AVATAR_RADIUS - 38.0, 0.1),
                        ..default()
                    });
                });
        }
        return; // positions/ring handled next frame once entities exist
    }

    // Avatar anchors only move on an orientation flip — reposition only then,
    // so we don't dirty three avatar transforms (each with mesh children) every
    // frame.
    if layout.is_changed() {
        for (avatar, mut transform) in avatar_q.iter_mut() {
            let scale = avatar_scale(avatar.0, layout.orientation);
            transform.translation = avatar_anchor(avatar.0, &layout);
            transform.scale = Vec3::splat(scale);
        }
    }

    // Light the gold ring on the active seat only (write only on change).
    let active_seat = game_state.current_player;
    let playing = game_state.phase == GamePhase::Playing
        && !game_state.finish_order.contains(&active_seat);
    for (ring, mut vis) in ring_q.iter_mut() {
        let new_vis = if playing && ring.0 == active_seat {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *vis != new_vis {
            *vis = new_vis;
        }
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
    layout: Res<Layout>,
    mut badge_q: Query<(&mut Visibility, &mut Transform), With<PileCountBadge>>,
    mut text_q: Query<&mut Text, With<PileCountText>>,
) {
    let count = game_state.cards_in_play.len();
    let pile_scale = layout.pile_scale();
    if let Ok((mut vis, mut tf)) = badge_q.get_single_mut() {
        let new_vis = if count > 0 { Visibility::Visible } else { Visibility::Hidden };
        if *vis != new_vis {
            *vis = new_vis;
        }
        // Badge position is orientation-only — write it only on a layout flip.
        if layout.is_changed() {
            tf.translation.x = layout.play_pile_x() + (CARD_WIDTH / 2.0 + 16.0) * pile_scale;
            tf.translation.y = (CARD_HEIGHT / 2.0 - 4.0) * pile_scale;
            tf.scale = Vec3::splat(pile_scale);
        }
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

/// A short-lived sprite over the pile that scales up and fades out. Shared by
/// the burn flash and the special-resolve pulse — only the colour/size/grow
/// differ at spawn.
#[derive(Component)]
pub struct PilePulse {
    age: f32,
    ttl: f32,
    start_alpha: f32,
    /// Extra scale (on top of the pile's base scale) reached at end of life.
    grow: f32,
}

/// Spawns a `PilePulse` centred on the play pile. `size` is the un-scaled sprite
/// size; the pulse scales with the pile and grows toward `grow` while fading.
fn spawn_pile_pulse(
    commands: &mut Commands,
    layout: &Layout,
    color: Color,
    size: Vec2,
    start_alpha: f32,
    ttl: f32,
    grow: f32,
) {
    commands.spawn((
        PilePulse { age: 0.0, ttl, start_alpha, grow },
        SpriteBundle {
            sprite: Sprite {
                color: color.with_alpha(start_alpha),
                custom_size: Some(size),
                ..default()
            },
            transform: Transform::from_xyz(layout.play_pile_x(), 0.0, 615.0)
                .with_scale(Vec3::splat(layout.pile_scale())),
            ..default()
        },
    ));
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

fn update_pile_pulse(
    mut commands: Commands,
    time: Res<Time>,
    layout: Res<Layout>,
    mut q: Query<(Entity, &mut Sprite, &mut Transform, &mut PilePulse)>,
) {
    let base = layout.pile_scale();
    for (entity, mut sprite, mut transform, mut pulse) in q.iter_mut() {
        pulse.age += time.delta_seconds();
        let frac = (pulse.age / pulse.ttl).clamp(0.0, 1.0);
        let ease = 1.0 - (1.0 - frac).powi(2); // ease-out
        sprite.color = sprite.color.with_alpha(pulse.start_alpha * (1.0 - frac));
        transform.scale = Vec3::splat(base * (1.0 + pulse.grow * ease));
        if pulse.age >= pulse.ttl {
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// Watches round state for presentational events without touching gameplay:
/// a new finisher (lime "+N!" pop at the pile), the discard pile growing (a
/// burn — amber flash) and a special card landing on the pile (a coloured
/// resolve pulse). Tracked via `Local`s so each fires only on the frame its
/// count changes.
#[allow(clippy::too_many_arguments)]
fn detect_juice_events(
    mut commands: Commands,
    game_state: Res<GameState>,
    layout: Res<Layout>,
    reduced: Res<ReducedMotion>,
    cards: Query<&Card>,
    asset_server: Res<AssetServer>,
    mut last_finished: Local<usize>,
    mut last_discard: Local<usize>,
    mut last_in_play: Local<usize>,
) {
    let finished = game_state.finish_order.len();
    let discard = game_state.discard_pile.len();
    let in_play = game_state.cards_in_play.len();

    // Counts reset between plays/rounds — resync without firing.
    if finished < *last_finished { *last_finished = finished; }
    if discard < *last_discard { *last_discard = discard; }
    if in_play < *last_in_play { *last_in_play = in_play; }

    if !reduced.0 {
        let pile_x = layout.play_pile_x();

        // One pop per newly-eliminated seat, rising from the pile (handoff).
        if finished > *last_finished {
            let total = game_state.players.len();
            let pixel_font = asset_server.load("fonts/Silkscreen-Regular.ttf");
            for pos in *last_finished..finished {
                let pts = MatchState::score_for_position(pos, total);
                let label = if pts > 0 { format!("+{}!", pts) } else { "SHED!".to_string() };
                // Small horizontal jitter so stacked finishes don't overlap.
                let jitter = (pos as f32 - 1.5) * 26.0;
                commands.spawn((
                    FloatingText { age: 0.0, ttl: 0.75 },
                    Text2dBundle {
                        text: Text::from_section(
                            label,
                            TextStyle { font: pixel_font.clone(), font_size: 26.0, color: theme::LIME },
                        ),
                        transform: Transform::from_xyz(pile_x + jitter, CARD_HEIGHT * 0.5 + 30.0, 660.0)
                            .with_rotation(Quat::from_rotation_z((-8.0_f32).to_radians())),
                        ..default()
                    },
                ));
            }
        }

        // Pile burned (cards moved to discard) — amber flash that scales up.
        if discard > *last_discard {
            spawn_pile_pulse(
                &mut commands,
                &layout,
                Color::srgb(1.0, 0.85, 0.4),
                Vec2::new(CARD_WIDTH + 30.0, CARD_HEIGHT + 30.0),
                0.75,
                0.35,
                0.45,
            );
        }

        // A special card just landed on top of a grown pile — pulse a ring in
        // its colour as it resolves. (Burns clear the pile, so that path falls
        // through to the flash above instead.)
        if in_play > *last_in_play {
            if let Some(&top) = game_state.cards_in_play.last() {
                if let Ok(card) = cards.get(top) {
                    if let Some(color) = theme::special_color(card.rank) {
                        spawn_pile_pulse(
                            &mut commands,
                            &layout,
                            color,
                            Vec2::new(CARD_WIDTH + 18.0, CARD_HEIGHT + 18.0),
                            0.6,
                            0.3,
                            0.5,
                        );
                    }
                }
            }
        }
    }

    *last_finished = finished;
    *last_discard = discard;
    *last_in_play = in_play;
}

/// Ctrl+R flips the reduced-motion setting at runtime (a stand-in until the
/// settings/title screen lands). Ctrl-guarded like the Ctrl+M music toggle so a
/// bare R never collides with future bindings. SFX are unaffected — this only
/// gates motion juice.
fn toggle_reduced_motion(
    keys: Res<ButtonInput<KeyCode>>,
    mut reduced: ResMut<ReducedMotion>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl && keys.just_pressed(KeyCode::KeyR) {
        reduced.0 = !reduced.0;
        info!("Reduced motion {}", if reduced.0 { "ON" } else { "OFF" });
    }
}

// Pulse the pickup highlight when the human player needs to pick up cards
fn update_pickup_highlight(
    game_state: Res<GameState>,
    layout: Res<Layout>,
    time: Res<Time>,
    mut query: Query<(&mut Visibility, &mut Sprite, &mut Transform), With<PickupHighlight>>,
) {
    let show = game_state.needs_to_pickup
        && game_state.current_player == 0
        && game_state.phase == GamePhase::Playing;

    for (mut vis, mut sprite, mut tf) in &mut query {
        // Position is orientation-only; the alpha pulse below is the genuine
        // per-frame animation (and only while the prompt is shown).
        if layout.is_changed() {
            tf.translation.x = layout.play_pile_x();
            tf.scale = Vec3::splat(layout.pile_scale());
        }
        if show {
            if *vis != Visibility::Visible {
                *vis = Visibility::Visible;
            }
            let alpha = 0.4 + 0.35 * (time.elapsed_seconds() * 4.0).sin();
            sprite.color = Color::srgba(1.0, 0.75, 0.0, alpha);
        } else if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    }
}